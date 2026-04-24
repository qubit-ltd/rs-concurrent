/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use parking_lot::{Mutex, RwLock};

use crate::lock::{Monitor, MonitorGuard, WaitTimeoutStatus};

use super::super::{RejectedExecution, ShutdownReport};
use super::cas::{
    InflightSubmitCounter, LifecycleStateMachine, SubmissionAdmission, SubmitEnterOutcome,
};
use super::pool_job::PoolJob;
use super::thread_pool_build_error::ThreadPoolBuildError;
use super::thread_pool_config::ThreadPoolConfig;
use super::thread_pool_lifecycle::ThreadPoolLifecycle;
use super::thread_pool_state::ThreadPoolState;
use super::thread_pool_stats::ThreadPoolStats;

/// Queue owned by one worker and used for local dispatch plus stealing.
struct WorkerQueue {
    /// Logical worker index used as a stable identity key.
    worker_index: usize,
    /// Cross-thread inbox used by submitters to route work to this worker.
    inbox: Injector<PoolJob>,
    /// Stealer half of the worker-owned local deque.
    stealer: Stealer<PoolJob>,
    /// Whether this queue belongs to a worker that has reached run-loop start.
    ///
    /// Submit and steal paths ignore inactive queues so work is not routed to
    /// workers that failed to start.
    active: AtomicBool,
}

impl WorkerQueue {
    /// Creates an empty shared queue handle for one worker.
    ///
    /// # Parameters
    ///
    /// * `worker_index` - Stable index of the worker owning this queue.
    /// * `stealer` - Read-only stealing handle for the owner-local deque.
    ///
    /// # Returns
    ///
    /// A shared queue handle with an empty cross-thread inbox.
    fn new(worker_index: usize, stealer: Stealer<PoolJob>) -> Self {
        Self {
            worker_index,
            inbox: Injector::new(),
            stealer,
            active: AtomicBool::new(false),
        }
    }

    /// Returns the owning worker index.
    ///
    /// # Returns
    ///
    /// The worker index associated with this queue.
    #[inline]
    fn worker_index(&self) -> usize {
        self.worker_index
    }

    /// Returns whether this queue is currently active.
    ///
    /// # Returns
    ///
    /// `true` when the owning worker has started its run loop.
    #[inline]
    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Marks this queue as active after worker run-loop start.
    ///
    /// # Returns
    ///
    /// `true` when this call performed the state transition.
    fn activate(&self) -> bool {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Marks this queue as inactive when the worker exits.
    ///
    /// # Returns
    ///
    /// `true` when this call performed the state transition.
    fn deactivate(&self) -> bool {
        self.active
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Appends a job to the worker's cross-thread inbox.
    ///
    /// # Parameters
    ///
    /// * `job` - Job to enqueue.
    fn push_back(&self, job: PoolJob) {
        self.inbox.push(job);
    }

    /// Pops one job from this worker's cross-thread inbox into its local deque.
    ///
    /// # Returns
    ///
    /// `Some(job)` when the inbox or the destination local deque provides a
    /// job, otherwise `None`.
    ///
    /// # Implementation notes
    ///
    /// Submitters cannot push directly into the owner-only [`Worker`] deque, so
    /// the worker drains its shared inbox in small batches. The first job is
    /// returned immediately and the rest remain in the owner-local deque for
    /// cheaper subsequent pops.
    fn pop_inbox_into(&self, local: &Worker<PoolJob>) -> Option<PoolJob> {
        steal_batch_and_pop(&self.inbox, local)
    }

    /// Steals one job from this worker's local deque or inbox into `dest`.
    ///
    /// # Returns
    ///
    /// `Some(job)` when the victim's local deque or inbox provides a job,
    /// otherwise `None`.
    ///
    /// # Implementation notes
    ///
    /// A victim may have jobs already batched into its owner-local deque, or it
    /// may still have externally submitted jobs in its inbox. Thieves probe both
    /// sources and batch stolen work into their own local deque.
    fn steal_into(&self, dest: &Worker<PoolJob>) -> Option<PoolJob> {
        steal_batch_and_pop(&self.stealer, dest).or_else(|| steal_batch_and_pop(&self.inbox, dest))
    }

    /// Drains all queued jobs from this queue.
    ///
    /// # Returns
    ///
    /// A vector containing all queued jobs in FIFO order.
    fn drain(&self) -> Vec<PoolJob> {
        let mut jobs = Vec::new();
        while let Some(job) = steal_one(&self.stealer) {
            jobs.push(job);
        }
        while let Some(job) = steal_one(&self.inbox) {
            jobs.push(job);
        }
        jobs
    }
}

/// Worker-owned queue runtime.
///
/// The shared [`WorkerQueue`] can be seen by submitters, shutdown, and thieves,
/// but only the owning worker thread may touch `local`. This mirrors the
/// `Worker/Stealer` ownership split used by `crossbeam-deque` and avoids using
/// a shared injector for the worker's hottest repeated pops.
struct WorkerRuntime {
    /// Shared metadata and externally visible inbox for this worker.
    queue: Arc<WorkerQueue>,
    /// Owner-only deque used by the worker for batched and stolen jobs.
    local: Worker<PoolJob>,
}

impl WorkerRuntime {
    /// Creates a worker runtime and its shared queue handle.
    ///
    /// # Parameters
    ///
    /// * `worker_index` - Stable index of the worker owning this runtime.
    ///
    /// # Returns
    ///
    /// A runtime whose shared queue handle can be registered for submitters and
    /// thieves while its local deque remains owner-only.
    fn new(worker_index: usize) -> Self {
        let local = Worker::new_fifo();
        let queue = Arc::new(WorkerQueue::new(worker_index, local.stealer()));
        Self { queue, local }
    }

    /// Returns the owning worker index.
    #[inline]
    fn worker_index(&self) -> usize {
        self.queue.worker_index()
    }
}

/// Steals one job with immediate retry on transient contention.
///
/// # Parameters
///
/// * `source` - Queue source to probe.
///
/// # Returns
///
/// `Some(job)` when the source contains a job, otherwise `None`.
fn steal_one<S>(source: &S) -> Option<PoolJob>
where
    S: QueueStealSource,
{
    loop {
        match source.steal_one() {
            Steal::Success(job) => return Some(job),
            Steal::Empty => return None,
            // Another thread raced us while mutating queue internals. Retry
            // immediately so callers observe a stable empty/success result.
            Steal::Retry => continue,
        }
    }
}

/// Steals a batch into `dest` and returns one job.
///
/// # Parameters
///
/// * `source` - Queue source that may provide one or more jobs.
/// * `dest` - Owner-local deque receiving any stolen batch remainder.
///
/// # Returns
///
/// `Some(job)` when the source or destination yields a job, otherwise `None`.
fn steal_batch_and_pop<S>(source: &S, dest: &Worker<PoolJob>) -> Option<PoolJob>
where
    S: QueueStealSource,
{
    loop {
        match source.steal_batch_and_pop(dest) {
            Steal::Success(job) => return Some(job),
            Steal::Empty => return None,
            Steal::Retry => continue,
        }
    }
}

/// Small adapter trait over crossbeam steal sources used by this module.
trait QueueStealSource {
    /// Steals one job from this source.
    fn steal_one(&self) -> Steal<PoolJob>;

    /// Steals a batch into `dest` and pops one job from `dest`.
    fn steal_batch_and_pop(&self, dest: &Worker<PoolJob>) -> Steal<PoolJob>;
}

impl QueueStealSource for Injector<PoolJob> {
    #[inline]
    fn steal_one(&self) -> Steal<PoolJob> {
        self.steal()
    }

    #[inline]
    fn steal_batch_and_pop(&self, dest: &Worker<PoolJob>) -> Steal<PoolJob> {
        Injector::steal_batch_and_pop(self, dest)
    }
}

impl QueueStealSource for Stealer<PoolJob> {
    #[inline]
    fn steal_one(&self) -> Steal<PoolJob> {
        self.steal()
    }

    #[inline]
    fn steal_batch_and_pop(&self, dest: &Worker<PoolJob>) -> Steal<PoolJob> {
        Stealer::steal_batch_and_pop(self, dest)
    }
}

/// Shared state for a thread pool.
pub(crate) struct ThreadPoolInner {
    /// CAS gate controlling whether submit calls may still enter admission.
    submit_admission: SubmissionAdmission,
    /// Number of submit calls currently in progress.
    ///
    /// Shutdown paths wait for this counter to reach zero before changing
    /// lifecycle state and draining queues.
    inflight_submissions: InflightSubmitCounter,
    /// CAS lifecycle state machine used for fast-path shutdown checks.
    lifecycle: LifecycleStateMachine,
    /// Mutable pool state protected by a monitor.
    state_monitor: Monitor<ThreadPoolState>,
    /// Global fallback queue for accepted submissions.
    ///
    /// This queue is used when local enqueue is not selected or fails, and as
    /// the migration target when a worker retires with leftover local jobs.
    global_queue: Mutex<VecDeque<PoolJob>>,
    /// Immutable queue capacity copied from config.
    queue_capacity: Option<usize>,
    /// Core worker target used by submit fast-path admission.
    core_pool_size_target: AtomicUsize,
    /// Number of live workers mirrored from state for submit fast path.
    live_worker_count: AtomicUsize,
    /// Number of idle workers mirrored from state for submit fast path.
    idle_worker_count: AtomicUsize,
    /// Number of accepted tasks currently queued but not yet started.
    queued_task_count: AtomicUsize,
    /// Number of tasks currently held by workers.
    running_task_count: AtomicUsize,
    /// Total number of tasks accepted since pool creation.
    submitted_task_count: AtomicUsize,
    /// Total number of tasks completed since pool creation.
    completed_task_count: AtomicUsize,
    /// Total number of queued tasks cancelled by immediate shutdown.
    cancelled_task_count: AtomicUsize,
    /// Registered worker-local queues used for local dispatch and stealing.
    ///
    /// This collection is read on the hot path (enqueue routing and
    /// work-stealing victim selection) and written only when workers are
    /// added/removed, so a read-write lock reduces unnecessary reader
    /// serialization.
    worker_queues: RwLock<Vec<Arc<WorkerQueue>>>,
    /// Number of worker queues currently marked active.
    ///
    /// Submit hot paths use this atomic fast path to avoid scanning all worker
    /// queues under a lock.
    active_worker_count: AtomicUsize,
    /// Round-robin cursor used for submit-path local queue selection.
    next_enqueue_worker: AtomicUsize,
    /// Round-robin cursor used for steal victim probing.
    next_steal_worker: AtomicUsize,
    /// Prefix used for naming newly spawned workers.
    thread_name_prefix: String,
    /// Optional stack size in bytes for newly spawned workers.
    stack_size: Option<usize>,
}

/// RAII guard for one in-flight submit operation.
struct SubmitFlightGuard<'a> {
    inner: &'a ThreadPoolInner,
}

impl Drop for SubmitFlightGuard<'_> {
    /// Marks one in-flight submit as finished and wakes shutdown waiters when
    /// the last in-flight submit exits after admission has been closed.
    fn drop(&mut self) {
        if self.inner.inflight_submissions.leave() && !self.inner.submit_admission.is_open() {
            self.inner.state_monitor.notify_all();
        }
    }
}

/// Reserved worker slot state used to spawn a thread outside the state lock.
struct WorkerStartReservation {
    /// Stable index assigned to the worker being created.
    worker_index: usize,
    /// Optional first task handed directly to the new worker.
    first_task: Option<PoolJob>,
    /// Whether reservation already counted one accepted submission.
    counted_submit: bool,
    /// Whether reservation already counted one running task.
    counted_running: bool,
}

/// Maximum interval between submit-drain predicate rechecks during shutdown.
///
/// We still use condition-variable wakeups for responsiveness, but periodic
/// timed rechecks prevent rare lost-wakeup races from stalling shutdown paths.
const INFLIGHT_SUBMIT_DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(10);
/// Maximum CAS retries for bounded-queue fast-path slot reservation.
///
/// This keeps fast-path admission opportunistic: we absorb transient CAS
/// contention with a few retries, but still fall back to the state-locked
/// slow path quickly instead of spinning indefinitely.
const FAST_QUEUE_RESERVE_MAX_ATTEMPTS: usize = 3;
/// Maximum number of worker-local queues probed by one submit call.
///
/// A small bounded probe set preserves O(1) enqueue complexity while reducing
/// false fallbacks to the global queue when the first selected slot is
/// temporarily inactive.
const LOCAL_ENQUEUE_MAX_PROBES: usize = 4;

impl ThreadPoolInner {
    /// Creates shared state for a thread pool.
    ///
    /// # Parameters
    ///
    /// * `config` - Initial immutable and mutable pool configuration.
    ///
    /// # Returns
    ///
    /// A shared-state object ready to accept worker and queue operations.
    pub(crate) fn new(config: ThreadPoolConfig) -> Self {
        let mut config = config;
        let queue_capacity = config.queue_capacity;
        let core_pool_size = config.core_pool_size;
        let thread_name_prefix = std::mem::take(&mut config.thread_name_prefix);
        let stack_size = config.stack_size;
        Self {
            submit_admission: SubmissionAdmission::new_open(),
            inflight_submissions: InflightSubmitCounter::new(),
            lifecycle: LifecycleStateMachine::new_running(),
            state_monitor: Monitor::new(ThreadPoolState::new(config)),
            global_queue: Mutex::new(VecDeque::new()),
            queue_capacity,
            core_pool_size_target: AtomicUsize::new(core_pool_size),
            live_worker_count: AtomicUsize::new(0),
            idle_worker_count: AtomicUsize::new(0),
            queued_task_count: AtomicUsize::new(0),
            running_task_count: AtomicUsize::new(0),
            submitted_task_count: AtomicUsize::new(0),
            completed_task_count: AtomicUsize::new(0),
            cancelled_task_count: AtomicUsize::new(0),
            worker_queues: RwLock::new(Vec::new()),
            active_worker_count: AtomicUsize::new(0),
            next_enqueue_worker: AtomicUsize::new(0),
            next_steal_worker: AtomicUsize::new(0),
            thread_name_prefix,
            stack_size,
        }
    }

    /// Returns the current queued-task counter.
    #[inline]
    pub(crate) fn queued_count(&self) -> usize {
        self.queued_task_count.load(Ordering::Acquire)
    }

    /// Returns the current running-task counter.
    #[inline]
    pub(crate) fn running_count(&self) -> usize {
        self.running_task_count.load(Ordering::Acquire)
    }

    /// Returns the current submitted-task counter.
    #[inline]
    fn submitted_count(&self) -> usize {
        self.submitted_task_count.load(Ordering::Acquire)
    }

    /// Returns the current completed-task counter.
    #[inline]
    fn completed_count(&self) -> usize {
        self.completed_task_count.load(Ordering::Acquire)
    }

    /// Returns the current cancelled-task counter.
    #[inline]
    fn cancelled_count(&self) -> usize {
        self.cancelled_task_count.load(Ordering::Acquire)
    }

    /// Attempts to reserve one queued-task slot without taking the state lock.
    ///
    /// # Returns
    ///
    /// `true` when one queue slot is reserved, otherwise `false` for bounded
    /// queues that are currently full.
    fn try_reserve_queue_slot_fast(&self) -> bool {
        if let Some(capacity) = self.queue_capacity {
            for _ in 0..FAST_QUEUE_RESERVE_MAX_ATTEMPTS {
                let current = self.queued_count();
                if current >= capacity {
                    return false;
                }
                if self
                    .queued_task_count
                    .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    return true;
                }
            }
            return false;
        }
        self.queued_task_count.fetch_add(1, Ordering::AcqRel);
        true
    }

    /// Tries the submit fast path for direct queue admission without state
    /// locking.
    ///
    /// # Returns
    ///
    /// `Some((wake_idle, prefer_local))` when fast-path admission succeeds;
    /// otherwise `None` and caller should run the state-locked slow path.
    fn try_submit_queue_fast_path(&self) -> Option<(bool, bool)> {
        if !self.lifecycle.load().is_running() {
            return None;
        }
        let live_workers = self.live_worker_count.load(Ordering::Acquire);
        let core_pool_size = self.core_pool_size_target.load(Ordering::Acquire);
        if live_workers < core_pool_size {
            return None;
        }
        let has_active_worker = self.has_active_worker();
        if !has_active_worker {
            return None;
        }
        if !self.try_reserve_queue_slot_fast() {
            return None;
        }
        self.submitted_task_count.fetch_add(1, Ordering::AcqRel);
        let idle_workers = self.idle_worker_count.load(Ordering::Acquire);
        let should_wake_one_idle_worker = idle_workers > 0;
        let prefer_local_enqueue = idle_workers == 0 && has_active_worker;
        Some((should_wake_one_idle_worker, prefer_local_enqueue))
    }

    /// Returns whether the queue is saturated under current queue capacity.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked state snapshot containing optional queue capacity.
    ///
    /// # Returns
    ///
    /// `true` when queue capacity exists and queued count reached capacity.
    fn is_queue_saturated_locked(&self, state: &ThreadPoolState) -> bool {
        state
            .queue_capacity
            .is_some_and(|capacity| self.queued_count() >= capacity)
    }

    /// Acquires the pool state monitor while tolerating poisoned locks.
    ///
    /// # Returns
    ///
    /// A monitor guard for the mutable pool state.
    #[inline]
    pub(crate) fn lock_state(&self) -> MonitorGuard<'_, ThreadPoolState> {
        self.state_monitor.lock()
    }

    /// Acquires the pool state and reads it while holding the monitor lock.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that reads the state.
    ///
    /// # Returns
    ///
    /// The value returned by the closure.
    #[inline]
    pub(crate) fn read_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&ThreadPoolState) -> R,
    {
        self.state_monitor.read(f)
    }

    /// Acquires the pool state and mutates it while holding the monitor lock.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that mutates the state.
    ///
    /// # Returns
    ///
    /// The value returned by the closure.
    #[inline]
    pub(crate) fn write_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut ThreadPoolState) -> R,
    {
        self.state_monitor.write(f)
    }

    /// Mirrors the CAS lifecycle state into monitor-protected pool state.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked mutable state snapshot to update.
    ///
    /// # Note
    ///
    /// Worker loops and stats read lifecycle from [`ThreadPoolState`]. Shutdown
    /// entry points update lifecycle through CAS first, then mirror it here so
    /// monitor-based consumers observe the same terminal state.
    fn sync_lifecycle_locked(&self, state: &mut ThreadPoolState) {
        state.lifecycle = self.lifecycle.load();
    }

    /// Enters the submit admission path and returns an in-flight guard.
    ///
    /// # Returns
    ///
    /// A guard that decrements the in-flight submit counter on drop.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution::Shutdown`] when submissions are no longer
    /// accepted.
    fn begin_submit(&self) -> Result<SubmitFlightGuard<'_>, RejectedExecution> {
        match self.inflight_submissions.try_enter(&self.submit_admission) {
            SubmitEnterOutcome::Entered => Ok(SubmitFlightGuard { inner: self }),
            SubmitEnterOutcome::Rejected {
                became_zero_after_rollback,
            } => {
                if became_zero_after_rollback {
                    // Shutdown paths wait for in-flight submissions to drain.
                    // If this rollback produced zero, wake waiters explicitly.
                    self.state_monitor.notify_all();
                }
                Err(RejectedExecution::Shutdown)
            }
        }
    }

    /// Pushes one job into the global fallback queue.
    ///
    /// # Parameters
    ///
    /// * `job` - Job to publish for later execution.
    #[inline]
    fn push_global_job(&self, job: PoolJob) {
        self.global_queue.lock().push_back(job);
    }

    /// Publishes one accepted queued submission and optionally wakes a waiter.
    ///
    /// # Parameters
    ///
    /// * `job` - Accepted job that must be published to either a worker-local
    ///   queue or the global queue.
    /// * `should_wake_one_idle_worker` - Whether one idle worker should be
    ///   notified after publication.
    /// * `prefer_local_enqueue` - Whether to try one worker-local queue before
    ///   falling back to the global queue.
    fn publish_queued_submission(
        &self,
        job: PoolJob,
        should_wake_one_idle_worker: bool,
        prefer_local_enqueue: bool,
    ) {
        let mut pending_job = Some(job);
        if prefer_local_enqueue && !self.try_enqueue_worker_job(&mut pending_job) {
            debug_assert!(pending_job.is_some());
        }
        if let Some(job) = pending_job.take() {
            self.push_global_job(job);
        }
        if should_wake_one_idle_worker {
            self.state_monitor.notify_one();
        }
    }

    /// Pops one job from the global fallback queue.
    ///
    /// # Returns
    ///
    /// `Some(job)` when the global queue has work, otherwise `None`.
    fn pop_global_job(&self) -> Option<PoolJob> {
        self.global_queue.lock().pop_front()
    }

    /// Drains all jobs currently visible in the global fallback queue.
    ///
    /// # Returns
    ///
    /// A vector containing every drained job.
    fn drain_global_queued_jobs(&self) -> Vec<PoolJob> {
        self.global_queue.lock().drain(..).collect()
    }

    /// Returns whether at least one worker queue is currently active.
    ///
    /// # Returns
    ///
    /// `true` when some worker has reached run-loop start and can consume
    /// queued tasks.
    fn has_active_worker(&self) -> bool {
        self.active_worker_count.load(Ordering::Acquire) > 0
    }

    /// Submits a job into the queue.
    ///
    /// # Overall logic
    ///
    /// This method follows a staged admission strategy with an optimistic
    /// lock-free fast path:
    ///
    /// 1. Acquire an in-flight submit guard so shutdown transitions can wait
    ///    until this submission finishes publishing accepted work.
    /// 2. Try atomic fast-path queue admission (no state monitor lock).
    /// 3. If fast path does not apply, fall back to the state-locked slow path
    ///    to decide whether to spawn, queue, or reject.
    /// 4. Publish accepted queued jobs into a worker-local queue or the global
    ///    fallback queue outside the state lock.
    ///
    /// For queued submissions we use a targeted wake-up strategy: wake exactly
    /// one idle worker only when idle workers exist. This avoids the
    /// `notify_all` "thundering herd" effect under high submission rates.
    ///
    /// # Parameters
    ///
    /// * `job` - Type-erased job to execute or cancel later.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the job is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution::Shutdown`] after shutdown, returns
    /// [`RejectedExecution::Saturated`] when the queue and worker capacity are
    /// full, or returns [`RejectedExecution::WorkerSpawnFailed`] if a required
    /// worker cannot be created.
    pub(crate) fn submit(self: &Arc<Self>, job: PoolJob) -> Result<(), RejectedExecution> {
        let _submit_guard = self.begin_submit()?;
        if let Some((should_wake_one_idle_worker, prefer_local_enqueue)) =
            self.try_submit_queue_fast_path()
        {
            // Fast path: all admission checks and queue-slot reservation are
            // finished with atomics, so we can publish without taking state
            // monitor lock.
            self.publish_queued_submission(job, should_wake_one_idle_worker, prefer_local_enqueue);
            return Ok(());
        }
        let mut pending_job = Some(job);
        let queued_dispatch = {
            let mut state = self.lock_state();
            // Submit admissibility is decided by the CAS gate before entering
            // this block. We re-check CAS lifecycle here without mutating
            // monitor state, so the hot submit path avoids an unnecessary
            // write to the shared state lock.
            if !self.lifecycle.load().is_running() {
                return Err(RejectedExecution::Shutdown);
            }
            if state.live_workers < state.core_pool_size {
                let first_task = pending_job.take();
                let reservation = self.reserve_worker_slot_locked(&mut state, first_task, true);
                drop(state);
                self.spawn_reserved_worker(reservation)?;
                return Ok(());
            }
            if !self.is_queue_saturated_locked(&state) {
                let has_active_worker = self.has_active_worker();
                if !has_active_worker && state.live_workers < state.maximum_pool_size {
                    // Keep a runnable worker available for the accepted task.
                    // Only started workers are allowed to consume queued jobs,
                    // so when no worker is active we must spawn immediately
                    // when there is still spare worker capacity. Once all
                    // worker slots are already reserved, new work falls back
                    // to the global queue and will be consumed after startup.
                    let first_task = pending_job.take();
                    let reservation = self.reserve_worker_slot_locked(&mut state, first_task, true);
                    drop(state);
                    self.spawn_reserved_worker(reservation)?;
                    return Ok(());
                }
                self.submitted_task_count.fetch_add(1, Ordering::AcqRel);
                self.queued_task_count.fetch_add(1, Ordering::AcqRel);
                // Only wake a waiter when at least one worker is currently
                // idle. Busy workers will eventually poll queues after the
                // current task, so a broadcast wake-up is unnecessary.
                let should_wake_one_idle_worker = state.idle_workers > 0;
                // Under sustained load and without idle workers, local queues
                // reduce global queue mutex contention for both bounded and
                // unbounded modes.
                let prefer_local_enqueue = state.idle_workers == 0 && has_active_worker;
                (should_wake_one_idle_worker, prefer_local_enqueue)
            } else if state.live_workers < state.maximum_pool_size {
                let first_task = pending_job.take();
                let reservation = self.reserve_worker_slot_locked(&mut state, first_task, true);
                drop(state);
                self.spawn_reserved_worker(reservation)?;
                return Ok(());
            } else {
                return Err(RejectedExecution::Saturated);
            }
        };

        let (should_wake_one_idle_worker, prefer_local_enqueue) = queued_dispatch;
        let queued_job = pending_job
            .take()
            .expect("queued submission must keep one pending job");
        self.publish_queued_submission(
            queued_job,
            should_wake_one_idle_worker,
            prefer_local_enqueue,
        );
        Ok(())
    }

    /// Tries to enqueue a job into one worker-local queue.
    ///
    /// # Parameters
    ///
    /// * `job` - Slot containing one pending job. This method moves the job
    ///   out of the slot on success.
    ///
    /// # Returns
    ///
    /// `true` when a worker-local queue accepted the job, otherwise `false`.
    ///
    /// # Overall logic
    ///
    /// The submit path keeps this operation O(1): start from one round-robin
    /// slot and probe a small bounded number of queues. If no active slot is
    /// found in the probe window, caller falls back to the global queue.
    fn try_enqueue_worker_job(&self, job: &mut Option<PoolJob>) -> bool {
        let queues = self.worker_queues.read();
        if queues.is_empty() {
            return false;
        }
        if job.is_none() {
            return false;
        }
        let start = self.next_enqueue_worker.fetch_add(1, Ordering::Relaxed);
        let queue_count = queues.len();
        let probe_count = queue_count.min(LOCAL_ENQUEUE_MAX_PROBES);
        for offset in 0..probe_count {
            let slot = (start + offset) % queue_count;
            if !queues[slot].is_active() {
                continue;
            }
            if let Some(job) = job.take() {
                queues[slot].push_back(job);
                return true;
            }
        }
        false
    }

    /// Starts one missing core worker.
    ///
    /// # Returns
    ///
    /// `Ok(true)` when a worker was spawned, or `Ok(false)` when the core
    /// pool size is already satisfied.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution::Shutdown`] after shutdown or
    /// [`RejectedExecution::WorkerSpawnFailed`] if the worker cannot be
    /// created.
    pub(crate) fn prestart_core_thread(self: &Arc<Self>) -> Result<bool, RejectedExecution> {
        let mut state = self.lock_state();
        if !self.lifecycle.load().is_running() {
            return Err(RejectedExecution::Shutdown);
        }
        if state.live_workers >= state.core_pool_size {
            return Ok(false);
        }
        let reservation = self.reserve_worker_slot_locked(&mut state, None, false);
        drop(state);
        self.spawn_reserved_worker(reservation)?;
        Ok(true)
    }

    /// Starts all missing core workers.
    ///
    /// # Returns
    ///
    /// The number of workers started.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution`] if shutdown is observed or a worker cannot
    /// be created.
    pub(crate) fn prestart_all_core_threads(self: &Arc<Self>) -> Result<usize, RejectedExecution> {
        let mut started = 0;
        while self.prestart_core_thread()? {
            started += 1;
        }
        Ok(started)
    }

    /// Reserves one worker slot while the caller holds the state lock.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked mutable pool state.
    /// * `first_task` - Optional first task assigned directly to this worker.
    /// * `counted_submit` - Whether this reservation already counts one
    ///   accepted submission in `submitted_task_count`.
    ///
    /// # Returns
    ///
    /// Reservation object consumed by [`Self::spawn_reserved_worker`].
    fn reserve_worker_slot_locked(
        &self,
        state: &mut ThreadPoolState,
        first_task: Option<PoolJob>,
        counted_submit: bool,
    ) -> WorkerStartReservation {
        let index = state.next_worker_index;
        state.next_worker_index += 1;
        state.live_workers += 1;
        self.live_worker_count.fetch_add(1, Ordering::AcqRel);
        let counted_running = first_task.is_some();
        if counted_running {
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
        }
        if counted_submit {
            self.submitted_task_count.fetch_add(1, Ordering::AcqRel);
        }
        WorkerStartReservation {
            worker_index: index,
            first_task,
            counted_submit,
            counted_running,
        }
    }

    /// Spawns one worker from a previously reserved slot.
    ///
    /// # Parameters
    ///
    /// * `reservation` - Slot reservation allocated under the state lock.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution::WorkerSpawnFailed`] when
    /// [`thread::Builder::spawn`] fails. In that case this method rolls back
    /// all counters and queue registration touched by the reservation.
    fn spawn_reserved_worker(
        self: &Arc<Self>,
        reservation: WorkerStartReservation,
    ) -> Result<(), RejectedExecution> {
        let worker_runtime = self.register_worker_queue(reservation.worker_index);
        let has_first_task = reservation.counted_running;
        let worker_inner = Arc::clone(self);
        let mut builder = thread::Builder::new().name(format!(
            "{}-{}",
            self.thread_name_prefix, reservation.worker_index
        ));
        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }
        let first_task = reservation.first_task;
        match builder.spawn(move || run_worker(worker_inner, worker_runtime, first_task)) {
            Ok(_) => Ok(()),
            Err(source) => {
                // Worker thread never reached run-loop start; remove the
                // registration and roll back reserved counters.
                let (requeued_jobs, was_active) =
                    self.remove_worker_queue(reservation.worker_index);
                for job in requeued_jobs {
                    self.push_global_job(job);
                }
                if was_active {
                    let previous = self.active_worker_count.fetch_sub(1, Ordering::AcqRel);
                    debug_assert!(previous > 0, "thread pool active worker counter underflow",);
                }
                let mut state = self.lock_state();
                state.live_workers = state
                    .live_workers
                    .checked_sub(1)
                    .expect("thread pool live worker counter underflow");
                let previous = self.live_worker_count.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0, "thread pool live worker counter underflow");
                if has_first_task {
                    let previous = self.running_task_count.fetch_sub(1, Ordering::AcqRel);
                    debug_assert!(previous > 0, "thread pool running task counter underflow",);
                }
                if reservation.counted_submit {
                    let previous = self.submitted_task_count.fetch_sub(1, Ordering::AcqRel);
                    debug_assert!(previous > 0, "thread pool submitted task counter underflow",);
                }
                self.notify_if_terminated(&state);
                Err(RejectedExecution::WorkerSpawnFailed {
                    source: Arc::new(source),
                })
            }
        }
    }

    /// Registers an empty worker-local queue for a newly spawned worker.
    ///
    /// # Parameters
    ///
    /// * `worker_index` - Stable index of the new worker.
    fn register_worker_queue(&self, worker_index: usize) -> WorkerRuntime {
        let runtime = WorkerRuntime::new(worker_index);
        self.worker_queues.write().push(Arc::clone(&runtime.queue));
        runtime
    }

    /// Removes one worker-local queue and returns all jobs still queued in it.
    ///
    /// # Parameters
    ///
    /// * `worker_index` - Stable index of the retiring worker.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// 1. Remaining queued jobs from the removed queue.
    /// 2. Whether the removed queue was active.
    fn remove_worker_queue(&self, worker_index: usize) -> (Vec<PoolJob>, bool) {
        let queue = {
            let mut queues = self.worker_queues.write();
            if let Some(position) = queues
                .iter()
                .position(|queue| queue.worker_index() == worker_index)
            {
                Some(queues.remove(position))
            } else {
                None
            }
        };
        queue.map_or_else(
            || (Vec::new(), false),
            |queue| {
                let was_active = queue.deactivate();
                (queue.drain(), was_active)
            },
        )
    }

    /// Attempts to take one queued job for the specified worker.
    ///
    /// # Overall logic
    ///
    /// The lookup order favors locality first, then adapts by queue mode:
    ///
    /// 1. Pop from the worker's own local queue.
    /// 2. Drain the worker's shared inbox into its local queue.
    /// 3. For unbounded mode, probe global queue before steal to reduce
    ///    steal-scan overhead under high global backlog.
    /// 4. Steal from other workers' local queues and inboxes.
    /// 5. Pop from the global fallback queue.
    ///
    /// This method mutates queue-related counters only after a job is
    /// successfully claimed.
    ///
    /// # Parameters
    ///
    /// * `_state` - Locked pool state snapshot kept by caller. This method
    ///   currently relies on atomic counters and queue primitives only.
    /// * `worker_runtime` - Queue runtime owned by the worker requesting work.
    ///
    /// # Returns
    ///
    /// `Some(job)` when any queue has work, otherwise `None`.
    fn try_take_queued_job_locked(
        &self,
        _state: &ThreadPoolState,
        worker_runtime: &WorkerRuntime,
    ) -> Option<PoolJob> {
        let worker_index = worker_runtime.worker_index();
        if let Some(job) = worker_runtime.local.pop() {
            let previous = self.queued_task_count.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "thread pool queued task counter underflow");
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
            return Some(job);
        }

        if let Some(job) = worker_runtime.queue.pop_inbox_into(&worker_runtime.local) {
            let previous = self.queued_task_count.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "thread pool queued task counter underflow");
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
            return Some(job);
        }

        if self.queue_capacity.is_none()
            && let Some(job) = self.pop_global_job()
        {
            let previous = self.queued_task_count.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "thread pool queued task counter underflow");
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
            return Some(job);
        }

        if let Some(job) = self.try_steal_job_locked(worker_index, &worker_runtime.local) {
            let previous = self.queued_task_count.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "thread pool queued task counter underflow");
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
            return Some(job);
        }

        if let Some(job) = self.pop_global_job() {
            let previous = self.queued_task_count.fetch_sub(1, Ordering::AcqRel);
            debug_assert!(previous > 0, "thread pool queued task counter underflow");
            self.running_task_count.fetch_add(1, Ordering::AcqRel);
            return Some(job);
        }

        None
    }

    /// Attempts to steal one queued job from another worker queue.
    ///
    /// # Parameters
    ///
    /// * `worker_index` - Worker that is requesting stolen work.
    /// * `local` - Owner-local deque that receives any batched stolen jobs.
    ///
    /// # Returns
    ///
    /// `Some(job)` when any other worker queue can provide one job.
    fn try_steal_job_locked(
        &self,
        worker_index: usize,
        local: &Worker<PoolJob>,
    ) -> Option<PoolJob> {
        let queues = self.worker_queues.read();
        let queue_count = queues.len();
        if queue_count <= 1 {
            return None;
        }
        // Rotate victim probing start index to avoid repeatedly hammering the
        // same queue under contention.
        let start = self.next_steal_worker.fetch_add(1, Ordering::Relaxed) % queue_count;
        for offset in 0..queue_count {
            let victim = &queues[(start + offset) % queue_count];
            if victim.worker_index() == worker_index {
                continue;
            }
            if !victim.is_active() {
                continue;
            }
            if let Some(job) = victim.steal_into(local) {
                return Some(job);
            }
        }
        None
    }

    /// Drains all jobs from all worker-local queues.
    ///
    /// # Returns
    ///
    /// A vector containing every job drained from worker-local queues.
    fn drain_all_worker_queued_jobs_locked(&self) -> Vec<PoolJob> {
        let queues = self
            .worker_queues
            .read()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut jobs = Vec::new();
        for queue in queues {
            jobs.extend(queue.drain());
        }
        jobs
    }

    /// Waits until all in-flight submit calls have left admission.
    ///
    /// # Parameters
    ///
    /// * `state` - State guard currently held by the caller.
    ///
    /// # Returns
    ///
    /// The same state guard after in-flight submit count reaches zero.
    ///
    /// # Overall logic
    ///
    /// The submit path notifies this monitor when the in-flight counter drops
    /// to zero after admission closes. To guard against rare races where that
    /// notification arrives before we actually block, this helper uses
    /// short timed waits and re-checks the atomic predicate every cycle.
    fn wait_for_inflight_submissions_to_drain_locked<'a>(
        &self,
        mut state: MonitorGuard<'a, ThreadPoolState>,
    ) -> MonitorGuard<'a, ThreadPoolState> {
        while self.inflight_submissions.load() > 0 {
            let (next_state, _) = state.wait_timeout(INFLIGHT_SUBMIT_DRAIN_POLL_INTERVAL);
            state = next_state;
        }
        state
    }

    /// Requests graceful shutdown.
    ///
    /// The pool rejects later submissions but lets queued work drain.
    pub(crate) fn shutdown(&self) {
        self.submit_admission.close();
        self.lifecycle.transition_running_to_shutdown();
        let mut state = self.lock_state();
        state = self.wait_for_inflight_submissions_to_drain_locked(state);
        self.sync_lifecycle_locked(&mut state);
        self.state_monitor.notify_all();
        self.notify_if_terminated(&state);
    }

    /// Requests abrupt shutdown and cancels queued jobs.
    ///
    /// # Returns
    ///
    /// A report containing queued jobs cancelled and jobs running at the time
    /// of the request.
    pub(crate) fn shutdown_now(&self) -> ShutdownReport {
        self.submit_admission.close();
        self.lifecycle.transition_to_stopping();
        let (jobs, report) = {
            let mut state = self.lock_state();
            state = self.wait_for_inflight_submissions_to_drain_locked(state);
            self.sync_lifecycle_locked(&mut state);
            let queued = self.queued_task_count.swap(0, Ordering::AcqRel);
            let running = self.running_count();
            let mut jobs = self.drain_global_queued_jobs();
            jobs.extend(self.drain_all_worker_queued_jobs_locked());
            debug_assert_eq!(jobs.len(), queued);
            self.cancelled_task_count
                .fetch_add(queued, Ordering::AcqRel);
            self.state_monitor.notify_all();
            self.notify_if_terminated(&state);
            (jobs, ShutdownReport::new(queued, running, queued))
        };
        for job in jobs {
            job.cancel();
        }
        report
    }

    /// Returns whether shutdown has been requested.
    ///
    /// # Returns
    ///
    /// `true` if the pool is no longer in the running lifecycle state.
    pub(crate) fn is_shutdown(&self) -> bool {
        let lifecycle = self.lifecycle.load();
        lifecycle.is_shutdown() || matches!(lifecycle, ThreadPoolLifecycle::Stopping)
    }

    /// Returns whether the pool is fully terminated.
    ///
    /// # Returns
    ///
    /// `true` if shutdown has started and no queued, running, or live worker
    /// state remains.
    pub(crate) fn is_terminated(&self) -> bool {
        self.read_state(|state| self.is_terminated_locked(state))
    }

    /// Blocks the current thread until this pool is terminated.
    ///
    /// This method waits on a condition variable and therefore blocks the
    /// calling thread.
    pub(crate) fn wait_for_termination(&self) {
        self.state_monitor
            .wait_until(|state| self.is_terminated_locked(state), |_| ());
    }

    /// Returns a point-in-time pool snapshot.
    ///
    /// # Returns
    ///
    /// A snapshot built while holding the pool state lock.
    pub(crate) fn stats(&self) -> ThreadPoolStats {
        let queued_tasks = self.queued_count();
        let running_tasks = self.running_count();
        let submitted_tasks = self.submitted_count();
        let completed_tasks = self.completed_count();
        let cancelled_tasks = self.cancelled_count();
        self.read_state(|state| {
            ThreadPoolStats::new(
                state,
                queued_tasks,
                running_tasks,
                submitted_tasks,
                completed_tasks,
                cancelled_tasks,
            )
        })
    }

    /// Updates the core pool size.
    ///
    /// # Parameters
    ///
    /// * `core_pool_size` - New core pool size.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the value is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`ThreadPoolBuildError::CorePoolSizeExceedsMaximum`] when the
    /// new core size is greater than the current maximum size.
    pub(crate) fn set_core_pool_size(
        self: &Arc<Self>,
        core_pool_size: usize,
    ) -> Result<(), ThreadPoolBuildError> {
        let err = self.write_state(|state| {
            if core_pool_size > state.maximum_pool_size {
                Some(state.maximum_pool_size)
            } else {
                state.core_pool_size = core_pool_size;
                None
            }
        });
        if let Some(maximum_pool_size) = err {
            return Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum {
                core_pool_size,
                maximum_pool_size,
            });
        }
        self.core_pool_size_target
            .store(core_pool_size, Ordering::Release);
        self.state_monitor.notify_all();
        Ok(())
    }

    /// Updates the maximum pool size.
    ///
    /// # Parameters
    ///
    /// * `maximum_pool_size` - New maximum pool size.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the value is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`ThreadPoolBuildError::ZeroMaximumPoolSize`] for zero, or
    /// [`ThreadPoolBuildError::CorePoolSizeExceedsMaximum`] when the current
    /// core size is greater than the new maximum size.
    pub(crate) fn set_maximum_pool_size(
        self: &Arc<Self>,
        maximum_pool_size: usize,
    ) -> Result<(), ThreadPoolBuildError> {
        if maximum_pool_size == 0 {
            return Err(ThreadPoolBuildError::ZeroMaximumPoolSize);
        }
        let exceeds = self.write_state(|state| {
            if state.core_pool_size > maximum_pool_size {
                Some(state.core_pool_size)
            } else {
                state.maximum_pool_size = maximum_pool_size;
                None
            }
        });
        if let Some(core_pool_size) = exceeds {
            return Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum {
                core_pool_size,
                maximum_pool_size,
            });
        }
        self.state_monitor.notify_all();
        Ok(())
    }

    /// Updates the worker keep-alive timeout.
    ///
    /// # Parameters
    ///
    /// * `keep_alive` - New idle timeout.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the timeout is accepted.
    ///
    /// # Errors
    ///
    /// Returns [`ThreadPoolBuildError::ZeroKeepAlive`] when the duration is
    /// zero.
    pub(crate) fn set_keep_alive(&self, keep_alive: Duration) -> Result<(), ThreadPoolBuildError> {
        if keep_alive.is_zero() {
            return Err(ThreadPoolBuildError::ZeroKeepAlive);
        }
        self.write_state(|state| state.keep_alive = keep_alive);
        self.state_monitor.notify_all();
        Ok(())
    }

    /// Updates whether idle core workers may time out.
    ///
    /// # Parameters
    ///
    /// * `allow` - Whether idle core workers may retire after keep-alive.
    pub(crate) fn allow_core_thread_timeout(&self, allow: bool) {
        self.write_state(|state| state.allow_core_thread_timeout = allow);
        self.state_monitor.notify_all();
    }

    /// Notifies termination waiters when the state is terminal.
    ///
    /// # Parameters
    ///
    /// * `state` - Current pool state observed while holding the state lock.
    fn notify_if_terminated(&self, state: &ThreadPoolState) {
        if self.is_terminated_locked(state) {
            self.state_monitor.notify_all();
        }
    }

    /// Returns whether this pool is terminated for one locked state snapshot.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked pool state containing lifecycle and live-worker data.
    ///
    /// # Returns
    ///
    /// `true` when lifecycle is non-running, queue and running counters are
    /// zero, and no live workers remain.
    fn is_terminated_locked(&self, state: &ThreadPoolState) -> bool {
        !state.lifecycle.is_running()
            && self.queued_count() == 0
            && self.running_count() == 0
            && state.live_workers == 0
    }
}

/// Runs a single worker loop until the pool asks it to exit.
///
/// # Parameters
///
/// * `inner` - Shared pool state used for queue access and counters.
/// * `worker_runtime` - Queue runtime owned by this worker.
/// * `first_task` - Optional job assigned directly when the worker is spawned.
fn run_worker(
    inner: Arc<ThreadPoolInner>,
    worker_runtime: WorkerRuntime,
    first_task: Option<PoolJob>,
) {
    mark_worker_started(&inner, &worker_runtime.queue);
    if let Some(job) = first_task {
        job.run();
        finish_running_job(&inner);
    }
    loop {
        let job = wait_for_job(&inner, &worker_runtime);
        match job {
            Some(job) => {
                job.run();
                finish_running_job(&inner);
            }
            None => return,
        }
    }
}

/// Marks a spawned worker as fully started.
///
/// # Parameters
///
/// * `inner` - Shared pool state whose active-worker atomic counter is
///   updated.
/// * `worker_queue` - Queue owned by this worker.
fn mark_worker_started(inner: &ThreadPoolInner, worker_queue: &WorkerQueue) {
    if worker_queue.activate() {
        inner.active_worker_count.fetch_add(1, Ordering::AcqRel);
    }
}

/// Waits until a worker can take a job or should exit.
///
/// # Parameters
///
/// * `inner` - Shared pool state and monitor wait queue.
/// * `worker_runtime` - Queue runtime owned by the worker requesting a job.
///
/// # Returns
///
/// `Some(job)` when work is available, or `None` when the worker should exit.
fn wait_for_job(inner: &ThreadPoolInner, worker_runtime: &WorkerRuntime) -> Option<PoolJob> {
    let worker_index = worker_runtime.worker_index();
    let mut state = inner.lock_state();
    loop {
        match state.lifecycle {
            ThreadPoolLifecycle::Running => {
                if let Some(job) = inner.try_take_queued_job_locked(&mut state, worker_runtime) {
                    return Some(job);
                }
                if state.live_workers > state.maximum_pool_size && state.live_workers > 0 {
                    unregister_exiting_worker(inner, &mut state, worker_index);
                    return None;
                }
                if state.worker_wait_is_timed() {
                    let keep_alive = state.keep_alive;
                    mark_worker_idle(inner, &mut state);
                    // Re-check queues after publishing idle state to close the
                    // window where submit races with idle accounting and would
                    // otherwise miss waking this worker.
                    if let Some(job) = inner.try_take_queued_job_locked(&state, worker_runtime) {
                        unmark_worker_idle(inner, &mut state);
                        return Some(job);
                    }
                    let (next_state, status) = state.wait_timeout(keep_alive);
                    state = next_state;
                    unmark_worker_idle(inner, &mut state);
                    if status == WaitTimeoutStatus::TimedOut
                        && inner.queued_count() == 0
                        && state.idle_worker_can_retire()
                    {
                        unregister_exiting_worker(inner, &mut state, worker_index);
                        return None;
                    }
                } else {
                    mark_worker_idle(inner, &mut state);
                    // Re-check queues after publishing idle state to close the
                    // window where submit races with idle accounting and would
                    // otherwise miss waking this worker.
                    if let Some(job) = inner.try_take_queued_job_locked(&state, worker_runtime) {
                        unmark_worker_idle(inner, &mut state);
                        return Some(job);
                    }
                    state = state.wait();
                    unmark_worker_idle(inner, &mut state);
                }
            }
            ThreadPoolLifecycle::Shutdown => {
                if let Some(job) = inner.try_take_queued_job_locked(&mut state, worker_runtime) {
                    return Some(job);
                }
                unregister_exiting_worker(inner, &mut state, worker_index);
                return None;
            }
            ThreadPoolLifecycle::Stopping => {
                unregister_exiting_worker(inner, &mut state, worker_index);
                return None;
            }
        }
    }
}

/// Mirrors a worker entering idle wait state into locked and lock-free views.
///
/// # Parameters
///
/// * `inner` - Shared pool state containing the fast-path idle counter.
/// * `state` - Locked state snapshot containing authoritative idle workers.
fn mark_worker_idle(inner: &ThreadPoolInner, state: &mut ThreadPoolState) {
    state.idle_workers += 1;
    inner.idle_worker_count.fetch_add(1, Ordering::AcqRel);
}

/// Mirrors a worker leaving idle wait state into locked and lock-free views.
///
/// # Parameters
///
/// * `inner` - Shared pool state containing the fast-path idle counter.
/// * `state` - Locked state snapshot containing authoritative idle workers.
fn unmark_worker_idle(inner: &ThreadPoolInner, state: &mut ThreadPoolState) {
    state.idle_workers = state
        .idle_workers
        .checked_sub(1)
        .expect("thread pool idle worker counter underflow");
    let previous = inner.idle_worker_count.fetch_sub(1, Ordering::AcqRel);
    debug_assert!(previous > 0, "thread pool idle worker counter underflow");
}

/// Marks a worker-held job as finished.
///
/// # Parameters
///
/// * `inner` - Shared pool state whose running and completed counters are
///   updated.
fn finish_running_job(inner: &ThreadPoolInner) {
    let previous = inner.running_task_count.fetch_sub(1, Ordering::AcqRel);
    debug_assert!(previous > 0, "thread pool running task counter underflow");
    inner.completed_task_count.fetch_add(1, Ordering::AcqRel);
    // Hot path fast return: during normal running lifecycle this completion
    // cannot make the pool terminated, so avoid touching the state monitor.
    if inner.lifecycle.load().is_running() {
        return;
    }
    // Even in non-running lifecycle, we only need a locked termination check
    // near the terminal boundary.
    if inner.running_count() != 0 || inner.queued_count() != 0 {
        return;
    }
    let state = inner.lock_state();
    inner.notify_if_terminated(&state);
}

/// Marks a worker as exited.
///
/// # Parameters
///
/// * `inner` - Shared pool coordination state used for termination
///   notification.
/// * `state` - Locked mutable state whose live worker count is decremented.
/// * `worker_index` - Stable index of the exiting worker.
fn unregister_exiting_worker(
    inner: &ThreadPoolInner,
    state: &mut ThreadPoolState,
    worker_index: usize,
) {
    // Migrate leftover local jobs back to the global queue before removing the
    // worker registration so queued work is not lost while this worker retires.
    let (requeued_jobs, was_active) = inner.remove_worker_queue(worker_index);
    for job in requeued_jobs {
        inner.push_global_job(job);
    }
    if was_active {
        let previous = inner.active_worker_count.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "thread pool active worker counter underflow",);
    }
    state.live_workers = state
        .live_workers
        .checked_sub(1)
        .expect("thread pool live worker counter underflow");
    let previous = inner.live_worker_count.fetch_sub(1, Ordering::AcqRel);
    debug_assert!(previous > 0, "thread pool live worker counter underflow");
    inner.notify_if_terminated(state);
}

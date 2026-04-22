/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{sync::Arc, thread, time::Duration};

use crate::lock::{Monitor, MonitorGuard, WaitTimeoutStatus};

use super::super::{RejectedExecution, ShutdownReport};
use super::pool_job::PoolJob;
use super::thread_pool_build_error::ThreadPoolBuildError;
use super::thread_pool_config::ThreadPoolConfig;
use super::thread_pool_lifecycle::ThreadPoolLifecycle;
use super::thread_pool_state::ThreadPoolState;
use super::thread_pool_stats::ThreadPoolStats;

/// Shared state for a thread pool.
pub(crate) struct ThreadPoolInner {
    /// Mutable pool state protected by a monitor.
    state_monitor: Monitor<ThreadPoolState>,
    /// Prefix used for naming newly spawned workers.
    thread_name_prefix: String,
    /// Optional stack size in bytes for newly spawned workers.
    stack_size: Option<usize>,
}

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
        let thread_name_prefix = std::mem::take(&mut config.thread_name_prefix);
        let stack_size = config.stack_size;
        Self {
            state_monitor: Monitor::new(ThreadPoolState::new(config)),
            thread_name_prefix,
            stack_size,
        }
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

    /// Submits a job into the queue.
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
        let mut state = self.lock_state();
        if !state.lifecycle.is_running() {
            return Err(RejectedExecution::Shutdown);
        }
        if state.live_workers < state.core_pool_size {
            self.spawn_worker_locked(&mut state, Some(job))?;
            state.submitted_tasks += 1;
            return Ok(());
        }
        if !state.is_saturated() {
            state.queue.push_back(job);
            state.submitted_tasks += 1;
            if state.live_workers == 0
                && let Err(error) = self.spawn_worker_locked(&mut state, None)
            {
                if let Some(job) = state.queue.pop_back() {
                    state.submitted_tasks = state
                        .submitted_tasks
                        .checked_sub(1)
                        .expect("thread pool submitted task counter underflow");
                    drop(state);
                    job.cancel();
                }
                return Err(error);
            }
            self.state_monitor.notify_all();
            return Ok(());
        }
        if state.live_workers < state.maximum_pool_size {
            self.spawn_worker_locked(&mut state, Some(job))?;
            state.submitted_tasks += 1;
            Ok(())
        } else {
            Err(RejectedExecution::Saturated)
        }
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
        if !state.lifecycle.is_running() {
            return Err(RejectedExecution::Shutdown);
        }
        if state.live_workers >= state.core_pool_size {
            return Ok(false);
        }
        self.spawn_worker_locked(&mut state, None)?;
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

    /// Spawns a worker while the caller holds the pool state lock.
    ///
    /// # Parameters
    ///
    /// * `state` - Locked mutable pool state to update while spawning.
    /// * `first_task` - Optional first job assigned directly to the new worker.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the worker thread is spawned.
    ///
    /// # Errors
    ///
    /// Returns [`RejectedExecution::WorkerSpawnFailed`] if
    /// [`thread::Builder::spawn`] fails.
    fn spawn_worker_locked(
        self: &Arc<Self>,
        state: &mut ThreadPoolState,
        first_task: Option<PoolJob>,
    ) -> Result<(), RejectedExecution> {
        let index = state.next_worker_index;
        state.next_worker_index += 1;
        state.live_workers += 1;
        if first_task.is_some() {
            state.running_tasks += 1;
        }

        let worker_inner = Arc::clone(self);
        let mut builder =
            thread::Builder::new().name(format!("{}-{index}", self.thread_name_prefix));
        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }
        match builder.spawn(move || run_worker(worker_inner, first_task)) {
            Ok(_) => Ok(()),
            Err(source) => {
                state.live_workers = state
                    .live_workers
                    .checked_sub(1)
                    .expect("thread pool live worker counter underflow");
                if state.running_tasks > 0 {
                    state.running_tasks -= 1;
                }
                self.notify_if_terminated(state);
                Err(RejectedExecution::WorkerSpawnFailed {
                    source: Arc::new(source),
                })
            }
        }
    }

    /// Requests graceful shutdown.
    ///
    /// The pool rejects later submissions but lets queued work drain.
    pub(crate) fn shutdown(&self) {
        let mut state = self.lock_state();
        if state.lifecycle.is_running() {
            state.lifecycle = ThreadPoolLifecycle::Shutdown;
        }
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
        let (jobs, report) = {
            let mut state = self.lock_state();
            if state.lifecycle.is_running() || state.lifecycle.is_shutdown() {
                state.lifecycle = ThreadPoolLifecycle::Stopping;
            }
            let queued = state.queue.len();
            let running = state.running_tasks;
            let jobs = state.queue.drain(..).collect::<Vec<_>>();
            state.cancelled_tasks += queued;
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
        self.read_state(|state| !state.lifecycle.is_running())
    }

    /// Returns whether the pool is fully terminated.
    ///
    /// # Returns
    ///
    /// `true` if shutdown has started and no queued, running, or live worker
    /// state remains.
    pub(crate) fn is_terminated(&self) -> bool {
        self.read_state(ThreadPoolState::is_terminated)
    }

    /// Blocks the current thread until this pool is terminated.
    ///
    /// This method waits on a condition variable and therefore blocks the
    /// calling thread.
    pub(crate) fn wait_for_termination(&self) {
        self.state_monitor
            .wait_until(|state| state.is_terminated(), |_| ());
    }

    /// Returns a point-in-time pool snapshot.
    ///
    /// # Returns
    ///
    /// A snapshot built while holding the pool state lock.
    pub(crate) fn stats(&self) -> ThreadPoolStats {
        self.read_state(ThreadPoolStats::new)
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
        if state.is_terminated() {
            self.state_monitor.notify_all();
        }
    }
}

/// Runs a single worker loop until the pool asks it to exit.
///
/// # Parameters
///
/// * `inner` - Shared pool state used for queue access and counters.
/// * `first_task` - Optional job assigned directly when the worker is spawned.
fn run_worker(inner: Arc<ThreadPoolInner>, first_task: Option<PoolJob>) {
    if let Some(job) = first_task {
        job.run();
        finish_running_job(&inner);
    }
    loop {
        let job = wait_for_job(&inner);
        match job {
            Some(job) => {
                job.run();
                finish_running_job(&inner);
            }
            None => return,
        }
    }
}

/// Waits until a worker can take a job or should exit.
///
/// # Parameters
///
/// * `inner` - Shared pool state and monitor wait queue.
///
/// # Returns
///
/// `Some(job)` when work is available, or `None` when the worker should exit.
fn wait_for_job(inner: &ThreadPoolInner) -> Option<PoolJob> {
    let mut state = inner.lock_state();
    loop {
        match state.lifecycle {
            ThreadPoolLifecycle::Running => {
                if let Some(job) = state.queue.pop_front() {
                    state.running_tasks += 1;
                    return Some(job);
                }
                if state.live_workers > state.maximum_pool_size && state.live_workers > 0 {
                    unregister_exiting_worker(inner, &mut state);
                    return None;
                }
                if state.worker_wait_is_timed() {
                    let keep_alive = state.keep_alive;
                    state.idle_workers += 1;
                    let (next_state, status) = state.wait_timeout(keep_alive);
                    state = next_state;
                    state.idle_workers = state
                        .idle_workers
                        .checked_sub(1)
                        .expect("thread pool idle worker counter underflow");
                    if status == WaitTimeoutStatus::TimedOut
                        && state.queue.is_empty()
                        && state.idle_worker_can_retire()
                    {
                        unregister_exiting_worker(inner, &mut state);
                        return None;
                    }
                } else {
                    state.idle_workers += 1;
                    state = state.wait();
                    state.idle_workers = state
                        .idle_workers
                        .checked_sub(1)
                        .expect("thread pool idle worker counter underflow");
                }
            }
            ThreadPoolLifecycle::Shutdown => {
                if let Some(job) = state.queue.pop_front() {
                    state.running_tasks += 1;
                    return Some(job);
                }
                unregister_exiting_worker(inner, &mut state);
                return None;
            }
            ThreadPoolLifecycle::Stopping => {
                unregister_exiting_worker(inner, &mut state);
                return None;
            }
        }
    }
}

/// Marks a worker-held job as finished.
///
/// # Parameters
///
/// * `inner` - Shared pool state whose running and completed counters are
///   updated.
fn finish_running_job(inner: &ThreadPoolInner) {
    let mut state = inner.lock_state();
    state.running_tasks = state
        .running_tasks
        .checked_sub(1)
        .expect("thread pool running task counter underflow");
    state.completed_tasks += 1;
    inner.notify_if_terminated(&state);
}

/// Marks a worker as exited.
///
/// # Parameters
///
/// * `inner` - Shared pool coordination state used for termination
///   notification.
/// * `state` - Locked mutable state whose live worker count is decremented.
fn unregister_exiting_worker(inner: &ThreadPoolInner, state: &mut ThreadPoolState) {
    state.live_workers = state
        .live_workers
        .checked_sub(1)
        .expect("thread pool live worker counter underflow");
    inner.notify_if_terminated(state);
}

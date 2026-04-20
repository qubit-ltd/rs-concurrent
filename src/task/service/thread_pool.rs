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
    future::Future,
    io,
    pin::Pin,
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
    },
    thread,
    time::Duration,
};

use qubit_function::Callable;
use thiserror::Error;

use crate::task::{
    TaskCompletion,
    TaskHandle,
    task_runner::run_callable,
};

use super::{
    ExecutorService,
    RejectedExecution,
    ShutdownReport,
};

/// Default thread name prefix used by [`ThreadPoolBuilder`].
const DEFAULT_THREAD_NAME_PREFIX: &str = "qubit-thread-pool";

/// Default idle lifetime for workers above the core pool size.
const DEFAULT_KEEP_ALIVE: Duration = Duration::from_secs(60);

/// OS thread pool implementing [`ExecutorService`].
///
/// `ThreadPool` accepts fallible tasks, stores them in an internal FIFO queue,
/// and executes them on worker threads. Workers are created lazily up to the
/// configured core size, queued after that, and may grow up to the maximum size
/// when a bounded queue is full. Submitted tasks return [`TaskHandle`], which
/// supports both blocking [`TaskHandle::get`] and async `.await` result
/// retrieval.
///
/// `shutdown` is graceful: already accepted queued tasks are allowed to run.
/// `shutdown_now` is abrupt: queued tasks that have not started are completed
/// with [`TaskExecutionError::Cancelled`](crate::task::TaskExecutionError::Cancelled).
///
/// # Author
///
/// Haixing Hu
pub struct ThreadPool {
    inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Creates a thread pool with equal core and maximum worker counts.
    ///
    /// # Parameters
    ///
    /// * `worker_count` - Core and maximum worker count for this pool.
    ///
    /// # Returns
    ///
    /// `Ok(ThreadPool)` if all workers are spawned successfully. Returns
    /// [`ThreadPoolBuildError`] if the configuration is invalid or a worker
    /// thread cannot be spawned.
    #[inline]
    pub fn new(worker_count: usize) -> Result<Self, ThreadPoolBuildError> {
        Self::builder().worker_count(worker_count).build()
    }

    /// Creates a builder for configuring a thread pool.
    ///
    /// # Returns
    ///
    /// A builder with default worker count and an unbounded queue.
    #[inline]
    pub fn builder() -> ThreadPoolBuilder {
        ThreadPoolBuilder::default()
    }

    /// Returns the number of queued tasks waiting for a worker.
    ///
    /// # Returns
    ///
    /// The number of accepted tasks that have not started yet.
    #[inline]
    pub fn queued_count(&self) -> usize {
        self.inner.lock_state().queue.len()
    }

    /// Returns the number of tasks currently held by workers.
    ///
    /// # Returns
    ///
    /// The number of tasks that workers have taken from the queue and have not
    /// yet finished processing.
    #[inline]
    pub fn running_count(&self) -> usize {
        self.inner.lock_state().running_tasks
    }

    /// Returns the number of worker threads that have not exited.
    ///
    /// # Returns
    ///
    /// The number of live worker loops still owned by this pool.
    #[inline]
    pub fn worker_count(&self) -> usize {
        self.inner.lock_state().live_workers
    }

    /// Returns the configured core pool size.
    ///
    /// # Returns
    ///
    /// The number of workers kept for normal load before tasks are queued.
    #[inline]
    pub fn core_pool_size(&self) -> usize {
        self.inner.lock_state().core_pool_size
    }

    /// Returns the configured maximum pool size.
    ///
    /// # Returns
    ///
    /// The maximum number of worker threads this pool may create.
    #[inline]
    pub fn maximum_pool_size(&self) -> usize {
        self.inner.lock_state().maximum_pool_size
    }

    /// Returns a point-in-time snapshot of pool counters.
    ///
    /// # Returns
    ///
    /// A snapshot containing worker, queue, and task counters observed under
    /// the pool state lock.
    #[inline]
    pub fn stats(&self) -> ThreadPoolStats {
        self.inner.stats()
    }

    /// Starts one core worker if the pool has fewer live workers than its
    /// configured core size.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if a worker was started, `Ok(false)` if no core worker was
    /// needed, or `Err(RejectedExecution)` if the pool is shut down or worker
    /// creation fails.
    #[inline]
    pub fn prestart_core_thread(&self) -> Result<bool, RejectedExecution> {
        self.inner.prestart_core_thread()
    }

    /// Starts all missing core workers.
    ///
    /// # Returns
    ///
    /// The number of workers started, or `Err(RejectedExecution)` if the pool
    /// is shut down or worker creation fails.
    #[inline]
    pub fn prestart_all_core_threads(&self) -> Result<usize, RejectedExecution> {
        self.inner.prestart_all_core_threads()
    }

    /// Updates the core pool size.
    ///
    /// Increasing the core size does not eagerly create new workers unless
    /// queued work is waiting. Call [`Self::prestart_all_core_threads`] when
    /// eager creation is desired. Decreasing the core size lets excess idle
    /// workers retire according to the keep-alive policy.
    ///
    /// # Parameters
    ///
    /// * `core_pool_size` - New core pool size.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the size is accepted. Returns [`ThreadPoolBuildError`] when
    /// the new core size would exceed the maximum size.
    pub fn set_core_pool_size(&self, core_pool_size: usize) -> Result<(), ThreadPoolBuildError> {
        self.inner.set_core_pool_size(core_pool_size)
    }

    /// Updates the maximum pool size.
    ///
    /// Excess workers are not interrupted. They retire after finishing current
    /// work or timing out while idle.
    ///
    /// # Parameters
    ///
    /// * `maximum_pool_size` - New maximum pool size.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the size is accepted. Returns [`ThreadPoolBuildError`] when
    /// the maximum size is zero or smaller than the core size.
    pub fn set_maximum_pool_size(
        &self,
        maximum_pool_size: usize,
    ) -> Result<(), ThreadPoolBuildError> {
        self.inner.set_maximum_pool_size(maximum_pool_size)
    }

    /// Updates how long excess idle workers may wait before exiting.
    ///
    /// # Parameters
    ///
    /// * `keep_alive` - New idle timeout for workers above the core size.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the timeout is accepted. Returns [`ThreadPoolBuildError`]
    /// when the duration is zero.
    pub fn set_keep_alive(&self, keep_alive: Duration) -> Result<(), ThreadPoolBuildError> {
        self.inner.set_keep_alive(keep_alive)
    }

    /// Updates whether core workers may also retire after keep-alive timeout.
    ///
    /// # Parameters
    ///
    /// * `allow` - Whether core workers are subject to idle timeout.
    pub fn allow_core_thread_timeout(&self, allow: bool) {
        self.inner.allow_core_thread_timeout(allow);
    }

    /// Submits an already type-erased pool job.
    ///
    /// This is crate-internal so higher-level services can attach their own
    /// lifecycle callbacks without exposing the raw queue representation as
    /// public API.
    pub(crate) fn submit_job(&self, job: PoolJob) -> Result<(), RejectedExecution> {
        self.inner.submit(job)
    }
}

impl Drop for ThreadPool {
    /// Requests graceful shutdown when the pool value is dropped.
    fn drop(&mut self) {
        self.inner.shutdown();
    }
}

impl ExecutorService for ThreadPool {
    type Handle<R, E>
        = TaskHandle<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    type Termination<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    /// Accepts a callable and queues it for pool workers.
    fn submit_callable<C, R, E>(&self, task: C) -> Result<Self::Handle<R, E>, RejectedExecution>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let (handle, completion) = TaskHandle::completion_pair();
        let completion_for_run = completion.clone();
        let job = PoolJob::new(
            Box::new(move || run_task(task, completion_for_run)),
            Box::new(move || {
                completion.cancel();
            }),
        );
        self.inner.submit(job)?;
        Ok(handle)
    }

    /// Stops accepting new tasks after already queued work is drained.
    #[inline]
    fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Stops accepting tasks and cancels queued tasks that have not started.
    #[inline]
    fn shutdown_now(&self) -> ShutdownReport {
        self.inner.shutdown_now()
    }

    /// Returns whether shutdown has been requested.
    #[inline]
    fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown()
    }

    /// Returns whether shutdown was requested and all workers have exited.
    #[inline]
    fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }

    /// Waits until the pool has terminated.
    ///
    /// This future blocks the polling thread while waiting on a condition
    /// variable.
    fn await_termination(&self) -> Self::Termination<'_> {
        Box::pin(async move {
            self.inner.wait_for_termination();
        })
    }
}

/// Builder for [`ThreadPool`].
///
/// The default builder uses the available CPU parallelism as worker count and
/// an unbounded FIFO queue.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone)]
pub struct ThreadPoolBuilder {
    core_pool_size: usize,
    maximum_pool_size: usize,
    queue_capacity: Option<usize>,
    thread_name_prefix: String,
    stack_size: Option<usize>,
    keep_alive: Duration,
    allow_core_thread_timeout: bool,
    prestart_core_threads: bool,
}

impl ThreadPoolBuilder {
    /// Sets the number of worker threads.
    ///
    /// # Parameters
    ///
    /// * `worker_count` - Core and maximum worker count for this pool.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn worker_count(mut self, worker_count: usize) -> Self {
        self.core_pool_size = worker_count;
        self.maximum_pool_size = worker_count;
        self
    }

    /// Sets the core pool size.
    ///
    /// A submitted task creates a new worker while the live worker count is
    /// below this value. Once the core size is reached, tasks are queued before
    /// the pool considers growing toward the maximum size.
    ///
    /// # Parameters
    ///
    /// * `core_pool_size` - Number of core workers.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn core_pool_size(mut self, core_pool_size: usize) -> Self {
        self.core_pool_size = core_pool_size;
        self
    }

    /// Sets the maximum pool size.
    ///
    /// The pool grows above the core size only when the queue cannot accept a
    /// submitted task.
    ///
    /// # Parameters
    ///
    /// * `maximum_pool_size` - Maximum number of live workers.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn maximum_pool_size(mut self, maximum_pool_size: usize) -> Self {
        self.maximum_pool_size = maximum_pool_size;
        self
    }

    /// Sets a bounded queue capacity.
    ///
    /// The capacity counts only tasks waiting in the queue. Tasks already held
    /// by worker threads are not included.
    ///
    /// # Parameters
    ///
    /// * `capacity` - Maximum number of queued tasks.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = Some(capacity);
        self
    }

    /// Uses an unbounded queue.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn unbounded_queue(mut self) -> Self {
        self.queue_capacity = None;
        self
    }

    /// Sets the worker thread name prefix.
    ///
    /// Worker names are created by appending the worker index to this prefix.
    ///
    /// # Parameters
    ///
    /// * `prefix` - Prefix for worker thread names.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn thread_name_prefix(mut self, prefix: &str) -> Self {
        self.thread_name_prefix = prefix.to_owned();
        self
    }

    /// Sets the worker thread stack size.
    ///
    /// # Parameters
    ///
    /// * `stack_size` - Stack size in bytes for each worker thread.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = Some(stack_size);
        self
    }

    /// Sets the idle timeout for workers above the core pool size.
    ///
    /// # Parameters
    ///
    /// * `keep_alive` - Duration an excess worker may stay idle.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    /// Allows core workers to retire after the keep-alive timeout.
    ///
    /// # Parameters
    ///
    /// * `allow` - Whether idle core workers may time out.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn allow_core_thread_timeout(mut self, allow: bool) -> Self {
        self.allow_core_thread_timeout = allow;
        self
    }

    /// Starts all core workers during [`Self::build`].
    ///
    /// Without this option, workers are created lazily as tasks are submitted,
    /// matching the default JDK `ThreadPoolExecutor` behavior.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn prestart_core_threads(mut self) -> Self {
        self.prestart_core_threads = true;
        self
    }

    /// Builds the configured thread pool.
    ///
    /// # Returns
    ///
    /// `Ok(ThreadPool)` if all workers are spawned successfully. Returns
    /// [`ThreadPoolBuildError`] if the configuration is invalid or a worker
    /// thread cannot be spawned.
    pub fn build(self) -> Result<ThreadPool, ThreadPoolBuildError> {
        self.validate()?;
        let prestart_core_threads = self.prestart_core_threads;
        let inner = Arc::new(ThreadPoolInner::new(ThreadPoolConfig {
            core_pool_size: self.core_pool_size,
            maximum_pool_size: self.maximum_pool_size,
            queue_capacity: self.queue_capacity,
            thread_name_prefix: self.thread_name_prefix,
            stack_size: self.stack_size,
            keep_alive: self.keep_alive,
            allow_core_thread_timeout: self.allow_core_thread_timeout,
        }));
        if prestart_core_threads {
            inner
                .prestart_all_core_threads()
                .map_err(ThreadPoolBuildError::from_rejected_execution)?;
        }
        Ok(ThreadPool { inner })
    }

    /// Validates this builder configuration.
    fn validate(&self) -> Result<(), ThreadPoolBuildError> {
        if self.maximum_pool_size == 0 {
            return Err(ThreadPoolBuildError::ZeroMaximumPoolSize);
        }
        if self.core_pool_size > self.maximum_pool_size {
            return Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum {
                core_pool_size: self.core_pool_size,
                maximum_pool_size: self.maximum_pool_size,
            });
        }
        if self.queue_capacity == Some(0) {
            return Err(ThreadPoolBuildError::ZeroQueueCapacity);
        }
        if self.stack_size == Some(0) {
            return Err(ThreadPoolBuildError::ZeroStackSize);
        }
        if self.keep_alive.is_zero() {
            return Err(ThreadPoolBuildError::ZeroKeepAlive);
        }
        Ok(())
    }
}

impl Default for ThreadPoolBuilder {
    /// Creates a builder with CPU parallelism defaults.
    fn default() -> Self {
        let worker_count = default_worker_count();
        Self {
            core_pool_size: worker_count,
            maximum_pool_size: worker_count,
            queue_capacity: None,
            thread_name_prefix: DEFAULT_THREAD_NAME_PREFIX.to_owned(),
            stack_size: None,
            keep_alive: DEFAULT_KEEP_ALIVE,
            allow_core_thread_timeout: false,
            prestart_core_threads: false,
        }
    }
}

/// Error returned when a [`ThreadPool`] cannot be built.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Error)]
pub enum ThreadPoolBuildError {
    /// The configured maximum pool size is zero.
    #[error("thread pool maximum pool size must be greater than zero")]
    ZeroMaximumPoolSize,

    /// The configured core pool size is greater than the maximum pool size.
    #[error(
        "thread pool core pool size {core_pool_size} exceeds maximum pool size {maximum_pool_size}"
    )]
    CorePoolSizeExceedsMaximum {
        /// Configured core pool size.
        core_pool_size: usize,

        /// Configured maximum pool size.
        maximum_pool_size: usize,
    },

    /// The configured bounded queue capacity is zero.
    #[error("thread pool queue capacity must be greater than zero")]
    ZeroQueueCapacity,

    /// The configured worker stack size is zero.
    #[error("thread pool stack size must be greater than zero")]
    ZeroStackSize,

    /// The configured keep-alive timeout is zero.
    #[error("thread pool keep-alive timeout must be greater than zero")]
    ZeroKeepAlive,

    /// A worker thread could not be spawned.
    #[error("failed to spawn thread pool worker {index}: {source}")]
    SpawnWorker {
        /// Index of the worker that failed to spawn.
        index: usize,

        /// I/O error reported by [`std::thread::Builder::spawn`].
        source: io::Error,
    },
}

impl ThreadPoolBuildError {
    /// Converts a runtime worker-spawn rejection into a build error.
    fn from_rejected_execution(error: RejectedExecution) -> Self {
        match error {
            RejectedExecution::WorkerSpawnFailed { source } => Self::SpawnWorker {
                index: 0,
                source: io::Error::new(source.kind(), source.to_string()),
            },
            RejectedExecution::Shutdown => Self::SpawnWorker {
                index: 0,
                source: io::Error::other("thread pool shut down during prestart"),
            },
            RejectedExecution::Saturated => Self::SpawnWorker {
                index: 0,
                source: io::Error::other("thread pool saturated during prestart"),
            },
        }
    }
}

/// Point-in-time counters reported by [`ThreadPool`].
///
/// The snapshot is intended for monitoring and tests. It is not a stable
/// synchronization primitive; concurrent submissions and completions may make
/// the next snapshot different immediately after this one is returned.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ThreadPoolStats {
    /// Configured core pool size.
    pub core_pool_size: usize,

    /// Configured maximum pool size.
    pub maximum_pool_size: usize,

    /// Number of live worker loops.
    pub live_workers: usize,

    /// Number of workers currently waiting for work.
    pub idle_workers: usize,

    /// Number of queued tasks waiting for a worker.
    pub queued_tasks: usize,

    /// Number of tasks currently held by workers.
    pub running_tasks: usize,

    /// Number of tasks accepted since pool creation.
    pub submitted_tasks: usize,

    /// Number of worker-held jobs finished since pool creation.
    pub completed_tasks: usize,

    /// Number of queued jobs cancelled by immediate shutdown.
    pub cancelled_tasks: usize,

    /// Whether shutdown has been requested.
    pub shutdown: bool,

    /// Whether the pool has fully terminated.
    pub terminated: bool,
}

/// Immutable and initial mutable configuration used by a thread pool.
struct ThreadPoolConfig {
    core_pool_size: usize,
    maximum_pool_size: usize,
    queue_capacity: Option<usize>,
    thread_name_prefix: String,
    stack_size: Option<usize>,
    keep_alive: Duration,
    allow_core_thread_timeout: bool,
}

/// Shared state for a thread pool.
struct ThreadPoolInner {
    state: Mutex<ThreadPoolState>,
    available: Condvar,
    terminated: Condvar,
    thread_name_prefix: String,
    stack_size: Option<usize>,
}

impl ThreadPoolInner {
    /// Creates shared state for a thread pool.
    fn new(config: ThreadPoolConfig) -> Self {
        Self {
            state: Mutex::new(ThreadPoolState {
                lifecycle: ThreadPoolLifecycle::Running,
                queue: VecDeque::new(),
                queue_capacity: config.queue_capacity,
                running_tasks: 0,
                live_workers: 0,
                idle_workers: 0,
                submitted_tasks: 0,
                completed_tasks: 0,
                cancelled_tasks: 0,
                core_pool_size: config.core_pool_size,
                maximum_pool_size: config.maximum_pool_size,
                keep_alive: config.keep_alive,
                allow_core_thread_timeout: config.allow_core_thread_timeout,
                next_worker_index: 0,
            }),
            available: Condvar::new(),
            terminated: Condvar::new(),
            thread_name_prefix: config.thread_name_prefix,
            stack_size: config.stack_size,
        }
    }

    /// Acquires the pool state while tolerating poisoned locks.
    fn lock_state(&self) -> MutexGuard<'_, ThreadPoolState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Submits a job into the queue.
    fn submit(self: &Arc<Self>, job: PoolJob) -> Result<(), RejectedExecution> {
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
            self.available.notify_one();
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
    fn prestart_core_thread(self: &Arc<Self>) -> Result<bool, RejectedExecution> {
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
    fn prestart_all_core_threads(self: &Arc<Self>) -> Result<usize, RejectedExecution> {
        let mut started = 0;
        while self.prestart_core_thread()? {
            started += 1;
        }
        Ok(started)
    }

    /// Spawns a worker while the caller holds the pool state lock.
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
    fn shutdown(&self) {
        let mut state = self.lock_state();
        if state.lifecycle.is_running() {
            state.lifecycle = ThreadPoolLifecycle::Shutdown;
        }
        self.available.notify_all();
        self.notify_if_terminated(&state);
    }

    /// Requests abrupt shutdown and cancels queued jobs.
    fn shutdown_now(&self) -> ShutdownReport {
        let (jobs, report) = {
            let mut state = self.lock_state();
            if state.lifecycle.is_running() || state.lifecycle.is_shutdown() {
                state.lifecycle = ThreadPoolLifecycle::Stopping;
            }
            let queued = state.queue.len();
            let running = state.running_tasks;
            let jobs = state.queue.drain(..).collect::<Vec<_>>();
            state.cancelled_tasks += queued;
            self.available.notify_all();
            self.notify_if_terminated(&state);
            (jobs, ShutdownReport::new(queued, running, queued))
        };
        for job in jobs {
            job.cancel();
        }
        report
    }

    /// Returns whether shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        !self.lock_state().lifecycle.is_running()
    }

    /// Returns whether the pool is fully terminated.
    fn is_terminated(&self) -> bool {
        self.lock_state().is_terminated()
    }

    /// Blocks the current thread until this pool is terminated.
    fn wait_for_termination(&self) {
        let mut state = self.lock_state();
        while !state.is_terminated() {
            state = self
                .terminated
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    /// Returns a point-in-time pool snapshot.
    fn stats(&self) -> ThreadPoolStats {
        let state = self.lock_state();
        ThreadPoolStats {
            core_pool_size: state.core_pool_size,
            maximum_pool_size: state.maximum_pool_size,
            live_workers: state.live_workers,
            idle_workers: state.idle_workers,
            queued_tasks: state.queue.len(),
            running_tasks: state.running_tasks,
            submitted_tasks: state.submitted_tasks,
            completed_tasks: state.completed_tasks,
            cancelled_tasks: state.cancelled_tasks,
            shutdown: !state.lifecycle.is_running(),
            terminated: state.is_terminated(),
        }
    }

    /// Updates the core pool size.
    fn set_core_pool_size(
        self: &Arc<Self>,
        core_pool_size: usize,
    ) -> Result<(), ThreadPoolBuildError> {
        let mut state = self.lock_state();
        if core_pool_size > state.maximum_pool_size {
            return Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum {
                core_pool_size,
                maximum_pool_size: state.maximum_pool_size,
            });
        }
        state.core_pool_size = core_pool_size;
        self.available.notify_all();
        Ok(())
    }

    /// Updates the maximum pool size.
    fn set_maximum_pool_size(
        self: &Arc<Self>,
        maximum_pool_size: usize,
    ) -> Result<(), ThreadPoolBuildError> {
        let mut state = self.lock_state();
        if maximum_pool_size == 0 {
            return Err(ThreadPoolBuildError::ZeroMaximumPoolSize);
        }
        if state.core_pool_size > maximum_pool_size {
            return Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum {
                core_pool_size: state.core_pool_size,
                maximum_pool_size,
            });
        }
        state.maximum_pool_size = maximum_pool_size;
        self.available.notify_all();
        Ok(())
    }

    /// Updates the worker keep-alive timeout.
    fn set_keep_alive(&self, keep_alive: Duration) -> Result<(), ThreadPoolBuildError> {
        if keep_alive.is_zero() {
            return Err(ThreadPoolBuildError::ZeroKeepAlive);
        }
        let mut state = self.lock_state();
        state.keep_alive = keep_alive;
        self.available.notify_all();
        Ok(())
    }

    /// Updates whether idle core workers may time out.
    fn allow_core_thread_timeout(&self, allow: bool) {
        let mut state = self.lock_state();
        state.allow_core_thread_timeout = allow;
        self.available.notify_all();
    }

    /// Notifies termination waiters when the state is terminal.
    fn notify_if_terminated(&self, state: &ThreadPoolState) {
        if state.is_terminated() {
            self.terminated.notify_all();
        }
    }
}

/// Mutable pool state protected by [`ThreadPoolInner::state`].
struct ThreadPoolState {
    lifecycle: ThreadPoolLifecycle,
    queue: VecDeque<PoolJob>,
    queue_capacity: Option<usize>,
    running_tasks: usize,
    live_workers: usize,
    idle_workers: usize,
    submitted_tasks: usize,
    completed_tasks: usize,
    cancelled_tasks: usize,
    core_pool_size: usize,
    maximum_pool_size: usize,
    keep_alive: Duration,
    allow_core_thread_timeout: bool,
    next_worker_index: usize,
}

impl ThreadPoolState {
    /// Returns whether the queue is currently full.
    fn is_saturated(&self) -> bool {
        self.queue_capacity
            .is_some_and(|capacity| self.queue.len() >= capacity)
    }

    /// Returns whether the service lifecycle is fully terminated.
    fn is_terminated(&self) -> bool {
        !self.lifecycle.is_running()
            && self.queue.is_empty()
            && self.running_tasks == 0
            && self.live_workers == 0
    }

    /// Returns whether an idle worker should use a timed wait.
    fn worker_wait_is_timed(&self) -> bool {
        self.allow_core_thread_timeout || self.live_workers > self.core_pool_size
    }

    /// Returns whether an idle worker may retire now.
    fn idle_worker_can_retire(&self) -> bool {
        self.live_workers > self.maximum_pool_size
            || (self.worker_wait_is_timed()
                && (self.live_workers > self.core_pool_size || self.allow_core_thread_timeout))
    }
}

/// Lifecycle state for a thread pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThreadPoolLifecycle {
    /// The pool accepts new tasks and workers wait for queue items.
    Running,

    /// The pool rejects new tasks but drains queued work.
    Shutdown,

    /// The pool rejects new tasks, cancels queued work, and stops workers.
    Stopping,
}

impl ThreadPoolLifecycle {
    /// Returns whether this lifecycle still accepts new work.
    const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    /// Returns whether this lifecycle represents graceful shutdown.
    const fn is_shutdown(self) -> bool {
        matches!(self, Self::Shutdown)
    }
}

/// Type-erased pool job with a cancellation path for queued work.
pub(crate) struct PoolJob {
    run: Option<Box<dyn FnOnce() + Send + 'static>>,
    cancel: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl PoolJob {
    /// Creates a pool job from run and cancel callbacks.
    pub(crate) fn new(
        run: Box<dyn FnOnce() + Send + 'static>,
        cancel: Box<dyn FnOnce() + Send + 'static>,
    ) -> Self {
        Self {
            run: Some(run),
            cancel: Some(cancel),
        }
    }

    /// Runs this job if it has not been cancelled first.
    fn run(mut self) {
        if let Some(run) = self.run.take() {
            run();
        }
    }

    /// Cancels this queued job if it has not been run first.
    fn cancel(mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel();
        }
    }
}

/// Runs a callable task through a task completion endpoint.
fn run_task<C, R, E>(task: C, completion: TaskCompletion<R, E>)
where
    C: Callable<R, E>,
{
    if completion.start() {
        completion.complete(run_callable(task));
    }
}

/// Runs a single worker loop until the pool asks it to exit.
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
                    let (next_state, timeout) = inner
                        .available
                        .wait_timeout(state, keep_alive)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    state = next_state;
                    state.idle_workers = state
                        .idle_workers
                        .checked_sub(1)
                        .expect("thread pool idle worker counter underflow");
                    if timeout.timed_out()
                        && state.queue.is_empty()
                        && state.idle_worker_can_retire()
                    {
                        unregister_exiting_worker(inner, &mut state);
                        return None;
                    }
                } else {
                    state.idle_workers += 1;
                    state = inner
                        .available
                        .wait(state)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
fn unregister_exiting_worker(inner: &ThreadPoolInner, state: &mut ThreadPoolState) {
    state.live_workers = state
        .live_workers
        .checked_sub(1)
        .expect("thread pool live worker counter underflow");
    inner.notify_if_terminated(state);
}

/// Returns the default worker count for new builders.
fn default_worker_count() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

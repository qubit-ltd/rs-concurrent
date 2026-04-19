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

/// Fixed-size OS thread pool implementing [`ExecutorService`].
///
/// `ThreadPool` accepts fallible tasks, stores them in an internal FIFO queue,
/// and executes them on a fixed set of worker threads. Submitted tasks return
/// [`TaskHandle`], which supports both blocking [`TaskHandle::get`] and async
/// `.await` result retrieval.
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
    /// Creates a fixed-size thread pool with an unbounded queue.
    ///
    /// # Parameters
    ///
    /// * `worker_count` - Number of worker threads to spawn.
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
    worker_count: usize,
    queue_capacity: Option<usize>,
    thread_name_prefix: String,
    stack_size: Option<usize>,
}

impl ThreadPoolBuilder {
    /// Sets the number of worker threads.
    ///
    /// # Parameters
    ///
    /// * `worker_count` - Number of OS worker threads to spawn.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn worker_count(mut self, worker_count: usize) -> Self {
        self.worker_count = worker_count;
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

    /// Builds the configured thread pool.
    ///
    /// # Returns
    ///
    /// `Ok(ThreadPool)` if all workers are spawned successfully. Returns
    /// [`ThreadPoolBuildError`] if the configuration is invalid or a worker
    /// thread cannot be spawned.
    pub fn build(self) -> Result<ThreadPool, ThreadPoolBuildError> {
        self.validate()?;
        let inner = Arc::new(ThreadPoolInner::new(self.queue_capacity));
        for index in 0..self.worker_count {
            inner.register_worker();
            let worker_inner = Arc::clone(&inner);
            let mut builder =
                thread::Builder::new().name(format!("{}-{index}", self.thread_name_prefix));
            if let Some(stack_size) = self.stack_size {
                builder = builder.stack_size(stack_size);
            }
            if let Err(source) = builder.spawn(move || run_worker(worker_inner)) {
                inner.unregister_worker_after_spawn_failure();
                inner.shutdown_now();
                return Err(ThreadPoolBuildError::SpawnWorker { index, source });
            }
        }
        Ok(ThreadPool { inner })
    }

    /// Validates this builder configuration.
    fn validate(&self) -> Result<(), ThreadPoolBuildError> {
        if self.worker_count == 0 {
            return Err(ThreadPoolBuildError::ZeroWorkers);
        }
        if self.queue_capacity == Some(0) {
            return Err(ThreadPoolBuildError::ZeroQueueCapacity);
        }
        if self.stack_size == Some(0) {
            return Err(ThreadPoolBuildError::ZeroStackSize);
        }
        Ok(())
    }
}

impl Default for ThreadPoolBuilder {
    /// Creates a builder with CPU parallelism defaults.
    fn default() -> Self {
        Self {
            worker_count: default_worker_count(),
            queue_capacity: None,
            thread_name_prefix: DEFAULT_THREAD_NAME_PREFIX.to_owned(),
            stack_size: None,
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
    /// The configured worker count is zero.
    #[error("thread pool requires at least one worker")]
    ZeroWorkers,

    /// The configured bounded queue capacity is zero.
    #[error("thread pool queue capacity must be greater than zero")]
    ZeroQueueCapacity,

    /// The configured worker stack size is zero.
    #[error("thread pool stack size must be greater than zero")]
    ZeroStackSize,

    /// A worker thread could not be spawned.
    #[error("failed to spawn thread pool worker {index}: {source}")]
    SpawnWorker {
        /// Index of the worker that failed to spawn.
        index: usize,

        /// I/O error reported by [`std::thread::Builder::spawn`].
        source: io::Error,
    },
}

/// Shared state for a thread pool.
struct ThreadPoolInner {
    state: Mutex<ThreadPoolState>,
    available: Condvar,
    terminated: Condvar,
}

impl ThreadPoolInner {
    /// Creates shared state for a thread pool.
    fn new(queue_capacity: Option<usize>) -> Self {
        Self {
            state: Mutex::new(ThreadPoolState {
                lifecycle: ThreadPoolLifecycle::Running,
                queue: VecDeque::new(),
                queue_capacity,
                running_tasks: 0,
                live_workers: 0,
            }),
            available: Condvar::new(),
            terminated: Condvar::new(),
        }
    }

    /// Acquires the pool state while tolerating poisoned locks.
    fn lock_state(&self) -> MutexGuard<'_, ThreadPoolState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Registers a worker that is about to be spawned.
    fn register_worker(&self) {
        self.lock_state().live_workers += 1;
    }

    /// Reverts a worker registration after spawn failure.
    fn unregister_worker_after_spawn_failure(&self) {
        let mut state = self.lock_state();
        state.live_workers = state
            .live_workers
            .checked_sub(1)
            .expect("thread pool worker registration underflow");
        self.notify_if_terminated(&state);
    }

    /// Submits a job into the queue.
    fn submit(&self, job: PoolJob) -> Result<(), RejectedExecution> {
        let mut state = self.lock_state();
        if !state.lifecycle.is_running() {
            return Err(RejectedExecution::Shutdown);
        }
        if state.is_saturated() {
            return Err(RejectedExecution::Saturated);
        }
        state.queue.push_back(job);
        self.available.notify_one();
        Ok(())
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
struct PoolJob {
    run: Option<Box<dyn FnOnce() + Send + 'static>>,
    cancel: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl PoolJob {
    /// Creates a pool job from run and cancel callbacks.
    fn new(
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
fn run_worker(inner: Arc<ThreadPoolInner>) {
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
                state = inner
                    .available
                    .wait(state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
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

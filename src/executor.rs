/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Executor
//!
//! Provides trait definitions similar to JDK Executor interface for executing submitted tasks.
//!
//! # Author
//!
//! Haixing Hu

use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
};

use tokio::{sync::Notify, task::JoinHandle};

/// Runnable task trait
///
/// Represents a task that can be executed. Similar to the `Runnable` interface in JDK.
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::Runnable;
///
/// struct MyTask {
///     name: String,
/// }
///
/// impl Runnable for MyTask {
///     fn run(&self) {
///         println!("Task {} is executing", self.name);
///     }
/// }
///
/// let task = MyTask {
///     name: String::from("task-1"),
/// };
/// task.run();
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait Runnable: Send {
    /// Executes the task
    ///
    /// This method will be called when the task is scheduled for execution by the executor.
    fn run(&self);
}

/// Callable task trait that returns a result
///
/// Represents a computation task that can return a result. Similar to the `Callable` interface in JDK.
/// Unlike `Runnable`, `Callable` can return a computation result or an error.
///
/// # Type Parameters
///
/// * `T` - The result type returned by the task
/// * `E` - The error type that the task may produce
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::Callable;
///
/// struct ComputeTask {
///     x: i32,
///     y: i32,
/// }
///
/// impl Callable<i32, String> for ComputeTask {
///     fn call(&self) -> Result<i32, String> {
///         if self.y == 0 {
///             Err(String::from("Divisor cannot be zero"))
///         } else {
///             Ok(self.x / self.y)
///         }
///     }
/// }
///
/// let task = ComputeTask { x: 10, y: 2 };
/// match task.call() {
///     Ok(result) => println!("Computation result: {}", result),
///     Err(e) => println!("Computation error: {}", e),
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait Callable<T, E>: Send
where
    T: Send,
    E: Send,
{
    /// Computes a result
    ///
    /// This method will be called when the task is scheduled for execution by the executor.
    /// The method can return a computation result or an error.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Computation succeeded, returns the result
    /// * `Err(E)` - Computation failed, returns an error
    fn call(&self) -> Result<T, E>;
}

/// Synchronous task executor trait
///
/// Provides functionality similar to the JDK `Executor` interface for executing submitted synchronous tasks.
/// The executor is responsible for decoupling the execution of tasks from their submission, so that
/// task submitters do not need to worry about how tasks are executed, which thread they run on, etc.
///
/// # Features
///
/// - Decouples task submission and execution: Task submitters only need to submit tasks without worrying about execution details
/// - Flexible execution strategies: Implementors can freely choose synchronous execution, thread pool execution, and other strategies
/// - Focuses on synchronous tasks: Only handles regular closure tasks, not async Futures
///
/// # Use Cases
///
/// - Thread pools: Submit tasks to a thread pool for execution
/// - Task schedulers: Schedule task execution based on priority, delay, and other policies
/// - CPU-intensive computation: Execute computationally intensive synchronous tasks
/// - Testing environment: Use synchronous executors in tests for easier debugging and verification
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::Executor;
///
/// struct SimpleExecutor;
///
/// impl Executor for SimpleExecutor {
///     fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
///         // Simple synchronous execution
///         task();
///     }
/// }
///
/// let executor = SimpleExecutor;
/// executor.execute(Box::new(|| {
///     println!("Synchronous task executing");
/// }));
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait Executor: Send + Sync {
    /// Executes the given synchronous task
    ///
    /// Submits the task to the executor for execution. The task may be executed synchronously
    /// on the current thread, or asynchronously on another thread, depending on the implementor's strategy.
    ///
    /// # Parameters
    ///
    /// * `task` - The synchronous task to be executed, which must implement the `Send` trait for transfer between threads
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::Executor;
    ///
    /// let executor = create_executor();
    /// executor.execute(Box::new(|| {
    ///     println!("Task is executing");
    /// }));
    /// ```
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>);
}

/// Asynchronous task executor trait
///
/// Provides asynchronous task execution functionality, specifically for executing async Future tasks.
/// Separated from `Executor`, following the single responsibility principle.
///
/// # Features
///
/// - Focuses on asynchronous tasks: Only handles async Futures, not synchronous closures
/// - Non-blocking execution: Tasks execute in an async runtime without blocking threads
/// - Efficient concurrency: Multiple async tasks can share a small number of threads
///
/// # Use Cases
///
/// - Asynchronous I/O operations: Network requests, file I/O, etc.
/// - Concurrent task coordination: Requires large amounts of concurrent async operations
/// - Microservice calls: Asynchronously call multiple services and aggregate results
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::AsyncExecutor;
///
/// struct SimpleAsyncExecutor;
///
/// impl AsyncExecutor for SimpleAsyncExecutor {
///     fn spawn<F>(&self, future: F)
///     where
///         F: Future<Output = ()> + Send + 'static,
///     {
///         // Spawn task in tokio runtime
///         tokio::spawn(future);
///     }
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let executor = SimpleAsyncExecutor;
///     executor.spawn(async {
///         println!("Async task is executing");
///     });
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait AsyncExecutor: Send + Sync {
    /// Spawns and executes an asynchronous task
    ///
    /// Submits an async task to the executor for execution. The task will be executed
    /// in the async runtime managed by the executor, without blocking the current thread.
    ///
    /// # Parameters
    ///
    /// * `future` - The async task to be executed
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::AsyncExecutor;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let executor = create_async_executor();
    ///     executor.spawn(async {
    ///         let data = fetch_data().await;
    ///         process_data(data).await;
    ///     });
    /// }
    /// ```
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

/// Shutdownable executor trait
///
/// Provides lifecycle management functionality similar to the JDK `ExecutorService` interface.
/// Executors implementing this trait can be gracefully shut down, waiting for executing tasks to complete.
///
/// # Features
///
/// - Graceful shutdown: The `shutdown()` method stops accepting new tasks but waits for submitted tasks to complete
/// - Immediate shutdown: The `shutdown_now()` method attempts to stop all executing tasks
/// - State queries: Can query whether the executor is shut down or terminated
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::{Executor, ExecutorService};
///
/// #[tokio::main]
/// async fn main() {
///     let executor = create_executor();
///
///     // Submit task
///     executor.execute(Box::new(|| {
///         println!("Task executing");
///     }));
///
///     // Graceful shutdown
///     executor.shutdown();
///
///     // Wait for all tasks to complete
///     executor.await_termination().await;
///
///     assert!(executor.is_terminated());
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait ExecutorService: Executor {
    /// Initiates an orderly shutdown
    ///
    /// Initiates an orderly shutdown in which previously submitted tasks are executed,
    /// but no new tasks will be accepted. Invocation has no additional effect if already shut down.
    ///
    /// This method does not wait for previously submitted tasks to complete execution.
    /// Use `await_termination()` to do that.
    fn shutdown(&self);

    /// Attempts to stop all actively executing tasks
    ///
    /// Attempts to stop all actively executing tasks, halts the processing of waiting tasks,
    /// and returns a list of the tasks that were awaiting execution.
    ///
    /// This method does not wait for actively executing tasks to terminate.
    /// Use `await_termination()` to do that.
    ///
    /// # Returns
    ///
    /// List of tasks that never commenced execution
    fn shutdown_now(&self) -> Vec<Box<dyn Runnable>>;

    /// Returns true if this executor has been shut down
    ///
    /// # Returns
    ///
    /// `true` if this executor has been shut down
    fn is_shutdown(&self) -> bool;

    /// Returns true if all tasks have completed following shut down
    ///
    /// # Returns
    ///
    /// `true` if all tasks have completed following shut down.
    /// Note that `is_terminated()` is never `true` unless either
    /// `shutdown()` or `shutdown_now()` was called first.
    fn is_terminated(&self) -> bool;

    /// Blocks until all tasks have completed execution
    ///
    /// Blocks until all tasks have completed execution after a shutdown request,
    /// or the timeout occurs, or the current task is interrupted, whichever happens first.
    ///
    /// # Returns
    ///
    /// A future that completes when all tasks have finished
    fn await_termination(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Shutdownable asynchronous executor trait
///
/// Provides lifecycle management functionality for asynchronous executors.
/// This is the async counterpart of `ExecutorService`, specifically for managing
/// the lifecycle of asynchronous task executors.
///
/// # Features
///
/// - Graceful shutdown: The `shutdown()` method stops accepting new async tasks but waits for submitted tasks to complete
/// - Immediate shutdown: The `shutdown_now()` method attempts to stop all executing async tasks
/// - State queries: Can query whether the async executor is shut down or terminated
///
/// # Use Cases
///
/// - Async I/O service shutdown: Gracefully shut down async network or file I/O services
/// - Application cleanup: Ensure all async tasks complete before application exit
/// - Resource management: Control the lifecycle of async task pools
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::{AsyncExecutor, AsyncExecutorService};
///
/// #[tokio::main]
/// async fn main() {
///     let executor = create_async_executor();
///
///     // Submit async task
///     executor.spawn(async {
///         println!("Async task executing");
///     });
///
///     // Graceful shutdown
///     executor.shutdown();
///
///     // Wait for all async tasks to complete
///     executor.await_termination().await;
///
///     assert!(executor.is_terminated());
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub trait AsyncExecutorService: AsyncExecutor {
    /// Initiates an orderly shutdown
    ///
    /// Initiates an orderly shutdown in which previously submitted async tasks are executed,
    /// but no new tasks will be accepted. Invocation has no additional effect if already shut down.
    ///
    /// This method does not wait for previously submitted tasks to complete execution.
    /// Use `await_termination()` to do that.
    fn shutdown(&self);

    /// Attempts to stop all actively executing async tasks
    ///
    /// Attempts to stop all actively executing async tasks and halts the processing of waiting tasks.
    /// Unlike the synchronous version, this method does not return a list of pending tasks
    /// because async tasks cannot be easily captured once spawned.
    ///
    /// This method does not wait for actively executing tasks to terminate.
    /// Use `await_termination()` to do that.
    fn shutdown_now(&self);

    /// Returns true if this async executor has been shut down
    ///
    /// # Returns
    ///
    /// `true` if this async executor has been shut down
    fn is_shutdown(&self) -> bool;

    /// Returns true if all async tasks have completed following shut down
    ///
    /// # Returns
    ///
    /// `true` if all async tasks have completed following shut down.
    /// Note that `is_terminated()` is never `true` unless either
    /// `shutdown()` or `shutdown_now()` was called first.
    fn is_terminated(&self) -> bool;

    /// Blocks until all async tasks have completed execution
    ///
    /// Blocks until all async tasks have completed execution after a shutdown request,
    /// or the timeout occurs, or the current task is interrupted, whichever happens first.
    ///
    /// # Returns
    ///
    /// A future that completes when all async tasks have finished
    fn await_termination(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// Executes synchronous tasks immediately on the caller thread.
///
/// This executor is useful for deterministic tests and simple scenarios where
/// task submission and execution should happen in the same call stack.
#[derive(Debug, Default, Clone, Copy)]
pub struct DirectExecutor;

impl Executor for DirectExecutor {
    /// Executes the task inline on the current thread.
    #[inline]
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        task();
    }
}

/// Executes each submitted synchronous task on a dedicated thread.
///
/// This executor does not track task lifecycle and does not support shutdown.
#[derive(Debug, Default, Clone, Copy)]
pub struct ThreadPerTaskExecutor;

impl Executor for ThreadPerTaskExecutor {
    /// Spawns one OS thread per task and detaches it immediately.
    #[inline]
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        let _ = thread::spawn(move || {
            task();
        });
    }
}

/// Shared state for `ThreadPerTaskExecutorService`.
#[derive(Default)]
struct ThreadPerTaskExecutorServiceState {
    shutdown: AtomicBool,
    active_tasks: AtomicUsize,
    handles: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl ThreadPerTaskExecutorServiceState {
    /// Acquires the thread handle collection while tolerating poisoned locks.
    fn lock_handles(&self) -> MutexGuard<'_, Vec<thread::JoinHandle<()>>> {
        self.handles
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Joins all currently tracked worker threads.
    ///
    /// This method blocks until every tracked thread has completed.
    fn join_all_handles(&self) {
        let mut handles = self.lock_handles();
        let running = std::mem::take(&mut *handles);
        drop(handles);
        for handle in running {
            let _ = handle.join();
        }
    }
}

/// Thread-per-task executor with shutdown and termination coordination.
///
/// This type offers a simple `ExecutorService` implementation for synchronous
/// tasks. `shutdown_now()` cannot forcefully stop running threads; it only
/// stops accepting new tasks.
#[derive(Default, Clone)]
pub struct ThreadPerTaskExecutorService {
    state: Arc<ThreadPerTaskExecutorServiceState>,
}

impl ThreadPerTaskExecutorService {
    /// Creates a new service instance.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Executor for ThreadPerTaskExecutorService {
    /// Spawns a dedicated worker thread for the task when the service is active.
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        if self.state.shutdown.load(Ordering::Acquire) {
            return;
        }
        self.state.active_tasks.fetch_add(1, Ordering::AcqRel);
        let state = Arc::clone(&self.state);
        let handle = thread::spawn(move || {
            task();
            state.active_tasks.fetch_sub(1, Ordering::AcqRel);
        });
        self.state.lock_handles().push(handle);
    }
}

impl ExecutorService for ThreadPerTaskExecutorService {
    /// Stops accepting new synchronous tasks.
    fn shutdown(&self) {
        self.state.shutdown.store(true, Ordering::Release);
    }

    /// Stops accepting new synchronous tasks and returns no queued tasks.
    ///
    /// The service executes submitted work immediately, so there is no pending
    /// task queue to return.
    fn shutdown_now(&self) -> Vec<Box<dyn Runnable>> {
        self.state.shutdown.store(true, Ordering::Release);
        Vec::new()
    }

    /// Returns whether shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.state.shutdown.load(Ordering::Acquire)
    }

    /// Returns whether shutdown was requested and all tasks are finished.
    fn is_terminated(&self) -> bool {
        self.is_shutdown() && self.state.active_tasks.load(Ordering::Acquire) == 0
    }

    /// Waits for all tracked worker threads to complete.
    ///
    /// This future blocks the current thread while joining worker threads.
    fn await_termination(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            self.state.join_all_handles();
        })
    }
}

/// Executes asynchronous tasks by delegating to `tokio::spawn`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioExecutor;

impl AsyncExecutor for TokioExecutor {
    /// Spawns the future onto the current Tokio runtime.
    #[inline]
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        std::mem::drop(tokio::spawn(future));
    }
}

/// Shared state for `TokioExecutorService`.
#[derive(Default)]
struct TokioExecutorServiceState {
    shutdown: AtomicBool,
    active_tasks: AtomicUsize,
    handles: Mutex<Vec<JoinHandle<()>>>,
    terminated_notify: Notify,
}

impl TokioExecutorServiceState {
    /// Acquires the task handle collection while tolerating poisoned locks.
    fn lock_handles(&self) -> MutexGuard<'_, Vec<JoinHandle<()>>> {
        self.handles
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Drops handles for tasks that have already completed.
    fn cleanup_finished_handles(&self) {
        self.lock_handles().retain(|handle| !handle.is_finished());
    }

    /// Notifies awaiters if shutdown has been requested and no tasks remain.
    fn notify_if_terminated(&self) {
        if self.shutdown.load(Ordering::Acquire) && self.active_tasks.load(Ordering::Acquire) == 0 {
            self.terminated_notify.notify_waiters();
        }
    }
}

/// Task lifecycle guard for `TokioExecutorService`.
///
/// Decrements active task count and signals termination on drop.
struct TokioTaskGuard {
    state: Arc<TokioExecutorServiceState>,
}

impl TokioTaskGuard {
    /// Creates a new lifecycle guard.
    #[inline]
    fn new(state: Arc<TokioExecutorServiceState>) -> Self {
        Self { state }
    }
}

impl Drop for TokioTaskGuard {
    /// Updates service counters when a task completes or is cancelled.
    fn drop(&mut self) {
        if self.state.active_tasks.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.state.notify_if_terminated();
        }
    }
}

/// Tokio-backed async executor with lifecycle control.
///
/// After shutdown, new tasks are ignored. `shutdown_now()` aborts tracked tasks.
#[derive(Default, Clone)]
pub struct TokioExecutorService {
    state: Arc<TokioExecutorServiceState>,
}

impl TokioExecutorService {
    /// Creates a new service instance.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl AsyncExecutor for TokioExecutorService {
    /// Spawns an async task while the service is accepting work.
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if self.state.shutdown.load(Ordering::Acquire) {
            return;
        }
        self.state.active_tasks.fetch_add(1, Ordering::AcqRel);
        let state = Arc::clone(&self.state);
        let handle = tokio::spawn(async move {
            let _guard = TokioTaskGuard::new(state);
            future.await;
        });
        self.state.cleanup_finished_handles();
        self.state.lock_handles().push(handle);
    }
}

impl AsyncExecutorService for TokioExecutorService {
    /// Stops accepting new async tasks.
    fn shutdown(&self) {
        self.state.shutdown.store(true, Ordering::Release);
        self.state.notify_if_terminated();
    }

    /// Stops accepting new tasks and aborts tracked Tokio tasks.
    fn shutdown_now(&self) {
        self.state.shutdown.store(true, Ordering::Release);
        let mut handles = self.state.lock_handles();
        let draining = std::mem::take(&mut *handles);
        drop(handles);
        for handle in draining {
            handle.abort();
        }
        self.state.notify_if_terminated();
    }

    /// Returns whether shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.state.shutdown.load(Ordering::Acquire)
    }

    /// Returns whether shutdown was requested and no active tasks remain.
    fn is_terminated(&self) -> bool {
        self.is_shutdown() && self.state.active_tasks.load(Ordering::Acquire) == 0
    }

    /// Waits until shutdown is requested and all active tasks complete.
    fn await_termination(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            loop {
                self.state.cleanup_finished_handles();
                if self.is_terminated() {
                    return;
                }
                self.state.terminated_notify.notified().await;
            }
        })
    }
}

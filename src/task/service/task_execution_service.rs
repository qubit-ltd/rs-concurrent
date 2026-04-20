/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{
    collections::HashMap,
    panic::{
        AssertUnwindSafe,
        catch_unwind,
    },
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
    },
};

use qubit_function::{
    Callable,
    Runnable,
};
use thiserror::Error;

use crate::task::{
    TaskCompletion,
    TaskExecutionError,
    TaskHandle,
};

use super::{
    ExecutorService,
    RejectedExecution,
    ShutdownReport,
    ThreadPool,
    ThreadPoolBuildError,
    ThreadPoolBuilder,
    thread_pool::PoolJob,
};

/// Identifier type used by [`TaskExecutionService`].
pub type TaskId = u64;

/// Status of a task managed by [`TaskExecutionService`].
///
/// The status describes service-level progress, not the typed task result
/// stored in [`TaskHandle`]. The handle remains the source of truth for the
/// task's success value or error value.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// The task was accepted but has not started running.
    Submitted,

    /// A worker has started running the task.
    Running,

    /// The task completed successfully.
    Succeeded,

    /// The task ran and returned its own error value.
    Failed,

    /// The task panicked while running.
    Panicked,

    /// The task was cancelled before it started.
    Cancelled,
}

impl TaskStatus {
    /// Returns whether this status represents in-flight work.
    ///
    /// # Returns
    ///
    /// `true` for submitted or running tasks.
    #[inline]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Submitted | Self::Running)
    }
}

/// Count snapshot for a [`TaskExecutionService`].
///
/// Counters are derived from the service registry and therefore include
/// terminal task records retained for inspection.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TaskExecutionStats {
    /// Number of tasks accepted by the service.
    pub total: usize,

    /// Number of accepted tasks not yet started.
    pub submitted: usize,

    /// Number of tasks currently running.
    pub running: usize,

    /// Number of tasks that completed successfully.
    pub succeeded: usize,

    /// Number of tasks that returned an error.
    pub failed: usize,

    /// Number of tasks that panicked.
    pub panicked: usize,

    /// Number of tasks cancelled before start.
    pub cancelled: usize,
}

impl TaskExecutionStats {
    /// Adds one task status to this snapshot.
    fn add_status(&mut self, status: TaskStatus) {
        self.total += 1;
        match status {
            TaskStatus::Submitted => self.submitted += 1,
            TaskStatus::Running => self.running += 1,
            TaskStatus::Succeeded => self.succeeded += 1,
            TaskStatus::Failed => self.failed += 1,
            TaskStatus::Panicked => self.panicked += 1,
            TaskStatus::Cancelled => self.cancelled += 1,
        }
    }
}

/// Error returned when [`TaskExecutionService`] cannot accept a task.
///
/// This error is about the service submission path. The accepted task's own
/// result is still reported through [`TaskHandle`].
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Error)]
pub enum TaskExecutionServiceError {
    /// Another retained task record already uses the same task ID.
    #[error("task {0} already exists")]
    DuplicateTask(TaskId),

    /// The service is suspended and temporarily refuses new tasks.
    #[error("task execution service is suspended")]
    Suspended,

    /// The underlying thread pool rejected the task.
    #[error(transparent)]
    Rejected(#[from] RejectedExecution),
}

/// Higher-level task management service built on [`ThreadPool`].
///
/// The service owns business task IDs and status tracking. It does not expose
/// queue internals and does not inherit thread-pool configuration APIs except
/// through explicit delegation methods.
///
/// # Author
///
/// Haixing Hu
pub struct TaskExecutionService {
    pool: ThreadPool,
    state: Arc<TaskExecutionServiceState>,
}

impl TaskExecutionService {
    /// Creates a service backed by a default [`ThreadPool`].
    ///
    /// # Returns
    ///
    /// `Ok(TaskExecutionService)` if the backing pool is created.
    pub fn new() -> Result<Self, ThreadPoolBuildError> {
        Self::builder().build()
    }

    /// Creates a builder for configuring the backing thread pool.
    ///
    /// # Returns
    ///
    /// A service builder with default pool settings.
    #[inline]
    pub fn builder() -> TaskExecutionServiceBuilder {
        TaskExecutionServiceBuilder::default()
    }

    /// Submits a runnable task with a business task ID.
    ///
    /// # Parameters
    ///
    /// * `task_id` - Stable business ID for registry operations.
    /// * `task` - Runnable to execute.
    ///
    /// # Returns
    ///
    /// `Ok(handle)` if the service accepts the task. This only means
    /// acceptance; task success is observed through the handle. Returns
    /// [`TaskExecutionServiceError`] when the ID is duplicated, the service is
    /// suspended, or the backing pool rejects the task.
    #[inline]
    pub fn submit<T, E>(
        &self,
        task_id: TaskId,
        task: T,
    ) -> Result<TaskHandle<(), E>, TaskExecutionServiceError>
    where
        T: Runnable<E> + Send + 'static,
        E: Send + 'static,
    {
        self.submit_callable(task_id, move || task.run())
    }

    /// Submits a callable task with a business task ID.
    ///
    /// # Parameters
    ///
    /// * `task_id` - Stable business ID for registry operations.
    /// * `task` - Callable to execute.
    ///
    /// # Returns
    ///
    /// `Ok(handle)` if the service accepts the task. The handle reports the
    /// typed task result while this service records only service-level status.
    pub fn submit_callable<C, R, E>(
        &self,
        task_id: TaskId,
        task: C,
    ) -> Result<TaskHandle<R, E>, TaskExecutionServiceError>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let (handle, completion) = TaskHandle::completion_pair();
        let cancel_completion = completion.clone();
        let cancel_state = Arc::clone(&self.state);
        let cancel: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(move || {
            let cancelled = cancel_completion.cancel();
            if cancelled {
                cancel_state.set_status(task_id, TaskStatus::Cancelled);
            }
            cancelled
        });

        self.state.register(task_id, Arc::clone(&cancel))?;

        let run_state = Arc::clone(&self.state);
        let job = PoolJob::new(
            Box::new(move || run_tracked_task(task_id, task, completion, run_state)),
            Box::new(move || {
                cancel();
            }),
        );

        if let Err(error) = self.pool.submit_job(job) {
            self.state.remove(task_id);
            return Err(error.into());
        }
        Ok(handle)
    }

    /// Attempts to cancel a submitted task by ID.
    ///
    /// Cancellation succeeds only before the task starts running.
    ///
    /// # Parameters
    ///
    /// * `task_id` - ID of the task to cancel.
    ///
    /// # Returns
    ///
    /// `true` if the task was cancelled before start, or `false` if no active
    /// task with this ID can be cancelled.
    pub fn cancel(&self, task_id: TaskId) -> bool {
        let cancel = self.state.cancel_callback(task_id);
        cancel.is_some_and(|cancel| cancel())
    }

    /// Returns the current status of a task.
    ///
    /// # Parameters
    ///
    /// * `task_id` - ID of the task to inspect.
    ///
    /// # Returns
    ///
    /// `Some(status)` if the service retains a record for this ID, or `None`
    /// if the ID is unknown.
    #[inline]
    pub fn status(&self, task_id: TaskId) -> Option<TaskStatus> {
        self.state.status(task_id)
    }

    /// Returns registry-derived task statistics.
    ///
    /// # Returns
    ///
    /// A snapshot of retained task records grouped by status.
    #[inline]
    pub fn stats(&self) -> TaskExecutionStats {
        self.state.stats()
    }

    /// Suspends new submissions.
    ///
    /// Existing submitted and running tasks continue normally.
    #[inline]
    pub fn suspend(&self) {
        self.state.set_suspended(true);
    }

    /// Resumes accepting new submissions.
    #[inline]
    pub fn resume(&self) {
        self.state.set_suspended(false);
    }

    /// Returns whether the service is suspended.
    ///
    /// # Returns
    ///
    /// `true` if new submissions are rejected before reaching the pool.
    #[inline]
    pub fn is_suspended(&self) -> bool {
        self.state.is_suspended()
    }

    /// Waits for the active task snapshot observed at call time to finish.
    ///
    /// Tasks submitted after this method starts are not part of the waited
    /// snapshot. This method blocks the current thread.
    pub fn await_in_flight_tasks_completion(&self) {
        self.state.await_in_flight_tasks_completion();
    }

    /// Waits until the service registry has no submitted or running tasks.
    ///
    /// This method blocks the current thread and observes real-time idleness.
    pub fn await_idle(&self) {
        self.state.await_idle();
    }

    /// Initiates graceful shutdown of the backing pool.
    #[inline]
    pub fn shutdown(&self) {
        self.pool.shutdown();
    }

    /// Initiates immediate shutdown of the backing pool.
    ///
    /// # Returns
    ///
    /// A count-based report from the backing pool.
    #[inline]
    pub fn shutdown_now(&self) -> ShutdownReport {
        self.pool.shutdown_now()
    }

    /// Returns whether the backing pool has begun shutdown.
    #[inline]
    pub fn is_shutdown(&self) -> bool {
        self.pool.is_shutdown()
    }

    /// Returns whether the backing pool has terminated.
    #[inline]
    pub fn is_terminated(&self) -> bool {
        self.pool.is_terminated()
    }

    /// Waits until the backing pool has terminated.
    ///
    /// # Returns
    ///
    /// A future that completes after shutdown and worker exit.
    #[inline]
    pub fn await_termination(&self) -> <ThreadPool as ExecutorService>::Termination<'_> {
        self.pool.await_termination()
    }

    /// Returns the backing thread pool.
    ///
    /// # Returns
    ///
    /// A shared reference for low-level inspection such as pool statistics.
    #[inline]
    pub fn thread_pool(&self) -> &ThreadPool {
        &self.pool
    }
}

/// Builder for [`TaskExecutionService`].
///
/// The builder currently delegates to [`ThreadPoolBuilder`]. It exists so the
/// service can grow service-level configuration without leaking pool details
/// into the main service type.
#[derive(Debug, Default, Clone)]
pub struct TaskExecutionServiceBuilder {
    pool_builder: ThreadPoolBuilder,
}

impl TaskExecutionServiceBuilder {
    /// Sets the backing pool builder.
    ///
    /// # Parameters
    ///
    /// * `pool_builder` - Builder used to create the backing pool.
    ///
    /// # Returns
    ///
    /// This builder for fluent configuration.
    #[inline]
    pub fn thread_pool(mut self, pool_builder: ThreadPoolBuilder) -> Self {
        self.pool_builder = pool_builder;
        self
    }

    /// Builds the task execution service.
    ///
    /// # Returns
    ///
    /// `Ok(TaskExecutionService)` if the backing pool can be built.
    pub fn build(self) -> Result<TaskExecutionService, ThreadPoolBuildError> {
        Ok(TaskExecutionService {
            pool: self.pool_builder.build()?,
            state: Arc::new(TaskExecutionServiceState::default()),
        })
    }
}

/// Shared state for [`TaskExecutionService`].
#[derive(Default)]
struct TaskExecutionServiceState {
    inner: Mutex<TaskExecutionServiceInner>,
    idle: Condvar,
}

impl TaskExecutionServiceState {
    /// Acquires service state while tolerating poisoned locks.
    fn lock_inner(&self) -> MutexGuard<'_, TaskExecutionServiceInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Registers a submitted task.
    fn register(
        &self,
        task_id: TaskId,
        cancel: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<(), TaskExecutionServiceError> {
        let mut inner = self.lock_inner();
        if inner.suspended {
            return Err(TaskExecutionServiceError::Suspended);
        }
        if inner.tasks.contains_key(&task_id) {
            return Err(TaskExecutionServiceError::DuplicateTask(task_id));
        }
        inner.tasks.insert(
            task_id,
            TaskRecord {
                status: TaskStatus::Submitted,
                cancel,
            },
        );
        Ok(())
    }

    /// Removes a task record.
    fn remove(&self, task_id: TaskId) {
        let mut inner = self.lock_inner();
        inner.tasks.remove(&task_id);
        self.idle.notify_all();
    }

    /// Gets a task status.
    fn status(&self, task_id: TaskId) -> Option<TaskStatus> {
        self.lock_inner()
            .tasks
            .get(&task_id)
            .map(|record| record.status)
    }

    /// Gets a task cancel callback if the task is active.
    fn cancel_callback(&self, task_id: TaskId) -> Option<Arc<dyn Fn() -> bool + Send + Sync>> {
        let inner = self.lock_inner();
        let record = inner.tasks.get(&task_id)?;
        record
            .status
            .is_active()
            .then(|| Arc::clone(&record.cancel))
    }

    /// Updates a task status.
    fn set_status(&self, task_id: TaskId, status: TaskStatus) {
        let mut inner = self.lock_inner();
        if let Some(record) = inner.tasks.get_mut(&task_id) {
            record.status = status;
        }
        self.idle.notify_all();
    }

    /// Updates suspended flag.
    fn set_suspended(&self, suspended: bool) {
        self.lock_inner().suspended = suspended;
    }

    /// Returns whether new submissions are suspended.
    fn is_suspended(&self) -> bool {
        self.lock_inner().suspended
    }

    /// Returns task statistics.
    fn stats(&self) -> TaskExecutionStats {
        let inner = self.lock_inner();
        let mut stats = TaskExecutionStats::default();
        for record in inner.tasks.values() {
            stats.add_status(record.status);
        }
        stats
    }

    /// Waits for active task IDs observed at call time.
    fn await_in_flight_tasks_completion(&self) {
        let mut inner = self.lock_inner();
        let task_ids = inner
            .tasks
            .iter()
            .filter_map(|(&task_id, record)| record.status.is_active().then_some(task_id))
            .collect::<Vec<_>>();
        while task_ids
            .iter()
            .any(|task_id| inner.task_is_active(*task_id))
        {
            inner = self
                .idle
                .wait(inner)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    /// Waits until no retained task record is active.
    fn await_idle(&self) {
        let mut inner = self.lock_inner();
        while inner.has_active_tasks() {
            inner = self
                .idle
                .wait(inner)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// Mutable service state protected by a mutex.
#[derive(Default)]
struct TaskExecutionServiceInner {
    suspended: bool,
    tasks: HashMap<TaskId, TaskRecord>,
}

impl TaskExecutionServiceInner {
    /// Returns whether a retained task ID is still active.
    fn task_is_active(&self, task_id: TaskId) -> bool {
        self.tasks
            .get(&task_id)
            .is_some_and(|record| record.status.is_active())
    }

    /// Returns whether any retained task is still active.
    fn has_active_tasks(&self) -> bool {
        self.tasks.values().any(|record| record.status.is_active())
    }
}

/// Registry record for one managed task.
struct TaskRecord {
    status: TaskStatus,
    cancel: Arc<dyn Fn() -> bool + Send + Sync>,
}

/// Runs a task and updates service status around the typed handle result.
fn run_tracked_task<C, R, E>(
    task_id: TaskId,
    task: C,
    completion: TaskCompletion<R, E>,
    state: Arc<TaskExecutionServiceState>,
) where
    C: Callable<R, E>,
{
    if !completion.start() {
        state.set_status(task_id, TaskStatus::Cancelled);
        return;
    }
    state.set_status(task_id, TaskStatus::Running);
    match catch_unwind(AssertUnwindSafe(|| task.call())) {
        Ok(Ok(value)) => {
            state.set_status(task_id, TaskStatus::Succeeded);
            completion.complete(Ok(value));
        }
        Ok(Err(error)) => {
            state.set_status(task_id, TaskStatus::Failed);
            completion.complete(Err(TaskExecutionError::Failed(error)));
        }
        Err(_) => {
            state.set_status(task_id, TaskStatus::Panicked);
            completion.complete(Err(TaskExecutionError::Panicked));
        }
    }
}

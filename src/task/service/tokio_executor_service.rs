/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::sync::{
    Arc,
    Mutex,
    MutexGuard,
    atomic::{
        AtomicBool,
        AtomicUsize,
        Ordering,
    },
};

use qubit_function::Callable;
use tokio::{
    sync::Notify,
    task::AbortHandle,
};

use crate::task::{
    TaskExecutionError,
    task_runner::run_callable,
};

use super::{
    ExecutorService,
    RejectedExecution,
    ShutdownReport,
    TokioTaskHandle,
};

/// Shared state for [`TokioExecutorService`].
#[derive(Default)]
struct TokioExecutorServiceState {
    shutdown: AtomicBool,
    active_tasks: AtomicUsize,
    submission_lock: Mutex<()>,
    abort_handles: Mutex<Vec<AbortHandle>>,
    terminated_notify: Notify,
}

impl TokioExecutorServiceState {
    /// Acquires the submission lock while tolerating poisoned locks.
    fn lock_submission(&self) -> MutexGuard<'_, ()> {
        self.submission_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Acquires the abort handle list while tolerating poisoned locks.
    fn lock_abort_handles(&self) -> MutexGuard<'_, Vec<AbortHandle>> {
        self.abort_handles
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Wakes termination waiters when shutdown and task completion allow it.
    fn notify_if_terminated(&self) {
        if self.shutdown.load(Ordering::Acquire) && self.active_tasks.load(Ordering::Acquire) == 0 {
            self.terminated_notify.notify_waiters();
        }
    }
}

/// Task lifecycle guard for [`TokioExecutorService`].
struct TokioServiceTaskGuard {
    state: Arc<TokioExecutorServiceState>,
}

impl TokioServiceTaskGuard {
    /// Creates a guard that decrements the active task count on drop.
    fn new(state: Arc<TokioExecutorServiceState>) -> Self {
        Self { state }
    }
}

impl Drop for TokioServiceTaskGuard {
    /// Updates service counters when a task completes or is aborted.
    fn drop(&mut self) {
        if self.state.active_tasks.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.state.notify_if_terminated();
        }
    }
}

/// Tokio-backed service for submitted blocking tasks.
///
/// The service accepts fallible [`Runnable`](crate::task::Runnable) and
/// [`Callable`](crate::task::Callable) tasks, runs them through Tokio, and
/// returns awaitable handles for their final results.
#[derive(Default, Clone)]
pub struct TokioExecutorService {
    state: Arc<TokioExecutorServiceState>,
}

impl TokioExecutorService {
    /// Creates a new service instance.
    ///
    /// # Returns
    ///
    /// A Tokio-backed executor service.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExecutorService for TokioExecutorService {
    type Handle<R, E>
        = TokioTaskHandle<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    type Termination<'a>
        = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    /// Accepts a callable and runs it through Tokio.
    fn submit_callable<C, R, E>(&self, task: C) -> Result<Self::Handle<R, E>, RejectedExecution>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let submission_guard = self.state.lock_submission();
        if self.state.shutdown.load(Ordering::Acquire) {
            return Err(RejectedExecution::Shutdown);
        }
        self.state.active_tasks.fetch_add(1, Ordering::AcqRel);
        drop(submission_guard);

        let state = Arc::clone(&self.state);
        let handle = tokio::spawn(async move {
            let _guard = TokioServiceTaskGuard::new(state);
            match tokio::task::spawn_blocking(move || run_callable(task)).await {
                Ok(result) => result,
                Err(error) if error.is_cancelled() => Err(TaskExecutionError::Cancelled),
                Err(_) => Err(TaskExecutionError::Panicked),
            }
        });
        self.state.lock_abort_handles().push(handle.abort_handle());
        Ok(TokioTaskHandle::new(handle))
    }

    /// Stops accepting new tasks.
    fn shutdown(&self) {
        let _guard = self.state.lock_submission();
        self.state.shutdown.store(true, Ordering::Release);
        self.state.notify_if_terminated();
    }

    /// Stops accepting new tasks and aborts tracked Tokio tasks.
    fn shutdown_now(&self) -> ShutdownReport {
        let _guard = self.state.lock_submission();
        self.state.shutdown.store(true, Ordering::Release);
        let running = self.state.active_tasks.load(Ordering::Acquire);
        let mut handles = self.state.lock_abort_handles();
        let cancellation_count = handles.len();
        for handle in handles.drain(..) {
            handle.abort();
        }
        drop(handles);
        self.state.notify_if_terminated();
        ShutdownReport::new(0, running, cancellation_count)
    }

    /// Returns whether shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.state.shutdown.load(Ordering::Acquire)
    }

    /// Returns whether shutdown was requested and all tasks are finished.
    fn is_terminated(&self) -> bool {
        self.is_shutdown() && self.state.active_tasks.load(Ordering::Acquire) == 0
    }

    /// Waits until the service has terminated.
    fn await_termination(&self) -> Self::Termination<'_> {
        Box::pin(async move {
            loop {
                if self.is_terminated() {
                    return;
                }
                self.state.terminated_notify.notified().await;
            }
        })
    }
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
        atomic::{
            AtomicBool,
            AtomicUsize,
            Ordering,
        },
    },
    thread,
};

use qubit_function::Callable;

use crate::task::{
    TaskHandle,
    task_runner::run_callable,
};

use super::{
    ExecutorService,
    RejectedExecution,
    ShutdownReport,
};

/// Shared state for [`ThreadPerTaskExecutorService`].
#[derive(Default)]
struct ThreadPerTaskExecutorServiceState {
    shutdown: AtomicBool,
    active_tasks: AtomicUsize,
    submission_lock: Mutex<()>,
    termination_lock: Mutex<()>,
    termination: Condvar,
}

impl ThreadPerTaskExecutorServiceState {
    /// Acquires the submission lock while tolerating poisoned locks.
    fn lock_submission(&self) -> MutexGuard<'_, ()> {
        self.submission_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Acquires the termination lock while tolerating poisoned locks.
    fn lock_termination(&self) -> MutexGuard<'_, ()> {
        self.termination_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Wakes termination waiters when shutdown and task completion allow it.
    fn notify_if_terminated(&self) {
        if self.shutdown.load(Ordering::Acquire) && self.active_tasks.load(Ordering::Acquire) == 0 {
            self.termination.notify_all();
        }
    }

    /// Blocks the current thread until the service is terminated.
    fn wait_for_termination(&self) {
        let mut guard = self.lock_termination();
        while !(self.shutdown.load(Ordering::Acquire)
            && self.active_tasks.load(Ordering::Acquire) == 0)
        {
            guard = self
                .termination
                .wait(guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// Managed service that runs every accepted task on a dedicated OS thread.
///
/// The service has no queue: accepted tasks start immediately on their own
/// thread. Shutdown prevents later submissions but cannot forcefully stop
/// running OS threads.
#[derive(Default, Clone)]
pub struct ThreadPerTaskExecutorService {
    state: Arc<ThreadPerTaskExecutorServiceState>,
}

impl ThreadPerTaskExecutorService {
    /// Creates a new service instance.
    ///
    /// # Returns
    ///
    /// A service that accepts tasks until shutdown is requested.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ExecutorService for ThreadPerTaskExecutorService {
    type Handle<R, E>
        = TaskHandle<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    type Termination<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + 'a>>
    where
        Self: 'a;

    /// Accepts a callable and starts it on a dedicated OS thread.
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

        let (handle, sender, done) = TaskHandle::channel();
        let state = Arc::clone(&self.state);
        thread::spawn(move || {
            let result = run_callable(task);
            let _ = sender.send(result);
            done.store(true, Ordering::Release);
            if state.active_tasks.fetch_sub(1, Ordering::AcqRel) == 1 {
                state.notify_if_terminated();
            }
        });
        Ok(handle)
    }

    /// Stops accepting new tasks.
    fn shutdown(&self) {
        let _guard = self.state.lock_submission();
        self.state.shutdown.store(true, Ordering::Release);
        self.state.notify_if_terminated();
    }

    /// Stops accepting new tasks and reports currently running work.
    ///
    /// Running OS threads cannot be forcefully stopped by this service.
    fn shutdown_now(&self) -> ShutdownReport {
        let _guard = self.state.lock_submission();
        self.state.shutdown.store(true, Ordering::Release);
        let running = self.state.active_tasks.load(Ordering::Acquire);
        self.state.notify_if_terminated();
        ShutdownReport::new(0, running, 0)
    }

    /// Returns whether shutdown has been requested.
    fn is_shutdown(&self) -> bool {
        self.state.shutdown.load(Ordering::Acquire)
    }

    /// Returns whether shutdown was requested and all tasks are finished.
    fn is_terminated(&self) -> bool {
        self.is_shutdown() && self.state.active_tasks.load(Ordering::Acquire) == 0
    }

    /// Waits for all accepted tasks to complete after shutdown.
    ///
    /// This future blocks the polling thread while waiting on a condition
    /// variable.
    fn await_termination(&self) -> Self::Termination<'_> {
        Box::pin(async move {
            self.state.wait_for_termination();
        })
    }
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{
    thread,
    time::Duration,
};

use qubit_function::Callable;

use crate::task::{
    TaskHandle,
    task_runner::run_callable,
};

use super::Executor;

/// Executor that starts each task after a fixed delay.
///
/// `DelayExecutor` models delayed start, not minimum execution duration. The
/// returned [`TaskHandle`] is created immediately. A helper thread sleeps for
/// the configured delay and then runs the task. Dropping the handle does not
/// cancel the helper thread.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayExecutor {
    delay: Duration,
}

impl DelayExecutor {
    /// Creates an executor that delays task start by the supplied duration.
    ///
    /// # Parameters
    ///
    /// * `delay` - Duration to wait before running each task.
    ///
    /// # Returns
    ///
    /// A delay executor using the supplied delay.
    #[inline]
    pub const fn new(delay: Duration) -> Self {
        Self { delay }
    }

    /// Returns the configured delay.
    ///
    /// # Returns
    ///
    /// The duration waited before each task starts.
    #[inline]
    pub const fn delay(&self) -> Duration {
        self.delay
    }
}

impl Executor for DelayExecutor {
    type Execution<R, E>
        = TaskHandle<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    /// Starts a helper thread that waits and then runs the callable.
    fn call<C, R, E>(&self, task: C) -> Self::Execution<R, E>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let (handle, completion) = TaskHandle::completion_pair();
        let delay = self.delay;
        thread::spawn(move || {
            if !delay.is_zero() {
                thread::sleep(delay);
            }
            if completion.start() {
                completion.complete(run_callable(task));
            }
        });
        handle
    }
}

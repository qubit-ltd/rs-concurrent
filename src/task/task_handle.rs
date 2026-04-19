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
    atomic::{
        AtomicBool,
        Ordering,
    },
    mpsc::{
        self,
        Receiver,
        Sender,
    },
};

use super::{
    TaskExecutionError,
    TaskResult,
};

/// Blocking handle for a task running outside the caller's current stack.
///
/// `TaskHandle` is returned by thread-backed executors and services. Calling
/// [`Self::get`] waits until the accepted task completes and then reports the
/// final task result.
///
/// # Type Parameters
///
/// * `R` - The task success value.
/// * `E` - The task error value.
///
/// # Author
///
/// Haixing Hu
pub struct TaskHandle<R, E> {
    receiver: Receiver<TaskResult<R, E>>,
    done: Arc<AtomicBool>,
}

impl<R, E> TaskHandle<R, E> {
    /// Creates a handle and the completion pieces used by a task runner.
    ///
    /// # Returns
    ///
    /// A handle for the caller, a sender for the runner, and a completion flag
    /// that the runner must set after sending or dropping the result.
    pub(crate) fn channel() -> (Self, Sender<TaskResult<R, E>>, Arc<AtomicBool>) {
        let (sender, receiver) = mpsc::channel();
        let done = Arc::new(AtomicBool::new(false));
        (
            Self {
                receiver,
                done: Arc::clone(&done),
            },
            sender,
            done,
        )
    }

    /// Waits for the task to finish and returns its final result.
    ///
    /// # Returns
    ///
    /// `Ok(R)` if the task succeeds. If the accepted task returns `Err(E)`,
    /// panics, or is cancelled before producing a value, the corresponding
    /// [`TaskExecutionError`] is returned.
    pub fn get(self) -> TaskResult<R, E> {
        self.receiver
            .recv()
            .unwrap_or(Err(TaskExecutionError::Cancelled))
    }

    /// Returns whether the task has reported completion.
    ///
    /// # Returns
    ///
    /// `true` after the task runner has produced or abandoned its final result.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Attempts to cancel the task.
    ///
    /// Thread-backed handles cannot forcefully cancel an already running OS
    /// thread, so this method currently returns `false`.
    ///
    /// # Returns
    ///
    /// Always `false` for this handle type.
    #[inline]
    pub const fn cancel(&self) -> bool {
        false
    }
}

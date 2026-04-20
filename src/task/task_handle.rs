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
    },
    task::{
        Context,
        Poll,
        Waker,
    },
};

use qubit_atomic::AtomicBool;

use super::{
    TaskExecutionError,
    TaskResult,
};

/// Handle for a task running outside the caller's current stack.
///
/// `TaskHandle` is returned by thread-backed executors and services. Calling
/// [`Self::get`] blocks the current thread until the accepted task completes.
/// Awaiting the handle waits asynchronously for the same final task result.
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
    inner: Arc<TaskHandleInner<R, E>>,
}

/// Shared state used by a task handle and its completing runner.
struct TaskHandleInner<R, E> {
    state: Mutex<TaskHandleState<R, E>>,
    completed: Condvar,
    done: AtomicBool,
}

/// Mutable completion state protected by the task handle mutex.
struct TaskHandleState<R, E> {
    result: Option<TaskResult<R, E>>,
    started: bool,
    completed: bool,
    waker: Option<Waker>,
}

/// Completion endpoint owned by the task runner.
pub(crate) struct TaskCompletion<R, E> {
    inner: Arc<TaskHandleInner<R, E>>,
}

impl<R, E> TaskHandle<R, E> {
    /// Creates a handle and completion endpoint used by a task runner.
    ///
    /// # Returns
    ///
    /// A handle for the caller and a completion endpoint for the runner.
    pub(crate) fn completion_pair() -> (Self, TaskCompletion<R, E>) {
        let inner = Arc::new(TaskHandleInner {
            state: Mutex::new(TaskHandleState {
                result: None,
                started: false,
                completed: false,
                waker: None,
            }),
            completed: Condvar::new(),
            done: AtomicBool::new(false),
        });
        let handle = Self {
            inner: Arc::clone(&inner),
        };
        let completion = TaskCompletion { inner };
        (handle, completion)
    }

    /// Waits for the task to finish and returns its final result.
    ///
    /// This method blocks the current thread until a result is available.
    ///
    /// # Returns
    ///
    /// `Ok(R)` if the task succeeds. If the accepted task returns `Err(E)`,
    /// panics, or is cancelled before producing a value, the corresponding
    /// [`TaskExecutionError`] is returned.
    pub fn get(self) -> TaskResult<R, E> {
        let mut state = self.inner.lock_state();
        loop {
            if state.completed {
                return state
                    .result
                    .take()
                    .expect("task handle completed without a result");
            }
            state = self
                .inner
                .completed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    /// Returns whether the task has reported completion.
    ///
    /// # Returns
    ///
    /// `true` after the task runner has produced or abandoned its final result.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.inner.done.load()
    }

    /// Attempts to cancel the task.
    ///
    /// Cancellation can only win before the runner marks the task as started.
    /// It cannot interrupt a task that is already running on an OS thread.
    ///
    /// # Returns
    ///
    /// `true` if the task was cancelled before it started, or `false` if the
    /// task was already running or completed.
    #[inline]
    pub fn cancel(&self) -> bool {
        TaskCompletion {
            inner: Arc::clone(&self.inner),
        }
        .cancel()
    }
}

impl<R, E> Future for TaskHandle<R, E> {
    type Output = TaskResult<R, E>;

    /// Polls this handle for the accepted task's final result.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.inner.lock_state();
        if state.completed {
            Poll::Ready(
                state
                    .result
                    .take()
                    .expect("task handle completed without a result"),
            )
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<R, E> TaskHandleInner<R, E> {
    /// Acquires the shared completion state while tolerating poisoned locks.
    fn lock_state(&self) -> MutexGuard<'_, TaskHandleState<R, E>> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<R, E> Clone for TaskCompletion<R, E> {
    /// Clones the completion endpoint for mutually exclusive finish paths.
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<R, E> TaskCompletion<R, E> {
    /// Marks the task as started if it was not cancelled first.
    ///
    /// # Returns
    ///
    /// `true` if the runner should execute the task, or `false` if the task was
    /// already completed through cancellation.
    pub(crate) fn start(&self) -> bool {
        let mut state = self.inner.lock_state();
        if state.completed {
            false
        } else {
            state.started = true;
            true
        }
    }

    /// Completes the task with its final result.
    ///
    /// If another path has already completed the task, this result is ignored.
    pub(crate) fn complete(&self, result: TaskResult<R, E>) {
        self.finish(result, |_| true);
    }

    /// Cancels the task if it has not started yet.
    ///
    /// # Returns
    ///
    /// `true` if this call published a cancellation result, or `false` if the
    /// task was already started or completed.
    pub(crate) fn cancel(&self) -> bool {
        self.finish(Err(TaskExecutionError::Cancelled), |state| !state.started)
    }

    /// Publishes a terminal result when the supplied predicate allows it.
    fn finish<F>(&self, result: TaskResult<R, E>, can_finish: F) -> bool
    where
        F: FnOnce(&TaskHandleState<R, E>) -> bool,
    {
        let mut state = self.inner.lock_state();
        if state.completed || !can_finish(&state) {
            return false;
        }
        state.result = Some(result);
        state.completed = true;
        self.inner.done.store(true);
        let waker = state.waker.take();
        drop(state);
        self.inner.completed.notify_all();
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }
}

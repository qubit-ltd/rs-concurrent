/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
/// Type-erased pool job with a cancellation path for queued work.
///
/// `PoolJob` is a low-level extension point for building custom services on
/// top of [`super::thread_pool::ThreadPool`]. The pool calls the run callback after a worker takes
/// the job, or the cancel callback if the job is still queued during immediate
/// shutdown.
pub struct PoolJob {
    /// Callback executed once a worker starts the job.
    run: Option<Box<dyn FnOnce() + Send + 'static>>,
    /// Callback executed if the job is cancelled before a worker starts it.
    cancel: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl PoolJob {
    /// Creates a pool job from run and cancel callbacks.
    ///
    /// # Parameters
    ///
    /// * `run` - Callback executed once a worker starts this job.
    /// * `cancel` - Callback executed if this job is cancelled while queued.
    ///
    /// # Returns
    ///
    /// A type-erased job accepted by [`super::thread_pool::ThreadPool::submit_job`].
    pub fn new(
        run: Box<dyn FnOnce() + Send + 'static>,
        cancel: Box<dyn FnOnce() + Send + 'static>,
    ) -> Self {
        Self {
            run: Some(run),
            cancel: Some(cancel),
        }
    }

    /// Runs this job if it has not been cancelled first.
    ///
    /// Consumes the job and invokes the run callback at most once.
    pub(crate) fn run(mut self) {
        if let Some(run) = self.run.take() {
            run();
        }
    }

    /// Cancels this queued job if it has not been run first.
    ///
    /// Consumes the job and invokes the cancellation callback at most once.
    pub(crate) fn cancel(mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel();
        }
    }
}

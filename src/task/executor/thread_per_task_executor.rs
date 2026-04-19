/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{
    sync::atomic::Ordering,
    thread,
};

use qubit_function::Callable;

use crate::task::{
    TaskHandle,
    task_runner::run_callable,
};

use super::Executor;

/// Executes each task on a dedicated OS thread.
///
/// This executor does not manage lifecycle or maintain a queue. Each accepted
/// task receives a blocking [`TaskHandle`] that can be used to wait for the
/// result.
#[derive(Debug, Default, Clone, Copy)]
pub struct ThreadPerTaskExecutor;

impl Executor for ThreadPerTaskExecutor {
    type Execution<R, E>
        = TaskHandle<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    /// Spawns one OS thread for the callable and returns a handle to its result.
    fn call<C, R, E>(&self, task: C) -> Self::Execution<R, E>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        let (handle, sender, done) = TaskHandle::channel();
        thread::spawn(move || {
            let result = run_callable(task);
            let _ = sender.send(result);
            done.store(true, Ordering::Release);
        });
        handle
    }
}

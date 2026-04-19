/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use qubit_function::Callable;

use super::{
    Executor,
    FutureExecutor,
    TokioExecution,
};

/// Executes callable tasks on Tokio's blocking task pool.
///
/// `TokioExecutor` is a [`FutureExecutor`]: its [`Executor::call`] and
/// [`Executor::execute`] methods return futures resolving to the task's own
/// `Result`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TokioExecutor;

impl Executor for TokioExecutor {
    type Execution<R, E>
        = TokioExecution<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    /// Spawns the callable on Tokio's blocking task pool.
    fn call<C, R, E>(&self, task: C) -> Self::Execution<R, E>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        TokioExecution::new(tokio::task::spawn_blocking(move || task.call()))
    }
}

impl FutureExecutor for TokioExecutor {}

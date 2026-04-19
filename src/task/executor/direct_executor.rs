/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use qubit_function::Callable;

use super::Executor;

/// Executes tasks immediately on the caller thread.
///
/// This executor is useful for deterministic tests and simple composition
/// where task execution should happen in the same call stack.
#[derive(Debug, Default, Clone, Copy)]
pub struct DirectExecutor;

impl Executor for DirectExecutor {
    type Execution<R, E>
        = Result<R, E>
    where
        R: Send + 'static,
        E: Send + 'static;

    /// Executes the callable inline and returns its result.
    #[inline]
    fn call<C, R, E>(&self, task: C) -> Self::Execution<R, E>
    where
        C: Callable<R, E> + Send + 'static,
        R: Send + 'static,
        E: Send + 'static,
    {
        task.call()
    }
}

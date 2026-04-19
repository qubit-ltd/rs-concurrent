/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::panic::{
    AssertUnwindSafe,
    catch_unwind,
};

use qubit_function::Callable;

use super::{
    TaskExecutionError,
    TaskResult,
};

/// Runs a callable and converts task failure and panic into a handle result.
///
/// # Parameters
///
/// * `task` - The callable to run.
///
/// # Returns
///
/// `Ok(R)` if the callable returns success, `Failed(E)` if the callable
/// returns `Err(E)`, or `Panicked` if the callable panics.
pub(crate) fn run_callable<C, R, E>(task: C) -> TaskResult<R, E>
where
    C: Callable<R, E>,
{
    match catch_unwind(AssertUnwindSafe(|| task.call())) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(TaskExecutionError::Failed(err)),
        Err(_) => Err(TaskExecutionError::Panicked),
    }
}

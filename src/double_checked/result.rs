/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Execution Result
//!
//! Provides result types for task execution outcomes.
//!
//! # Author
//!
//! Haixing Hu
use std::error::Error;

use qubit_function::{
    BoxSupplierOnce,
    SupplierOnce,
};

use crate::double_checked::error::ExecutorError;

/// Task execution result
///
/// Represents the result of executing a task using an enum to clearly distinguish
/// between success, unmet conditions, and failure.
///
/// # Type Parameters
///
/// * `T` - The type of the return value when execution succeeds
/// * `E` - The type of the error when execution fails
///
/// # Examples
///
/// ```rust,ignore
/// use qubit_concurrent::double_checked::{ExecutionResult, ExecutorError};
///
/// let success: ExecutionResult<i32, String> = ExecutionResult::Success(42);
/// if let ExecutionResult::Success(val) = success {
///     println!("Value: {}", val);
/// }
///
/// let unmet: ExecutionResult<i32, String> = ExecutionResult::ConditionNotMet;
///
/// let failed: ExecutionResult<i32, String> =
///     ExecutionResult::Failed(ExecutorError::TaskFailed("Task failed".to_string()));
/// ```
///
/// # Author
///
/// Haixing Hu
#[derive(Debug)]
pub enum ExecutionResult<T, E>
where
    E: std::fmt::Display,
{
    /// Execution succeeded with a value
    Success(T),

    /// Double-checked locking condition was not met
    ConditionNotMet,

    /// Execution failed with an error
    Failed(ExecutorError<E>),
}

impl<T, E> ExecutionResult<T, E>
where
    E: std::fmt::Display,
{
    /// Checks if the execution was successful
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success(_))
    }

    /// Checks if the condition was not met
    pub fn is_unmet(&self) -> bool {
        matches!(self, ExecutionResult::ConditionNotMet)
    }

    /// Checks if the execution failed
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionResult::Failed(_))
    }

    /// Unwraps the success value, panicking if not successful
    pub fn unwrap(self) -> T {
        match self {
            ExecutionResult::Success(v) => v,
            ExecutionResult::ConditionNotMet => {
                panic!("Called unwrap on ExecutionResult::ConditionNotMet")
            }
            ExecutionResult::Failed(e) => {
                panic!("Called unwrap on ExecutionResult::Failed: {}", e)
            }
        }
    }

    /// Converts the result to a standard Result
    ///
    /// # Returns
    ///
    /// * `Ok(Some(T))` - If execution was successful
    /// * `Ok(None)` - If condition was not met
    /// * `Err(ExecutorError<E>)` - If execution failed
    pub fn into_result(self) -> Result<Option<T>, ExecutorError<E>> {
        match self {
            ExecutionResult::Success(v) => Ok(Some(v)),
            ExecutionResult::ConditionNotMet => Ok(None),
            ExecutionResult::Failed(e) => Err(e),
        }
    }
}

/// Execution context (state after task execution)
///
/// This type provides rollback and result retrieval functionality after
/// task execution. It holds the execution status and optionally sets
/// rollback operations.
///
/// # Type Parameters
///
/// * `T` - The type of the task return value
/// * `E` - The type of the task error
///
/// # Author
///
/// Haixing Hu
pub struct ExecutionContext<T, E>
where
    E: std::fmt::Display,
{
    result: ExecutionResult<T, E>,
    rollback_action: Option<BoxSupplierOnce<Result<(), Box<dyn Error + Send + Sync>>>>,
}

impl<T, E> ExecutionContext<T, E>
where
    E: std::fmt::Display,
{
    /// Creates a new execution context
    ///
    /// # Arguments
    ///
    /// * `result` - The execution result
    pub(super) fn new(result: ExecutionResult<T, E>) -> Self {
        Self {
            result,
            rollback_action: None,
        }
    }

    /// Sets rollback action (optional, only executed on failure)
    ///
    /// # Arguments
    ///
    /// * `rollback_action` - Any type that implements
    ///   `SupplierOnce<Result<(), RE>>`
    ///
    /// # Note
    ///
    /// Rollback is only set and executed when `result` is `Failed`.
    pub fn rollback<S, RE>(mut self, rollback_action: S) -> Self
    where
        S: SupplierOnce<Result<(), RE>> + 'static,
        RE: Error + Send + Sync + 'static,
    {
        if let ExecutionResult::Failed(_) = self.result {
            let boxed = rollback_action.into_box();
            self.rollback_action = Some(BoxSupplierOnce::new(move || {
                boxed
                    .get()
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
            }));
        }
        self
    }

    /// Gets the execution result (consumes the context)
    ///
    /// If rollback is set and execution failed, rollback will be executed
    /// before returning the result.
    ///
    /// If rollback execution fails, the error in the returned result will be
    /// updated to `RollbackFailed`.
    pub fn get_result(mut self) -> ExecutionResult<T, E> {
        if let ExecutionResult::Failed(ref mut original_error) = self.result {
            if let Some(rollback_action) = self.rollback_action.take() {
                if let Err(rollback_error) = rollback_action.get() {
                    log::error!("Rollback action failed: {}", rollback_error);
                    // Update the error to RollbackFailed
                    *original_error = ExecutorError::RollbackFailed {
                        original: original_error.to_string(),
                        rollback: rollback_error.to_string(),
                    };
                }
            }
        }
        self.result
    }

    /// Checks the execution result (does not consume the context)
    pub fn peek_result(&self) -> &ExecutionResult<T, E> {
        &self.result
    }

    /// Checks if execution was successful
    pub fn is_success(&self) -> bool {
        self.result.is_success()
    }
}

// Convenience methods for cases without return values
impl<E> ExecutionContext<(), E>
where
    E: std::fmt::Display,
{
    /// Completes execution (for operations without return values)
    ///
    /// Returns whether the execution was successful
    pub fn finish(self) -> bool {
        let result = self.get_result();
        result.is_success()
    }
}

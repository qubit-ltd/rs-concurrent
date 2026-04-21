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
//! Provides the task execution result enum for double-checked locking.
//!
//! # Author
//!
//! Haixing Hu
use std::fmt;

use crate::double_checked::executor_error::ExecutorError;

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
/// ```rust
/// use qubit_concurrent::double_checked::{ExecutionResult, ExecutorError};
///
/// let success: ExecutionResult<i32, String> = ExecutionResult::success(42);
/// if let ExecutionResult::Success(val) = success {
///     println!("Value: {}", val);
/// }
///
/// let unmet: ExecutionResult<i32, String> = ExecutionResult::unmet();
///
/// let failed: ExecutionResult<i32, String> =
///     ExecutionResult::task_failed("Task failed".to_string());
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
    /// Builds [`ExecutionResult::Success`] with `value`.
    #[inline]
    pub fn success(value: T) -> Self {
        ExecutionResult::Success(value)
    }

    /// Builds [`ExecutionResult::ConditionNotMet`].
    #[inline]
    pub fn unmet() -> Self {
        ExecutionResult::ConditionNotMet
    }

    /// Builds a failed result with [`ExecutorError::TaskFailed`].
    #[inline]
    pub fn task_failed(err: E) -> Self {
        ExecutionResult::Failed(ExecutorError::TaskFailed(err))
    }

    /// Builds a failed result with [`ExecutorError::PrepareFailed`].
    ///
    /// Accepts any [`fmt::Display`] value (including [`std::error::Error`] and [`String`]);
    /// the message is stored as a [`String`] via [`ToString`].
    #[inline]
    pub fn prepare_failed(msg: impl fmt::Display) -> Self {
        ExecutionResult::Failed(ExecutorError::PrepareFailed(msg.to_string()))
    }

    /// Builds a failed result with [`ExecutorError::PrepareCommitFailed`].
    #[inline]
    pub fn prepare_commit_failed(msg: impl fmt::Display) -> Self {
        ExecutionResult::Failed(ExecutorError::PrepareCommitFailed(msg.to_string()))
    }

    /// Builds a failed result with [`ExecutorError::PrepareRollbackFailed`].
    #[inline]
    pub fn prepare_rollback_failed(
        original: impl Into<String>,
        rollback: impl Into<String>,
    ) -> Self {
        ExecutionResult::Failed(ExecutorError::PrepareRollbackFailed {
            original: original.into(),
            rollback: rollback.into(),
        })
    }

    /// Builds a failed result with [`ExecutorError::LockPoisoned`].
    #[inline]
    pub fn lock_poisoned(msg: impl Into<String>) -> Self {
        ExecutionResult::Failed(ExecutorError::LockPoisoned(msg.into()))
    }

    /// Wraps an arbitrary [`ExecutorError`] as [`ExecutionResult::Failed`].
    #[inline]
    pub fn from_executor_error(err: ExecutorError<E>) -> Self {
        ExecutionResult::Failed(err)
    }

    /// Checks if the execution was successful
    #[inline]
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success(_))
    }

    /// Checks if the condition was not met
    #[inline]
    pub fn is_unmet(&self) -> bool {
        matches!(self, ExecutionResult::ConditionNotMet)
    }

    /// Checks if the execution failed
    #[inline]
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionResult::Failed(_))
    }

    /// Unwraps the success value, panicking if not successful
    #[inline]
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
    #[inline]
    pub fn into_result(self) -> Result<Option<T>, ExecutorError<E>> {
        match self {
            ExecutionResult::Success(v) => Ok(Some(v)),
            ExecutionResult::ConditionNotMet => Ok(None),
            ExecutionResult::Failed(e) => Err(e),
        }
    }
}

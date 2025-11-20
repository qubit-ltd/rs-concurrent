/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Error Types
//!
//! Provides error types for the double-checked lock executor.
//!
//! # Author
//!
//! Haixing Hu
use thiserror::Error;

/// Executor error types
///
/// Defines various error conditions that can occur during executor
/// operation, including condition failures, task execution errors,
/// and rollback failures.
///
/// # Type Parameters
///
/// * `E` - The original error type from task execution
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_rust_concurrent::double_checked::ExecutorError;
///
/// let error: ExecutorError<String> =
///     ExecutorError::ConditionNotMet;
/// println!("Error: {}", error);
///
/// let error_with_msg: ExecutorError<String> =
///     ExecutorError::ConditionNotMetWithMessage(
///         "Service is not running".to_string()
///     );
/// println!("Error: {}", error_with_msg);
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug)]
pub enum ExecutorError<E>
where
    E: std::fmt::Display,
{
    /// Task execution failed with original error
    TaskFailed(E),

    /// Preparation action failed
    PrepareFailed(String),

    /// Rollback operation failed
    RollbackFailed {
        /// The original error that triggered the rollback
        original: String,
        /// The error that occurred during rollback
        rollback: String,
    },

    /// Lock poisoned error
    LockPoisoned(String),
}

impl<E> std::fmt::Display for ExecutorError<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutorError::TaskFailed(e) => {
                write!(f, "Task execution failed: {}", e)
            }
            ExecutorError::PrepareFailed(msg) => {
                write!(f, "Preparation action failed: {}", msg)
            }
            ExecutorError::RollbackFailed { original, rollback } => {
                write!(
                    f,
                    "Rollback failed: original error = {}, rollback error = {}",
                    original, rollback
                )
            }
            ExecutorError::LockPoisoned(msg) => {
                write!(f, "Lock poisoned: {}", msg)
            }
        }
    }
}

impl<E> std::error::Error for ExecutorError<E> where E: std::fmt::Display + std::fmt::Debug {}


/// Builder error types
///
/// Defines error conditions that can occur during executor builder
/// construction, such as missing required parameters.
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_rust_concurrent::double_checked::BuilderError;
///
/// let error = BuilderError::MissingTester;
/// println!("Builder error: {}", error);
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Error)]
pub enum BuilderError {
    /// Missing required tester parameter
    #[error("Tester function is required")]
    MissingTester,
}

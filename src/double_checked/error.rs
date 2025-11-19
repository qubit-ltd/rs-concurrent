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
    /// Condition not met error
    ConditionNotMet,

    /// Condition not met with custom message
    ConditionNotMetWithMessage(String),

    /// Task execution failed with original error
    TaskFailed(E),

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
            ExecutorError::ConditionNotMet => {
                write!(f, "Double-checked lock condition not met")
            }
            ExecutorError::ConditionNotMetWithMessage(msg) => {
                write!(f, "Double-checked lock condition not met: {}", msg)
            }
            ExecutorError::TaskFailed(e) => {
                write!(f, "Task execution failed: {}", e)
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

/// Execution result type alias
///
/// A convenient type alias for `Result<R, ExecutorError<E>>`, used as
/// the return type for executor methods.
///
/// # Type Parameters
///
/// * `R` - The success value type
/// * `E` - The task error type
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_rust_concurrent::double_checked::ExecutionResult;
///
/// fn process_data() -> ExecutionResult<i32, String> {
///     Ok(42)
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub type ExecutionResult<R, E> = Result<R, ExecutorError<E>>;

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

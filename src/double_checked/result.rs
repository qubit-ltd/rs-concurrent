/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
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

/// Task execution result
///
/// Represents the result of executing a task, including success status,
/// return value, and error information. Similar to Java's `Result<T>` class
/// but renamed to avoid confusion with Rust's standard `Result` type.
///
/// # Type Parameters
///
/// * `T` - The type of the return value when execution succeeds
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_rust_concurrent::double_checked::ExecutionResult;
///
/// let result = ExecutionResult::succeed(42);
/// if result.success {
///     println!("Value: {}", result.value.unwrap());
/// }
///
/// let failed = ExecutionResult::fail("Task failed");
/// if !failed.success {
///     println!("Error: {:?}", failed.error);
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct ExecutionResult<T> {
    /// Whether the execution was successful
    pub success: bool,

    /// The return value when execution succeeds (only present when success = true)
    pub value: Option<T>,

    /// The error information when execution fails (only present when success = false)
    pub error: Option<Box<dyn Error + Send + Sync>>,
}

impl<T> ExecutionResult<T> {
    /// Creates a successful execution result
    ///
    /// # Arguments
    ///
    /// * `value` - The return value of the successful execution
    ///
    /// # Returns
    ///
    /// Returns a new `ExecutionResult` with success = true and the given value
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::double_checked::ExecutionResult;
    ///
    /// let result = ExecutionResult::succeed(42);
    /// assert!(result.success);
    /// assert_eq!(result.value, Some(42));
    /// ```
    pub fn succeed(value: T) -> Self {
        Self {
            success: true,
            value: Some(value),
            error: None,
        }
    }

    /// Creates an execution result for unmet conditions
    ///
    /// This method is used when the double-checked lock condition is not met.
    /// It represents a normal execution path where the task should not be
    /// executed, rather than an error condition.
    ///
    /// # Returns
    ///
    /// Returns a new `ExecutionResult` with success = false and no error message
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::double_checked::ExecutionResult;
    ///
    /// let result = ExecutionResult::unmet();
    /// assert!(!result.success);
    /// assert!(result.value.is_none());
    /// assert!(result.error.is_none());
    /// ```
    pub fn unmet() -> Self {
        Self {
            success: false,
            value: None,
            error: None,
        }
    }

    /// Creates a failed execution result with error information
    ///
    /// # Arguments
    ///
    /// * `error` - The error that caused the execution to fail
    ///
    /// # Returns
    ///
    /// Returns a new `ExecutionResult` with success = false and the given error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::double_checked::ExecutionResult;
    ///
    /// let result = ExecutionResult::fail("Task failed");
    /// assert!(!result.success);
    /// assert!(result.value.is_none());
    /// assert!(result.error.is_some());
    /// ```
    pub fn fail<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            success: false,
            value: None,
            error: Some(Box::new(error)),
        }
    }

    /// Creates a failed execution result from a boxed error
    pub fn fail_with_box(error: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            success: false,
            value: None,
            error: Some(error),
        }
    }

    /// Converts the execution result to a standard Result
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - If execution was successful, returns the value
    /// * `Err(Box<dyn Error + Send + Sync>)` - If execution failed, returns the error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::double_checked::ExecutionResult;
    ///
    /// let result = ExecutionResult::succeed(42);
    /// match result.into_result() {
    ///     Ok(value) => println!("Success: {}", value),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn into_result(self) -> Result<T, Box<dyn Error + Send + Sync>> {
        if self.success {
            Ok(self.value.unwrap())
        } else {
            Err(self.error.unwrap_or_else(|| "Unknown error".into()))
        }
    }
}

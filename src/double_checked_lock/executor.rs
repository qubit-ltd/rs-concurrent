/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Lock Executor
//!
//! Provides a double-checked lock executor for executing tasks with condition
//! checking and rollback support.
//!
//! # Author
//!
//! Haixing Hu

use prism3_function::{ArcTester, Tester};

use crate::lock::Lock;
use super::{
    BuilderError, ExecutionResult, ExecutorError, LogConfig,
};

/// Double-checked lock executor
///
/// Executes tasks using the double-checked locking pattern to minimize
/// lock contention while ensuring thread safety. The executor checks
/// conditions both before and after acquiring locks to optimize
/// performance in scenarios where conditions are frequently met.
///
/// # Features
///
/// - Double-checked locking: Fast path when conditions are met
/// - Condition testing: Configurable condition checking
/// - Error handling: Flexible error reporting and logging
/// - Rollback support: Automatic rollback on failure
/// - Multiple lock types: Support for both mutex and read-write locks
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use prism3_rust_concurrent::{
///     DoubleCheckedLockExecutor,
///     ArcMutex,
///     Lock,
/// };
///
/// let running = Arc::new(AtomicBool::new(true));
/// let data = ArcMutex::new(42);
///
/// let executor = DoubleCheckedLockExecutor::builder()
///     .tester_fn(move || running.load(Ordering::Acquire))
///     .build()
///     .unwrap();
///
/// let result = executor.call_mut(&data, |value| {
///     *value += 1;
///     Ok::<_, String>(*value)
/// });
///
/// match result {
///     Ok(value) => println!("Result: {}", value),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct DoubleCheckedLockExecutor {
    /// Condition tester - tests whether execution conditions are met
    /// Note: The shared state that this tester depends on must be
    /// thread-safe (e.g., Arc<AtomicBool>, Arc<Mutex<T>>) to ensure
    /// visibility across threads
    tester: ArcTester,

    /// Log configuration (optional)
    logger: Option<LogConfig>,
}

impl DoubleCheckedLockExecutor {
    /// Creates a new builder for the executor
    ///
    /// # Returns
    ///
    /// Returns a new builder instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::DoubleCheckedLockExecutor;
    ///
    /// let builder = DoubleCheckedLockExecutor::builder();
    /// ```
    pub fn builder() -> DoubleCheckedLockExecutorBuilder {
        DoubleCheckedLockExecutorBuilder::default()
    }

    /// Executes a task with no return value using a mutex lock
    ///
    /// # Type Parameters
    ///
    /// * `L` - Lock type that implements the `Lock<T>` trait
    /// * `T` - Type of the data protected by the lock
    /// * `F` - Task closure type
    /// * `E` - Task error type
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to a lock that implements the `Lock<T>`
    ///            trait
    /// * `task` - Task to execute, returns `Result<(), E>`
    ///
    /// # Returns
    ///
    /// Returns `ExecutionResult<(), E>` indicating whether execution
    /// was successful
    ///
    /// # Execution Flow
    ///
    /// 1. First condition check (fast fail outside lock)
    /// 2. Acquire lock
    /// 3. Second condition check (confirm inside lock)
    /// 4. Execute task
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::{
    ///     DoubleCheckedLockExecutor,
    ///     lock::{ArcMutex, Lock},
    /// };
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let data = ArcMutex::new(42);
    /// let result = executor.execute(&data, |value| {
    ///     *value += 1;
    ///     Ok::<_, String>(())
    /// });
    ///
    /// assert!(result.is_ok());
    /// ```
    pub fn execute<L, T, F, E>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<(), E>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<(), E>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        self.call_mut(lock, task)
    }

    /// Executes a task with a return value using a write lock
    ///
    /// This method uses the write lock of the provided lock, which
    /// provides exclusive access to the protected data. For exclusive
    /// locks (Mutex), this is the same as the read lock. For
    /// read-write locks (RwLock), this provides exclusive access that
    /// blocks all other operations.
    ///
    /// # Type Parameters
    ///
    /// * `L` - Lock type that implements the `Lock<T>` trait
    /// * `T` - Type of the data protected by the lock
    /// * `F` - Task closure type
    /// * `R` - Task return value type
    /// * `E` - Task error type
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to a lock that implements the `Lock<T>`
    ///            trait
    /// * `task` - Task to execute, returns `Result<R, E>`
    ///
    /// # Returns
    ///
    /// Returns `ExecutionResult<R, E>` containing the task's return
    /// value or an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::{
    ///     DoubleCheckedLockExecutor,
    ///     lock::{ArcMutex, Lock},
    /// };
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let data = ArcMutex::new(42);
    /// let result = executor.call_mut(&data, |value| {
    ///     *value += 1;
    ///     Ok::<_, String>(*value)
    /// });
    ///
    /// match result {
    ///     Ok(value) => println!("New value: {}", value),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn call_mut<L, T, F, R, E>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<R, E>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<R, E>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        // First check: fast fail outside lock
        if !self.tester.test() {
            self.handle_condition_not_met();
            return Err(ExecutorError::ConditionNotMet);
        }
        // Use Lock trait's write method to acquire lock and execute
        lock.write(|data| {
            // Second check: confirm again inside lock
            if !self.tester.test() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }
            // Execute task
            task(data).map_err(ExecutorError::TaskFailed)
        })
    }

    /// Executes a task with a return value using a read lock
    ///
    /// This method uses the read lock of the provided lock, which
    /// provides immutable access to the protected data. For exclusive
    /// locks (Mutex), this is the same as the write lock. For
    /// read-write locks (RwLock), this allows concurrent readers.
    ///
    /// # Type Parameters
    ///
    /// * `L` - Lock type that implements the `Lock<T>` trait
    /// * `T` - Type of the data protected by the lock
    /// * `F` - Task closure type
    /// * `R` - Task return value type
    /// * `E` - Task error type
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to a lock that implements the `Lock<T>`
    ///            trait
    /// * `task` - Task to execute, returns `Result<R, E>`
    ///
    /// # Returns
    ///
    /// Returns `ExecutionResult<R, E>` containing the task's return
    /// value or an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::{
    ///     DoubleCheckedLockExecutor,
    ///     lock::{ArcRwLock, Lock},
    /// };
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let data = ArcRwLock::new(42);
    /// let result = executor.call(&data, |value| {
    ///     Ok::<_, String>(*value)
    /// });
    ///
    /// match result {
    ///     Ok(value) => println!("Value: {}", value),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn call<L, T, F, R, E>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<R, E>
    where
        L: Lock<T>,
        F: FnOnce(&T) -> Result<R, E>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        // First check: fast fail outside lock
        if !self.tester.test() {
            self.handle_condition_not_met();
            return Err(ExecutorError::ConditionNotMet);
        }
        // Use Lock trait's read method to acquire lock and execute
        lock.read(|data| {
            // Second check: confirm again inside lock
            if !self.tester.test() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }
            // Execute task
            task(data).map_err(ExecutorError::TaskFailed)
        })
    }


    /// Executes a task with rollback mechanism using a read lock
    ///
    /// # Execution Flow
    ///
    /// 1. Check if conditions are met, fail if not
    /// 2. If conditions are met, execute `outside_action` first
    /// 3. Then acquire lock and check conditions again:
    ///    - If not met, release lock and execute `rollback_action`,
    ///      then fail
    ///    - If met, execute `task`
    /// 4. If `task` throws an exception, release lock and execute
    ///    `rollback_action`
    ///
    /// # Type Parameters
    ///
    /// * `L` - Lock type that implements the `Lock<T>` trait
    /// * `T` - Type of the data protected by the lock
    /// * `F` - Task closure type
    /// * `O` - Outside action closure type
    /// * `Rb` - Rollback action closure type
    /// * `V` - Task return value type
    /// * `E` - Task error type
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to a lock that implements the `Lock<T>`
    ///            trait
    /// * `task` - Core task to execute (read-only access)
    /// * `outside_action` - Preparation action outside the lock
    /// * `rollback_action` - Rollback action on failure
    ///
    /// # Deadlock Warning
    ///
    /// `outside_action` is executed **before** acquiring the lock, so
    /// it is **forbidden** to try to acquire the same lock or any
    /// other locks that might form a lock cycle in this action,
    /// otherwise it will cause deadlock!
    ///
    /// # Returns
    ///
    /// Returns `ExecutionResult<V, E>` containing the task's return
    /// value or an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::{
    ///     DoubleCheckedLockExecutor,
    ///     lock::{ArcRwLock, Lock},
    /// };
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let data = ArcRwLock::new(42);
    /// let result = executor.call_with_rollback(
    ///     &data,
    ///     |value| {
    ///         Ok::<_, String>(*value)
    ///     },
    ///     || {
    ///         println!("Preparing...");
    ///         Ok::<_, String>(())
    ///     },
    ///     || {
    ///         println!("Rolling back...");
    ///         Ok::<_, String>(())
    ///     },
    /// );
    ///
    /// match result {
    ///     Ok(value) => println!("Result: {}", value),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn call_with_rollback<L, T, F, O, Rb, V, E>(
        &self,
        lock: &L,
        task: F,
        outside_action: O,
        rollback_action: Rb,
    ) -> ExecutionResult<V, E>
    where
        L: Lock<T>,
        F: FnOnce(&T) -> Result<V, E>,
        O: FnOnce() -> Result<(), E>,
        Rb: FnOnce() -> Result<(), E>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        // First check
        if !self.tester.test() {
            self.handle_condition_not_met();
            return Err(ExecutorError::ConditionNotMet);
        }
        // Execute outside action
        if let Err(e) = outside_action() {
            log::error!("Outside action failed: {}", e);
            return Err(ExecutorError::TaskFailed(e));
        }
        // Use Lock trait's read method
        // Note: Cannot handle rollback directly inside closure, need
        // special handling
        let result = lock.read(|data| {
            // Second check
            if !self.tester.test() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }
            // Execute task
            task(data).map_err(ExecutorError::TaskFailed)
        });
        // Handle result and rollback
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                let error_msg = e.to_string();
                self.run_rollback(rollback_action, Some(&error_msg));
                Err(e)
            }
        }
    }

    /// Executes a task with rollback mechanism using a read-write
    /// lock's write lock
    ///
    /// # Type Parameters
    ///
    /// * `L` - Lock type that implements the `ReadWriteLock<T>` trait
    /// * `T` - Type of the data protected by the lock
    /// * `F` - Task closure type
    /// * `O` - Outside action closure type
    /// * `Rb` - Rollback action closure type
    /// * `V` - Task return value type
    /// * `E` - Task error type
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to a lock that implements the
    ///            `ReadWriteLock<T>` trait
    /// * `task` - Core task to execute
    /// * `outside_action` - Preparation action outside the lock
    /// * `rollback_action` - Rollback action on failure
    ///
    /// # Returns
    ///
    /// Returns `ExecutionResult<V, E>` containing the task's return
    /// value or an error
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::{
    ///     DoubleCheckedLockExecutor,
    ///     lock::{ArcRwLock, ReadWriteLock},
    /// };
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()
    ///     .unwrap();
    ///
    /// let data = ArcRwLock::new(42);
    /// let result = executor.call_with_rollback_mut(
    ///     &data,
    ///     |value| {
    ///         *value += 1;
    ///         Ok::<_, String>(*value)
    ///     },
    ///     || {
    ///         println!("Preparing...");
    ///         Ok::<_, String>(())
    ///     },
    ///     || {
    ///         println!("Rolling back...");
    ///         Ok::<_, String>(())
    ///     },
    /// );
    ///
    /// match result {
    ///     Ok(value) => println!("Result: {}", value),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    pub fn call_with_rollback_mut<L, T, F, O, Rb, V, E>(
        &self,
        lock: &L,
        task: F,
        outside_action: O,
        rollback_action: Rb,
    ) -> ExecutionResult<V, E>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<V, E>,
        O: FnOnce() -> Result<(), E>,
        Rb: FnOnce() -> Result<(), E>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        // First check
        if !self.tester.test() {
            self.handle_condition_not_met();
            return Err(ExecutorError::ConditionNotMet);
        }
        // Execute outside action
        if let Err(e) = outside_action() {
            log::error!("Outside action failed: {}", e);
            return Err(ExecutorError::TaskFailed(e));
        }
        // Use ReadWriteLock trait's write method
        let result = lock.write(|data| {
            // Second check
            if !self.tester.test() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }
            // Execute task
            task(data).map_err(ExecutorError::TaskFailed)
        });
        // Handle result and rollback
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                let error_msg = e.to_string();
                self.run_rollback(rollback_action, Some(&error_msg));
                Err(e)
            }
        }
    }

    /// Handles the case when conditions are not met
    fn handle_condition_not_met(&self) {
        // Log if configured
        if let Some(ref log_config) = self.logger {
            log::log!(log_config.level, "{}", log_config.message);
        }

        // Note: In Rust, we cannot directly throw exceptions like in Java.
        // Error information is passed through the return value.
        // In actual implementation, the result of `error_supplier` can be
        // stored in ExecutionResult.
    }

    /// Executes rollback operation
    fn run_rollback<R, E>(
        &self,
        rollback_action: R,
        original_error: Option<&str>,
    ) where
        R: FnOnce() -> Result<(), E>,
        E: std::fmt::Display,
    {
        if let Err(e) = rollback_action() {
            if let Some(original) = original_error {
                log::warn!(
                    "Rollback failed during error recovery: {}. \
                     Original error: {}",
                    e,
                    original
                );
            } else {
                log::error!("Rollback failed: {}", e);
            }
        }
    }
}

/// Builder for DoubleCheckedLockExecutor
///
/// Provides a fluent API for constructing `DoubleCheckedLockExecutor`
/// instances with various configuration options.
///
/// # Examples
///
/// ```rust,ignore
/// use prism3_rust_concurrent::DoubleCheckedLockExecutor;
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
///
/// let running = Arc::new(AtomicBool::new(true));
/// let running_clone = running.clone();
///
/// let executor = DoubleCheckedLockExecutor::builder()
///     .tester_fn(move || running_clone.load(Ordering::Acquire))
///     .logger(log::Level::Error, "Service is not running")
///     .error_supplier(|| MyError::NotRunning)
///     .build()
///     .unwrap();
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct DoubleCheckedLockExecutorBuilder {
    tester: Option<ArcTester>,
    logger: Option<LogConfig>,
}

impl Default for DoubleCheckedLockExecutorBuilder {
    /// Creates a default builder
    ///
    /// # Returns
    ///
    /// Returns a new builder with default configuration
    fn default() -> Self {
        Self {
            tester: None,
            logger: None,
        }
    }
}

impl DoubleCheckedLockExecutorBuilder {
    /// Sets the condition tester (required)
    ///
    /// Accepts an `ArcTester` instance for testing whether execution
    /// conditions are met.
    ///
    /// **Important**: The shared state that the tester depends on must
    /// be thread-safe (e.g., `Arc<AtomicBool>`, `Arc<Mutex<T>>`) to ensure
    /// visibility across threads.
    ///
    /// # Arguments
    ///
    /// * `tester` - Condition tester
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_function::ArcTester;
    /// use std::sync::{Arc, RwLock};
    ///
    /// let state = Arc::new(RwLock::new(State::Running));
    /// let state_clone = state.clone();
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester(ArcTester::new(move || {
    ///         matches!(*state_clone.read().unwrap(), State::Running)
    ///     }))
    ///     .build()?;
    /// ```
    pub fn tester(mut self, tester: ArcTester) -> Self {
        self.tester = Some(tester);
        self
    }

    /// Sets the condition tester function (convenience method)
    ///
    /// Accepts a closure and internally creates an `ArcTester`.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure for testing conditions
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    ///
    /// let running = Arc::new(AtomicBool::new(true));
    /// let running_clone = running.clone();
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(move || running_clone.load(Ordering::Acquire))
    ///     .build()?;
    /// ```
    pub fn tester_fn<F>(mut self, f: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.tester = Some(ArcTester::new(f));
        self
    }

    /// Sets the logger (optional)
    ///
    /// # Arguments
    ///
    /// * `level` - Log level
    /// * `message` - Log message
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .logger(log::Level::Error, "Service is not running")
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn logger(mut self, level: log::Level, message: impl Into<String>) -> Self {
        self.logger = Some(LogConfig {
            level,
            message: message.into(),
        });
        self
    }

    /// Builds the executor
    ///
    /// # Returns
    ///
    /// * `Ok(DoubleCheckedLockExecutor)` - If all required parameters
    ///   are set
    /// * `Err(BuilderError)` - If required parameters are missing
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(|| true)
    ///     .build()?;
    /// ```
    pub fn build(self) -> Result<DoubleCheckedLockExecutor, BuilderError> {
        let tester = self.tester.ok_or(BuilderError::MissingTester)?;

        Ok(DoubleCheckedLockExecutor {
            tester,
            logger: self.logger,
        })
    }
}

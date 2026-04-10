/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Locking Execution Builder
//!
//! Provides a fluent API builder using the typestate pattern.
//!
//! # Author
//!
//! Haixing Hu
use std::{
    error::Error,
    marker::PhantomData,
};

use qubit_function::{
    BoxFunctionOnce,
    BoxMutatingFunctionOnce,
    BoxSupplierOnce,
    BoxTester,
    FunctionOnce,
    MutatingFunctionOnce,
    SupplierOnce,
    Tester,
};

use super::{
    states::{
        Conditioned,
        Configuring,
        Initial,
    },
    ExecutionContext,
    ExecutionResult,
    LogConfig,
};
use crate::{
    double_checked::error::ExecutorError,
    lock::Lock,
};

/// Execution builder (using typestate pattern)
///
/// This builder uses the type system to enforce the correct call sequence
/// at compile time.
///
/// # Type Parameters
///
/// * `'a` - Lifetime of the lock
/// * `L` - Lock type (implements the Lock<T> trait)
/// * `T` - Type of data protected by the lock
/// * `State` - Current state (Initial, Configuring, Conditioned)
///
/// # Author
///
/// Haixing Hu
pub struct ExecutionBuilder<'a, L, T, State = Initial>
where
    L: Lock<T>,
{
    /// Reference to the lock that protects the shared data
    lock: &'a L,

    /// Optional logging configuration for execution events
    logger: Option<LogConfig>,

    /// Optional test condition that determines if execution should proceed
    tester: Option<BoxTester>,

    /// Optional preparation action executed between first check and locking
    prepare_action: Option<BoxSupplierOnce<Result<(), Box<dyn Error + Send + Sync>>>>,

    /// Phantom data for typestate pattern, tracks current builder state
    _phantom: PhantomData<(T, State)>,
}

/// Implementation for the `Initial` state of `ExecutionBuilder`.
///
/// In this state, the builder has just been created and allows:
/// - Configuring optional logging via `logger()`
/// - Setting the required test condition via `when()`
///
/// This is the starting state where users begin building their execution.
impl<'a, L, T> ExecutionBuilder<'a, L, T, Initial>
where
    L: Lock<T>,
{
    /// Creates a new execution builder
    ///
    /// # Arguments
    ///
    /// * `lock` - Reference to the lock object
    #[inline]
    pub(super) fn new(lock: &'a L) -> Self {
        Self {
            lock,
            logger: None,
            tester: None,
            prepare_action: None,
            _phantom: PhantomData,
        }
    }

    /// Configures logging (optional)
    ///
    /// # State Transition
    ///
    /// Initial → Configuring
    ///
    /// # Arguments
    ///
    /// * `level` - Log level
    /// * `message` - Log message
    #[inline]
    pub fn logger(
        mut self,
        level: log::Level,
        message: &str,
    ) -> ExecutionBuilder<'a, L, T, Configuring> {
        self.logger = Some(LogConfig {
            level,
            message: message.to_string(),
        });
        ExecutionBuilder {
            lock: self.lock,
            logger: self.logger,
            tester: self.tester,
            prepare_action: self.prepare_action,
            _phantom: PhantomData,
        }
    }

    /// Sets the test condition (required)
    ///
    /// # Safety Warning
    ///
    /// The `tester` closure is executed twice: first without the lock (fast
    /// path) and then with the lock held (slow path).
    ///
    /// For the first check (fast path) to be thread-safe, the `tester` closure
    /// MUST access shared state using atomic operations with appropriate memory
    /// ordering (e.g., `Ordering::SeqCst` or `Ordering::Acquire`). Relying on
    /// non-atomic shared state without locking leads to data races and
    /// undefined behavior.
    ///
    /// # State Transition
    ///
    /// Initial → Conditioned
    ///
    /// # Arguments
    ///
    /// * `tester` - The test condition
    #[inline]
    pub fn when<Tst>(mut self, tester: Tst) -> ExecutionBuilder<'a, L, T, Conditioned>
    where
        Tst: Tester + 'static,
    {
        self.tester = Some(tester.into_box());
        ExecutionBuilder {
            lock: self.lock,
            logger: self.logger,
            tester: self.tester,
            prepare_action: self.prepare_action,
            _phantom: PhantomData,
        }
    }
}

/// Implementation for the `Configuring` state of `ExecutionBuilder`.
///
/// In this state, logging has been configured and the builder allows:
/// - Overriding the logging configuration via `logger()`
/// - Setting the required test condition via `when()`
///
/// Users can stay in this state to adjust logging settings or transition
/// to the `Conditioned` state by setting a test condition.
impl<'a, L, T> ExecutionBuilder<'a, L, T, Configuring>
where
    L: Lock<T>,
{
    /// Continues configuring logging (can override previous configuration)
    ///
    /// # State Transition
    ///
    /// Configuring → Configuring
    ///
    /// # Arguments
    ///
    /// * `level` - Log level
    /// * `message` - Log message
    #[inline]
    pub fn logger(mut self, level: log::Level, message: &str) -> Self {
        self.logger = Some(LogConfig {
            level,
            message: message.to_string(),
        });
        self
    }

    /// Sets the test condition (required)
    ///
    /// # Safety Warning
    ///
    /// The `tester` closure is executed twice: first without the lock (fast
    /// path) and then with the lock held (slow path).
    ///
    /// For the first check (fast path) to be thread-safe, the `tester` closure
    /// MUST access shared state using atomic operations with appropriate memory
    /// ordering (e.g., `Ordering::SeqCst` or `Ordering::Acquire`). Relying on
    /// non-atomic shared state without locking leads to data races and
    /// undefined behavior.
    ///
    /// # State Transition
    ///
    /// Configuring → Conditioned
    ///
    /// # Arguments
    ///
    /// * `tester` - The test condition
    #[inline]
    pub fn when<Tst>(mut self, tester: Tst) -> ExecutionBuilder<'a, L, T, Conditioned>
    where
        Tst: Tester + 'static,
    {
        self.tester = Some(tester.into_box());
        ExecutionBuilder {
            lock: self.lock,
            logger: self.logger,
            tester: self.tester,
            prepare_action: self.prepare_action,
            _phantom: PhantomData,
        }
    }
}

/// Implementation for the `Conditioned` state of `ExecutionBuilder`.
///
/// In this state, the test condition has been set and the builder allows:
/// - Setting an optional prepare action via `prepare()`
/// - Executing read-only tasks with return values via `call()`
/// - Executing read-write tasks with return values via `call_mut()`
/// - Executing read-only tasks without return values via `execute()`
/// - Executing read-write tasks without return values via `execute_mut()`
///
/// This is the final state where users can configure preparation steps
/// and execute their tasks with double-checked locking semantics.
impl<'a, L, T> ExecutionBuilder<'a, L, T, Conditioned>
where
    L: Lock<T>,
    T: 'static,
{
    /// Sets prepare action (optional, executed between first check and locking)
    ///
    /// # State Transition
    ///
    /// Conditioned → Conditioned
    ///
    /// # Arguments
    ///
    /// * `prepare_action` - Any type that implements
    ///   `SupplierOnce<Result<(), E>>`
    #[inline]
    pub fn prepare<S, E>(mut self, prepare_action: S) -> Self
    where
        S: SupplierOnce<Result<(), E>> + 'static,
        E: Error + Send + Sync + 'static,
    {
        let boxed = prepare_action.into_box();
        self.prepare_action = Some(BoxSupplierOnce::new(move || {
            boxed
                .get()
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        }));
        self
    }

    /// Executes a read-only task (with return value)
    ///
    /// # Execution Flow
    ///
    /// 1. First condition check (outside lock)
    /// 2. Execute prepare action (if any)
    /// 3. Acquire lock
    /// 4. Second condition check (inside lock)
    /// 5. Execute task
    ///
    /// # State Transition
    ///
    /// Conditioned → ExecutionContext<R>
    ///
    /// # Arguments
    ///
    /// * `task` - Any type that implements `FunctionOnce<T, Result<R, E>>`
    #[inline]
    pub fn call<F, R, E>(self, task: F) -> ExecutionContext<R, E>
    where
        F: FunctionOnce<T, Result<R, E>> + 'static,
        E: Error + Send + Sync + 'static,
        R: 'static,
    {
        let task_boxed = task.into_box();
        let result = self.execute_with_read_lock(task_boxed);
        ExecutionContext::new(result)
    }

    /// Executes a read-write task (with return value)
    ///
    /// # State Transition
    ///
    /// Conditioned → ExecutionContext<R>
    ///
    /// # Arguments
    ///
    /// * `task` - Any type that implements
    ///   `MutatingFunctionOnce<T, Result<R, E>>`
    #[inline]
    pub fn call_mut<F, R, E>(self, task: F) -> ExecutionContext<R, E>
    where
        F: MutatingFunctionOnce<T, Result<R, E>> + 'static,
        E: Error + Send + Sync + 'static,
        R: 'static,
    {
        let task_boxed = task.into_box();
        let result = self.execute_with_write_lock(task_boxed);
        ExecutionContext::new(result)
    }

    /// Executes a read-only task (without return value)
    ///
    /// # Arguments
    ///
    /// * `task` - Any type that implements `FunctionOnce<T, Result<(), E>>`
    #[inline]
    pub fn execute<F, E>(self, task: F) -> ExecutionContext<(), E>
    where
        F: FunctionOnce<T, Result<(), E>> + 'static,
        E: Error + Send + Sync + 'static,
    {
        self.call(task)
    }

    /// Executes a read-write task (without return value)
    ///
    /// # Arguments
    ///
    /// * `task` - Any type that implements
    ///   `MutatingFunctionOnce<T, Result<(), E>>`
    #[inline]
    pub fn execute_mut<F, E>(self, task: F) -> ExecutionContext<(), E>
    where
        F: MutatingFunctionOnce<T, Result<(), E>> + 'static,
        E: Error + Send + Sync + 'static,
    {
        self.call_mut(task)
    }

    // ========================================================================
    // Internal helper methods
    // ========================================================================

    fn execute_with_read_lock<R, E>(
        mut self,
        task: BoxFunctionOnce<T, Result<R, E>>,
    ) -> ExecutionResult<R, E>
    where
        E: Error + Send + Sync + 'static,
    {
        // First check (outside lock)
        let tester = self
            .tester
            .take()
            .expect("Tester must be set in Conditioned state");
        if !tester.test() {
            if let Some(ref log_config) = self.logger {
                log::log!(log_config.level, "{}", log_config.message);
            }
            return ExecutionResult::ConditionNotMet;
        }

        // Execute prepare action
        if let Some(prepare_action) = self.prepare_action.take() {
            if let Err(e) = prepare_action.get() {
                log::error!("Prepare action failed: {}", e);
                return ExecutionResult::Failed(ExecutorError::PrepareFailed(e.to_string()));
            }
        }

        // Acquire lock and execute
        self.lock.read(|data| {
            // Second check (inside lock)
            if !tester.test() {
                if let Some(ref log_config) = self.logger {
                    log::log!(log_config.level, "{}", log_config.message);
                }
                return ExecutionResult::ConditionNotMet;
            }
            // Execute task
            match task.apply(data) {
                Ok(v) => ExecutionResult::Success(v),
                Err(e) => ExecutionResult::Failed(ExecutorError::TaskFailed(e)),
            }
        })
    }

    fn execute_with_write_lock<R, E>(
        mut self,
        task: BoxMutatingFunctionOnce<T, Result<R, E>>,
    ) -> ExecutionResult<R, E>
    where
        E: Error + Send + Sync + 'static,
    {
        // First check (outside lock)
        let tester = self
            .tester
            .take()
            .expect("Tester must be set in Conditioned state");
        if !tester.test() {
            if let Some(ref log_config) = self.logger {
                log::log!(log_config.level, "{}", log_config.message);
            }
            return ExecutionResult::ConditionNotMet;
        }

        // Execute prepare action
        if let Some(prepare_action) = self.prepare_action.take() {
            if let Err(e) = prepare_action.get() {
                log::error!("Prepare action failed: {}", e);
                return ExecutionResult::Failed(ExecutorError::PrepareFailed(e.to_string()));
            }
        }

        // Acquire lock and execute
        self.lock.write(|data| {
            // Second check (inside lock)
            if !tester.test() {
                if let Some(ref log_config) = self.logger {
                    log::log!(log_config.level, "{}", log_config.message);
                }
                return ExecutionResult::ConditionNotMet;
            }
            // Execute task
            match task.apply(data) {
                Ok(v) => ExecutionResult::Success(v),
                Err(e) => ExecutionResult::Failed(ExecutorError::TaskFailed(e)),
            }
        })
    }
}

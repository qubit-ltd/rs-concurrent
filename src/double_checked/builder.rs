/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use std::{error::Error, marker::PhantomData};

use prism3_function::{BoxTester, Tester};

use super::{ExecutionResult, LogConfig};
use crate::lock::Lock;

/// A builder for constructing and executing a double-checked locking operation.
///
/// This builder uses a fluent API to configure various aspects of the
/// operation, such as the condition tester, preparation and rollback actions,
/// and logging.
///
/// # Type Parameters
///
/// * `'a` - The lifetime of the lock.
/// * `L` - The type of the lock, which must implement `Lock<T>`.
/// * `T` - The type of the data protected by the lock.
///
/// # Author
///
/// Haixing Hu
pub struct ExecutionBuilder<'a, L, T>
where
    L: Lock<T>,
{
    lock: &'a L,
    tester: BoxTester,
    prepare_action: Option<Box<dyn FnOnce() -> Result<(), Box<dyn Error + Send + Sync>> + 'a>>,
    rollback_action: Option<Box<dyn FnOnce() -> Result<(), Box<dyn Error + Send + Sync>> + 'a>>,
    logger: Option<LogConfig>,
    _phantom: PhantomData<T>,
}

impl<'a, L, T> ExecutionBuilder<'a, L, T>
where
    L: Lock<T>,
{
    /// Creates a new `ExecutionBuilder` with the specified lock.
    ///
    /// The builder is initialized with a default tester that always returns `true`.
    ///
    /// # Arguments
    ///
    /// * `lock` - A reference to an object that implements the `Lock<T>` trait.
    ///
    /// # Returns
    ///
    /// Returns a new `ExecutionBuilder` instance.
    pub(super) fn new(lock: &'a L) -> Self {
        Self {
            lock,
            tester: BoxTester::new(|| true),
            prepare_action: None,
            rollback_action: None,
            logger: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the condition tester for the double-checked lock.
    ///
    /// The tester is a closure that returns `true` if the operation should
    /// proceed and `false` otherwise. It is checked once before acquiring the
    /// lock and once after.
    ///
    /// # Arguments
    ///
    /// * `tester` - A tester that implements the `Tester` trait.
    pub fn when<Tst>(mut self, tester: Tst) -> Self
    where
        Tst: Tester + 'static,
    {
        self.tester = tester.into_box();
        self
    }

    /// Sets an action to be performed before acquiring the lock.
    ///
    /// This action is executed only if the initial condition check passes. If
    /// this action fails, the entire operation is aborted.
    ///
    /// # Arguments
    ///
    /// * `prepare_action` - A closure to execute before the lock is acquired.
    pub fn prepare<F, E>(mut self, prepare_action: F) -> Self
    where
        F: FnOnce() -> Result<(), E> + 'a,
        E: Error + Send + Sync + 'static,
    {
        self.prepare_action = Some(Box::new(move || {
            prepare_action().map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        }));
        self
    }

    /// Sets a rollback action to be performed if the operation fails.
    ///
    /// The rollback action is executed if:
    /// - The condition check fails after the lock is acquired.
    /// - The main task fails.
    ///
    /// # Arguments
    ///
    /// * `rollback_action` - A closure to execute on failure.
    pub fn rollback<F, E>(mut self, rollback_action: F) -> Self
    where
        F: FnOnce() -> Result<(), E> + 'a,
        E: Error + Send + Sync + 'static,
    {
        self.rollback_action = Some(Box::new(move || {
            rollback_action().map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
        }));
        self
    }

    /// Configures logging for when the condition check fails.
    ///
    /// # Arguments
    ///
    /// * `level` - The `log::Level` for the message.
    /// * `message` - The message to log.
    pub fn logger(mut self, level: log::Level, message: impl Into<String>) -> Self {
        self.logger = Some(LogConfig {
            level,
            message: message.into(),
        });
        self
    }
}

// Implementation of execution methods
impl<'a, L, T> ExecutionBuilder<'a, L, T>
where
    L: Lock<T>,
{
    /// Executes a read-only task that returns a value.
    pub fn call<F, R, E>(self, task: F) -> ExecutionResult<R>
    where
        F: FnOnce(&T) -> Result<R, E>,
        E: Error + Send + Sync + 'static,
    {
        if !self.tester.test() {
            self.handle_condition_not_met();
            return ExecutionResult::unmet();
        }
        if let Some(prepare_action) = self.prepare_action {
            if let Err(e) = prepare_action() {
                log::error!("Prepare action failed: {}", e);
                return ExecutionResult::fail_with_box(e);
            }
        }
        let tester = &self.tester;
        let logger = &self.logger;
        let handle_condition_not_met = || {
            if let Some(ref log_config) = logger {
                log::log!(log_config.level, "{}", log_config.message);
            }
        };

        let result = self.lock.read(|data| {
            if !tester.test() {
                handle_condition_not_met();
                return ExecutionResult::unmet();
            }
            task(data).map_or_else(ExecutionResult::fail, ExecutionResult::succeed)
        });

        if !result.success {
            if let Some(rollback_action) = self.rollback_action {
                if let Err(e) = rollback_action() {
                    log::error!("Rollback action failed: {}", e);
                }
            }
        }
        result
    }

    /// Executes a read-write task that returns a value.
    pub fn call_mut<F, R, E>(self, task: F) -> ExecutionResult<R>
    where
        F: FnOnce(&mut T) -> Result<R, E>,
        E: Error + Send + Sync + 'static,
    {
        if !self.tester.test() {
            self.handle_condition_not_met();
            return ExecutionResult::unmet();
        }
        if let Some(prepare_action) = self.prepare_action {
            if let Err(e) = prepare_action() {
                log::error!("Prepare action failed: {}", e);
                return ExecutionResult::fail_with_box(e);
            }
        }
        let tester = &self.tester;
        let logger = &self.logger;
        let handle_condition_not_met = || {
            if let Some(ref log_config) = logger {
                log::log!(log_config.level, "{}", log_config.message);
            }
        };

        let result = self.lock.write(|data| {
            if !tester.test() {
                handle_condition_not_met();
                return ExecutionResult::unmet();
            }
            task(data).map_or_else(ExecutionResult::fail, ExecutionResult::succeed)
        });

        if !result.success {
            if let Some(rollback_action) = self.rollback_action {
                if let Err(e) = rollback_action() {
                    log::error!("Rollback action failed: {}", e);
                }
            }
        }
        result
    }

    /// Executes a read-only task with no return value.
    pub fn execute<F, E>(self, task: F) -> ExecutionResult<()>
    where
        F: FnOnce(&T) -> Result<(), E>,
        E: Error + Send + Sync + 'static,
    {
        self.call(task)
    }

    /// Executes a read-write task with no return value.
    pub fn execute_mut<F, E>(self, task: F) -> ExecutionResult<()>
    where
        F: FnOnce(&mut T) -> Result<(), E>,
        E: Error + Send + Sync + 'static,
    {
        self.call_mut(task)
    }

    fn handle_condition_not_met(&self) {
        if let Some(ref log_config) = self.logger {
            log::log!(log_config.level, "{}", log_config.message);
        }
    }
}

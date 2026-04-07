/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Locking Fluent API
//!
//! Provides a fluent API for building and executing a double-checked locking
//! operation.
//!
//! # Author
//!
//! Haixing Hu
use super::{
    states::Initial,
    ExecutionBuilder,
};
use crate::lock::Lock;

/// The entry point for the fluent API of the double-checked locking
/// executor.
///
/// This struct provides a starting point for building and executing a
/// double-checked locking operation in a chainable and readable way.
///
/// # Features
///
/// - **Fluent API**: Provides a chainable interface for configuration
/// - **Double-Checking**: Checks the condition before and after acquiring
///   the lock to minimize unnecessary lock contention
/// - **Flexible Configuration**: Supports prepare actions, rollback actions,
///   and logging
/// - **Type Safety**: Leverages Rust's type system to ensure thread safety
///
/// # Use Cases
///
/// - **Lazy Initialization**: Lazily initialize shared resources in
///   multi-threaded environments
/// - **Conditional Updates**: Update shared data only when specific
///   conditions are met
/// - **Performance Optimization**: Reduce unnecessary lock acquisitions to
///   improve concurrency
/// - **Transactional Operations**: Operations that require prepare and
///   rollback mechanisms
///
/// # Examples
///
/// ## Simple Read Operation
///
/// ```rust
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let data = ArcMutex::new(42);
///
/// let result = DoubleCheckedLock::on(&data)
///     .when(|| true)  // Always execute
///     .call(|value: &i32| {
///         Ok::<i32, std::io::Error>(*value)
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), 42);
/// ```
///
/// ## Simple Write Operation
///
/// ```rust
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let data = ArcMutex::new(0);
///
/// let result = DoubleCheckedLock::on(&data)
///     .when(|| true)  // Always execute
///     .call_mut(|value: &mut i32| {
///         *value = 100;
///         Ok::<(), std::io::Error>(())
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(data.read(|v| *v), 100);
/// ```
///
/// ## Conditional Execution
///
/// ```rust
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let counter = ArcMutex::new(0);
///
/// let result = DoubleCheckedLock::on(&counter)
///     .when(|| true)  // Always execute
///     .call_mut(|value: &mut i32| {
///         *value += 1;
///         Ok::<i32, std::io::Error>(*value)
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), 1);
/// ```
///
/// ## Error Handling
///
/// ```rust
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let data = ArcMutex::new(5);
///
/// let result = DoubleCheckedLock::on(&data)
///     .when(|| true)  // Always execute
///     .call_mut(|value: &mut i32| {
///         if *value < 10 {
///             Err(std::io::Error::new(
///                 std::io::ErrorKind::Other,
///                 "Value too small"
///             ))
///         } else {
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .get_result();
///
/// assert!(result.is_failed());
/// ```
///
/// ## Basic Usage - Lazy Initialization
///
/// ```rust
/// use std::sync::Arc;
/// use qubit_atomic::AtomicBool;
/// use qubit_concurrent::{
///     DoubleCheckedLock,
///     ArcMutex,
/// };
///
/// // Lazy initialization of a resource
/// let resource = ArcMutex::new(None::<String>);
/// let initialized = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&resource)
///     .when({
///         let initialized = initialized.clone();
///         move || !initialized.load()
///     })
///     .call_mut({
///         let initialized = initialized.clone();
///         move |data: &mut Option<String>| {
///             *data = Some("Initialized Resource".to_string());
///             initialized.store(true);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert!(initialized.load());
/// ```
///
/// ## Database Transaction with Prepare and Rollback
///
/// ```rust
/// use std::sync::Arc;
/// use qubit_atomic::AtomicBool;
/// use qubit_concurrent::{
///     double_checked::DoubleCheckedLock,
///     lock::ArcMutex,
/// };
///
/// // Simulate a database record
/// let balance = ArcMutex::new(1000);
/// let transaction_active = Arc::new(AtomicBool::new(false));
/// let connection_opened = Arc::new(AtomicBool::new(false));
/// let transaction_rolled_back = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&balance)
///     .when({
///         let transaction_active = transaction_active.clone();
///         move || !transaction_active.load()
///     })
///     .prepare({
///         let connection_opened = connection_opened.clone();
///         move || {
///             // Open database connection
///             connection_opened.store(true);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .call_mut(move |amount: &mut i32| {
///         // Deduct from balance
///         if *amount >= 100 {
///             *amount -= 100;
///             transaction_active.store(true);
///             Ok::<i32, std::io::Error>(*amount)
///         } else {
///             Err(std::io::Error::new(
///                 std::io::ErrorKind::Other,
///                 "Insufficient balance"
///             ))
///         }
///     })
///     .rollback({
///         let transaction_rolled_back = transaction_rolled_back.clone();
///         move || {
///             // Rollback transaction and close connection
///             transaction_rolled_back.store(true);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert!(connection_opened.load());
/// ```
///
/// ## Cache Lookup with Read Lock
///
/// ```rust
/// use std::sync::Arc;
/// use qubit_atomic::AtomicBool;
/// use qubit_concurrent::{
///     double_checked::DoubleCheckedLock,
///     lock::ArcRwLock,
/// };
///
/// // Check if cache is valid before reading
/// let cache = ArcRwLock::new(Some(42));
/// let cache_valid = Arc::new(AtomicBool::new(true));
///
/// let result = DoubleCheckedLock::on(&cache)
///     .when({
///         let cache_valid = cache_valid.clone();
///         move || cache_valid.load()
///     })
///     .call(|data: &Option<i32>| {
///         Ok::<Option<i32>, std::io::Error>(*data)
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), Some(42));
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct DoubleCheckedLock;

impl DoubleCheckedLock {
    /// Starts a double-checked locking operation on the specified lock.
    ///
    /// This is the entry point for all double-checked locking operations.
    /// It returns an `ExecutionBuilder` that allows you to configure the
    /// operation with conditions, actions, and error handling.
    ///
    /// # Examples
    ///
    /// ## Basic read operation
    ///
    /// ```rust
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let data = ArcMutex::new(42);
    ///
    /// let result = DoubleCheckedLock::on(&data)
    ///     .when(|| true)
    ///     .call(|value: &i32| {
    ///         Ok::<i32, std::io::Error>(*value)
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(result.unwrap(), 42);
    /// ```
    ///
    /// ## Basic write operation
    ///
    /// ```rust
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let data = ArcMutex::new(0);
    ///
    /// let result = DoubleCheckedLock::on(&data)
    ///     .when(|| true)
    ///     .call_mut(|value: &mut i32| {
    ///         *value = 100;
    ///         Ok::<(), std::io::Error>(())
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(data.read(|v| *v), 100);
    /// ```
    ///
    /// ## Conditional execution
    ///
    /// ```rust
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let counter = ArcMutex::new(0);
    ///
    /// let result = DoubleCheckedLock::on(&counter)
    ///     .when(|| true)  // Check condition before acquiring lock
    ///     .call_mut(|value: &mut i32| {
    ///         *value += 1;
    ///         Ok::<i32, std::io::Error>(*value)
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(result.unwrap(), 1);
    /// ```
    ///
    /// # Type Parameters
    ///
    /// * `'a` - The lifetime parameter for the lock reference
    /// * `L` - The lock type that must implement the `Lock<T>` trait
    /// * `T` - The type of data protected by the lock
    ///
    /// # Parameters
    ///
    /// * `lock` - A reference to an object that implements the `Lock<T>` trait
    ///
    /// # Returns
    ///
    /// Returns an `ExecutionBuilder` instance in Initial state to configure
    /// and execute the operation
    pub fn on<'a, L, T>(lock: &'a L) -> ExecutionBuilder<'a, L, T, Initial>
    where
        L: Lock<T>,
    {
        ExecutionBuilder::new(lock)
    }
}

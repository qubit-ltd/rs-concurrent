/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
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
use super::ExecutionBuilder;
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
/// ## Basic Usage - Lazy Initialization
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{
///     AtomicBool,
///     Ordering,
/// };
/// use prism3_concurrent::{
///     DoubleCheckedLockExecutor,
///     ArcMutex,
/// };
///
/// // Lazy initialization of a resource
/// let resource = ArcMutex::new(None::<String>);
/// let initialized = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLockExecutor::on(&resource)
///     .when({
///         let initialized = initialized.clone();
///         move || !initialized.load(Ordering::Acquire)
///     })
///     .call_mut(|data| {
///         *data = Some("Initialized Resource".to_string());
///         initialized.store(true, Ordering::Release);
///         Ok::<(), std::io::Error>(())
///     });
///
/// assert!(result.success);
/// assert!(initialized.load(Ordering::Acquire));
/// ```
///
/// ## Database Transaction with Prepare and Rollback
///
/// ```rust
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
/// use prism3_concurrent::{
///     double_checked::DoubleCheckedLockExecutor,
///     lock::ArcMutex,
/// };
///
/// // Simulate a database record
/// let balance = ArcMutex::new(1000);
/// let transaction_active = Arc::new(AtomicBool::new(false));
/// let connection_opened = Arc::new(AtomicBool::new(false));
/// let transaction_rolled_back = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLockExecutor::on(&balance)
///     .when({
///         let transaction_active = transaction_active.clone();
///         move || !transaction_active.load(Ordering::Acquire)
///     })
///     .prepare({
///         let connection_opened = connection_opened.clone();
///         move || {
///             // Open database connection
///             connection_opened.store(true, Ordering::Release);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .rollback({
///         let transaction_rolled_back = transaction_rolled_back.clone();
///         move || {
///             // Rollback transaction and close connection
///             transaction_rolled_back.store(true, Ordering::Release);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .call_mut(|amount| {
///         // Deduct from balance
///         if *amount >= 100 {
///             *amount -= 100;
///             transaction_active.store(true, Ordering::Release);
///             Ok::<i32, std::io::Error>(*amount)
///         } else {
///             Err(std::io::Error::new(
///                 std::io::ErrorKind::Other,
///                 "Insufficient balance"
///             ))
///         }
///     });
///
/// assert!(result.success);
/// assert!(connection_opened.load(Ordering::Acquire));
/// ```
///
/// ## Cache Lookup with Read Lock
///
/// ```rust
/// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
/// use prism3_concurrent::{
///     double_checked::DoubleCheckedLockExecutor,
///     lock::ArcRwLock,
/// };
///
/// // Check if cache is valid before reading
/// let cache = ArcRwLock::new(Some(42));
/// let cache_valid = Arc::new(AtomicBool::new(true));
///
/// let result = DoubleCheckedLockExecutor::on(&cache)
///     .when({
///         let cache_valid = cache_valid.clone();
///         move || cache_valid.load(Ordering::Acquire)
///     })
///     .call(|data| {
///         Ok::<Option<i32>, std::io::Error>(*data)
///     });
///
/// assert!(result.success);
/// assert_eq!(result.value, Some(Some(42)));
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct DoubleCheckedLockExecutor;

impl DoubleCheckedLockExecutor {
    /// Starts a double-checked locking operation on the specified lock.
    ///
    /// # Arguments
    ///
    /// * `lock` - A reference to an object that implements the `Lock<T>` trait.
    ///
    /// # Returns
    ///
    /// Returns a `ExecutionBuilder` instance to configure
    /// and execute the operation.
    pub fn on<'a, L, T>(lock: &'a L) -> ExecutionBuilder<'a, L, T>
    where
        L: Lock<T>,
    {
        ExecutionBuilder::new(lock)
    }
}

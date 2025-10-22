/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Synchronous Read-Write Lock Wrapper
//!
//! Provides an Arc-wrapped synchronous read-write lock for protecting
//! shared data with multiple concurrent readers or a single writer.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::{Arc, RwLock};
use crate::lock::read_write_lock::ReadWriteLock;

/// Synchronous Read-Write Lock Wrapper
///
/// Provides an encapsulation of synchronous read-write lock,
/// supporting multiple read operations or a single write operation.
/// Read operations can execute concurrently, while write operations
/// have exclusive access.
///
/// # Features
///
/// - Supports multiple concurrent read operations
/// - Write operations have exclusive access, mutually exclusive with
///   read operations
/// - Synchronously acquires locks, may block threads
/// - Thread-safe, supports multi-threaded sharing
/// - Automatic lock management through RAII ensures proper lock
///   release
///
/// # Use Cases
///
/// Suitable for read-heavy scenarios such as caching, configuration
/// management, etc.
///
/// # Usage Example
///
/// ```rust,ignore
/// use prism3_rust_concurrent::lock::{ArcRwLock, ReadWriteLock};
/// use std::sync::Arc;
///
/// let data = ArcRwLock::new(String::from("Hello"));
/// let data = Arc::new(data);
///
/// // Multiple read operations can execute concurrently
/// data.with_read_lock(|s| {
///     println!("Read: {}", s);
/// });
///
/// // Write operations have exclusive access
/// data.with_write_lock(|s| {
///     s.push_str(" World!");
///     println!("Write: {}", s);
/// });
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct ArcRwLock<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> ArcRwLock<T> {
    /// Creates a new synchronous read-write lock
    ///
    /// # Arguments
    ///
    /// * `data` - The data to be protected
    ///
    /// # Returns
    ///
    /// Returns a new `ArcRwLock` instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::ArcRwLock;
    ///
    /// let rw_lock = ArcRwLock::new(vec![1, 2, 3]);
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(data)),
        }
    }
}

impl<T> ReadWriteLock<T> for ArcRwLock<T> {
    /// Acquires the read lock and executes an operation
    ///
    /// Synchronously acquires the read lock, executes the provided
    /// closure, and then automatically releases the lock. Multiple
    /// read operations can execute concurrently.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the read
    ///   lock, can only read data
    ///
    /// # Returns
    ///
    /// Returns the result of executing the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{ArcRwLock, ReadWriteLock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// let length = data.with_read_lock(|v| v.len());
    /// println!("Vector length: {}", length);
    /// ```
    fn with_read_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.inner.read().unwrap();
        f(&*guard)
    }

    /// Acquires the write lock and executes an operation
    ///
    /// Synchronously acquires the write lock, executes the provided
    /// closure, and then automatically releases the lock. Write
    /// operations have exclusive access, mutually exclusive with
    /// read operations.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the write
    ///   lock, can modify data
    ///
    /// # Returns
    ///
    /// Returns the result of executing the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{ArcRwLock, ReadWriteLock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// data.with_write_lock(|v| {
    ///     v.push(4);
    ///     println!("Added element, new length: {}", v.len());
    /// });
    /// ```
    fn with_write_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.write().unwrap();
        f(&mut *guard)
    }
}

impl<T> Clone for ArcRwLock<T> {
    /// Clones the synchronous read-write lock
    ///
    /// Creates a new `ArcRwLock` instance that shares the same
    /// underlying lock with the original instance. This allows
    /// multiple threads to hold references to the same lock
    /// simultaneously.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}


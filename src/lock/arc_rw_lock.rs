/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
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

use std::sync::{
    Arc,
    RwLock,
};

use crate::lock::Lock;

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
/// use qubit_concurrent::lock::{ArcRwLock, ReadWriteLock};
/// use std::sync::Arc;
///
/// let data = ArcRwLock::new(String::from("Hello"));
///
/// // Multiple read operations can execute concurrently
/// data.read(|s| {
///     println!("Read: {}", s);
/// });
///
/// // Write operations have exclusive access
/// data.write(|s| {
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
    /// use qubit_concurrent::lock::ArcRwLock;
    ///
    /// let rw_lock = ArcRwLock::new(vec![1, 2, 3]);
    /// ```
    #[inline]
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(data)),
        }
    }
}

impl<T> Lock<T> for ArcRwLock<T> {
    /// Acquires a read lock and executes an operation
    ///
    /// Synchronously acquires the read lock, executes the provided
    /// closure, and then automatically releases the lock. Multiple
    /// read operations can execute concurrently, providing better
    /// performance for read-heavy workloads.
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
    /// use qubit_concurrent::lock::{ArcRwLock, Lock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// let length = data.read(|v| v.len());
    /// println!("Vector length: {}", length);
    /// ```
    #[inline]
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.inner.read().unwrap();
        f(&*guard)
    }

    /// Acquires a write lock and executes an operation
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
    /// use qubit_concurrent::lock::{ArcRwLock, Lock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// data.write(|v| {
    ///     v.push(4);
    ///     println!("Added element, new length: {}", v.len());
    /// });
    /// ```
    #[inline]
    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.write().unwrap();
        f(&mut *guard)
    }

    /// Attempts to acquire a read lock without blocking
    ///
    /// Attempts to immediately acquire the read lock. If the lock is
    /// currently held by another thread in write mode, returns `None`
    /// without blocking. This is a non-blocking operation.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the read lock
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was successfully acquired and the
    ///   closure executed
    /// * `None` - If the lock is currently held in write mode
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcRwLock, Lock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// if let Some(length) = data.try_read(|v| v.len()) {
    ///     println!("Vector length: {}", length);
    /// } else {
    ///     println!("Lock is busy with write operation");
    /// }
    /// ```
    #[inline]
    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Ok(guard) = self.inner.try_read() {
            Some(f(&*guard))
        } else {
            None
        }
    }

    /// Attempts to acquire a write lock without blocking
    ///
    /// Attempts to immediately acquire the write lock. If the lock is
    /// currently held by another thread (in either read or write mode),
    /// returns `None` without blocking. This is a non-blocking operation.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the write lock
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was successfully acquired and the
    ///   closure executed
    /// * `None` - If the lock is currently held by another thread
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcRwLock, Lock};
    ///
    /// let data = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// if let Some(new_length) = data.try_write(|v| {
    ///     v.push(4);
    ///     v.len()
    /// }) {
    ///     println!("New length: {}", new_length);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    #[inline]
    fn try_write<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if let Ok(mut guard) = self.inner.try_write() {
            Some(f(&mut *guard))
        } else {
            None
        }
    }
}

impl<T> Clone for ArcRwLock<T> {
    /// Clones the synchronous read-write lock
    ///
    /// Creates a new `ArcRwLock` instance that shares the same
    /// underlying lock with the original instance. This allows
    /// multiple threads to hold references to the same lock
    /// simultaneously.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

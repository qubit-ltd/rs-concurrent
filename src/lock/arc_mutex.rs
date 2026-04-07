/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

//! # Synchronous Mutex Wrapper (Parking Lot)
//!
//! Provides an Arc-wrapped synchronous mutex using parking_lot::Mutex
//! for protecting shared data in multi-threaded environments.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::Arc;

use parking_lot::Mutex;

use crate::lock::Lock;

/// Synchronous Mutex Wrapper (Parking Lot)
///
/// Provides an encapsulation of synchronous mutex using parking_lot::Mutex
/// for protecting shared data in synchronous environments. Supports safe
/// access and modification of shared data across multiple threads.
/// Compared to std::sync::Mutex, parking_lot::Mutex provides better
/// performance and more ergonomic API.
///
/// # Features
///
/// - Synchronously acquires locks, may block threads
/// - Supports trying to acquire locks (non-blocking)
/// - Thread-safe, supports multi-threaded sharing
/// - Automatic lock management through RAII ensures proper lock
///   release
/// - Better performance compared to std::sync::Mutex
/// - More ergonomic API with no unwrap() calls
///
/// # Usage Example
///
/// ```rust,ignore
/// use qubit_concurrent::lock::{ArcMutex, Lock};
/// use std::sync::Arc;
///
/// let counter = ArcMutex::new(0);
/// let counter = Arc::new(counter);
///
/// // Synchronously modify data
/// counter.write(|c| {
///     *c += 1;
///     println!("Counter: {}", *c);
/// });
///
/// // Try to acquire lock
/// if let Some(value) = counter.try_read(|c| *c) {
///     println!("Current value: {}", value);
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct ArcMutex<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> ArcMutex<T> {
    /// Creates a new synchronous mutex lock
    ///
    /// # Arguments
    ///
    /// * `data` - The data to be protected
    ///
    /// # Returns
    ///
    /// Returns a new `ArcMutex` instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::ArcMutex;
    ///
    /// let lock = ArcMutex::new(42);
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
        }
    }
}

impl<T> Lock<T> for ArcMutex<T> {
    /// Acquires a read lock and executes an operation
    ///
    /// For ArcMutex, this acquires the same exclusive lock as write
    /// operations, but provides immutable access to the data. This
    /// ensures thread safety while allowing read-only operations.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the read lock
    ///
    /// # Returns
    ///
    /// Returns the result of executing the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(42);
    ///
    /// let value = counter.read(|c| *c);
    /// println!("Current value: {}", value);
    /// ```
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.inner.lock();
        f(&*guard)
    }

    /// Acquires a write lock and executes an operation
    ///
    /// Synchronously acquires the exclusive lock, executes the provided
    /// closure with mutable access, and then automatically releases
    /// the lock. This is the recommended usage pattern for modifications.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the write lock
    ///
    /// # Returns
    ///
    /// Returns the result of executing the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(0);
    ///
    /// let result = counter.write(|c| {
    ///     *c += 1;
    ///     *c
    /// });
    ///
    /// println!("Counter value: {}", result);
    /// ```
    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.lock();
        f(&mut *guard)
    }

    /// Attempts to acquire a read lock without blocking
    ///
    /// Attempts to immediately acquire the read lock. If the lock is
    /// already held by another thread, returns `None`. This is a
    /// non-blocking operation.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the read lock
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was successfully acquired and the
    ///   closure executed
    /// * `None` - If the lock is already held by another thread
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(42);
    ///
    /// if let Some(value) = counter.try_read(|c| *c) {
    ///     println!("Current value: {}", value);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Some(guard) = self.inner.try_lock() {
            Some(f(&*guard))
        } else {
            None
        }
    }

    /// Attempts to acquire a write lock without blocking
    ///
    /// Attempts to immediately acquire the write lock. If the lock is
    /// already held by another thread, returns `None`. This is a
    /// non-blocking operation.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the write lock
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was successfully acquired and the
    ///   closure executed
    /// * `None` - If the lock is already held by another thread
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(0);
    ///
    /// if let Some(result) = counter.try_write(|c| {
    ///     *c += 1;
    ///     *c
    /// }) {
    ///     println!("New value: {}", result);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    fn try_write<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if let Some(mut guard) = self.inner.try_lock() {
            Some(f(&mut *guard))
        } else {
            None
        }
    }
}

impl<T> Clone for ArcMutex<T> {
    /// Clones the synchronous mutex
    ///
    /// Creates a new `ArcMutex` instance that shares the same
    /// underlying lock with the original instance. This allows
    /// multiple threads to hold references to the same lock
    /// simultaneously.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

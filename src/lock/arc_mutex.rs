/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Synchronous Mutex Wrapper
//!
//! Provides an Arc-wrapped synchronous mutex for protecting shared
//! data in multi-threaded environments.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::{Arc, Mutex};
use crate::lock::lock::Lock;

/// Synchronous Mutex Wrapper
///
/// Provides an encapsulation of synchronous mutex for protecting
/// shared data in synchronous environments. Supports safe access and
/// modification of shared data across multiple threads.
///
/// # Features
///
/// - Synchronously acquires locks, may block threads
/// - Supports trying to acquire locks (non-blocking)
/// - Thread-safe, supports multi-threaded sharing
/// - Automatic lock management through RAII ensures proper lock
///   release
///
/// # Usage Example
///
/// ```rust,ignore
/// use prism3_rust_concurrent::lock::{ArcMutex, Lock};
/// use std::sync::Arc;
///
/// let counter = ArcMutex::new(0);
/// let counter = Arc::new(counter);
///
/// // Synchronously modify data
/// counter.with_lock(|c| {
///     *c += 1;
///     println!("Counter: {}", *c);
/// });
///
/// // Try to acquire lock
/// if let Some(value) = counter.try_with_lock(|c| *c) {
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
    /// use prism3_rust_concurrent::lock::ArcMutex;
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
    /// Acquires the lock and executes an operation
    ///
    /// Synchronously acquires the lock, executes the provided
    /// closure, and then automatically releases the lock. This is
    /// the recommended usage pattern as it ensures proper lock
    /// release.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the lock
    ///
    /// # Returns
    ///
    /// Returns the result of executing the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(0);
    ///
    /// let result = counter.with_lock(|c| {
    ///     *c += 1;
    ///     *c
    /// });
    ///
    /// println!("Counter value: {}", result);
    /// ```
    fn with_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.lock().unwrap();
        f(&mut *guard)
    }

    /// Attempts to acquire the lock
    ///
    /// Attempts to immediately acquire the lock. If the lock is
    /// already held by another thread, returns `None`. This is a
    /// non-blocking operation.
    ///
    /// # Arguments
    ///
    /// * `f` - The closure to be executed while holding the lock
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
    /// use prism3_rust_concurrent::lock::{ArcMutex, Lock};
    ///
    /// let counter = ArcMutex::new(0);
    ///
    /// // Try to acquire lock
    /// if let Some(value) = counter.try_with_lock(|c| *c) {
    ///     println!("Current value: {}", value);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    fn try_with_lock<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if let Ok(mut guard) = self.inner.try_lock() {
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


/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Asynchronous Mutex Wrapper
//!
//! Provides an Arc-wrapped asynchronous mutex for protecting shared
//! data in async environments without blocking threads.
//!
//! # Author
//!
//! Haixing Hu
use std::sync::Arc;

use tokio::sync::Mutex as AsyncMutex;

use crate::lock::AsyncLock;

/// Asynchronous Mutex Wrapper
///
/// Provides an encapsulation of asynchronous mutex for protecting
/// shared data in asynchronous environments. Supports safe access
/// and modification of shared data across multiple asynchronous
/// tasks.
///
/// # Features
///
/// - Asynchronously acquires locks, does not block threads
/// - Supports trying to acquire locks (non-blocking)
/// - Thread-safe, supports multi-threaded sharing
/// - Automatic lock management through RAII ensures proper lock
///   release
///
/// # Usage Example
///
/// ```rust,ignore
/// use qubit_concurrent::lock::{ArcAsyncMutex, AsyncLock};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let counter = ArcAsyncMutex::new(0);
///     let counter = Arc::new(counter);
///
///     // Asynchronously modify data
///     counter.write(|c| {
///         *c += 1;
///         println!("Counter: {}", *c);
///     }).await;
///
///     // Try to acquire lock
///     if let Some(value) = counter.try_read(|c| *c) {
///         println!("Current value: {}", value);
///     }
/// }
/// ```
///
/// # Author
///
/// Haixing Hu
///
pub struct ArcAsyncMutex<T> {
    inner: Arc<AsyncMutex<T>>,
}

impl<T> ArcAsyncMutex<T> {
    /// Creates a new asynchronous mutex lock
    ///
    /// # Arguments
    ///
    /// * `data` - The data to be protected
    ///
    /// # Returns
    ///
    /// Returns a new `ArcAsyncMutex` instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::ArcAsyncMutex;
    ///
    /// let lock = ArcAsyncMutex::new(42);
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            inner: Arc::new(AsyncMutex::new(data)),
        }
    }
}

impl<T> AsyncLock<T> for ArcAsyncMutex<T>
where
    T: Send,
{
    /// Acquires the lock and executes an operation
    ///
    /// Asynchronously acquires the lock, executes the provided
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
    /// Returns a future that resolves to the result of executing
    /// the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{ArcAsyncMutex, AsyncLock};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let counter = ArcAsyncMutex::new(0);
    ///
    ///     let result = counter.write(|c| {
    ///         *c += 1;
    ///         *c
    ///     }).await;
    ///
    ///     println!("Counter value: {}", result);
    /// }
    /// ```
    async fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R + Send,
        R: Send,
    {
        let guard = self.inner.lock().await;
        f(&*guard)
    }

    async fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send,
    {
        let mut guard = self.inner.lock().await;
        f(&mut *guard)
    }

    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Ok(guard) = self.inner.try_lock() {
            Some(f(&*guard))
        } else {
            None
        }
    }

    fn try_write<R, F>(&self, f: F) -> Option<R>
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

impl<T> Clone for ArcAsyncMutex<T> {
    /// Clones the asynchronous mutex
    ///
    /// Creates a new `ArcAsyncMutex` instance that shares the same
    /// underlying lock with the original instance. This allows
    /// multiple tasks to hold references to the same lock
    /// simultaneously.
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

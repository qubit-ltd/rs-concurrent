/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Asynchronous Lock Trait
//!
//! Defines an asynchronous lock abstraction that supports acquiring
//! locks without blocking threads.
//!
//! # Author
//!
//! Haixing Hu
use std::future::Future;
use tokio::sync::Mutex as AsyncMutex;

/// Asynchronous lock trait
///
/// Provides a unified interface for different types of asynchronous
/// locks. This trait allows locks to be used in async contexts
/// through closures, avoiding the complexity of explicitly managing
/// lock guards and their lifetimes.
///
/// # Design Philosophy
///
/// The core methods of this trait accept closures that receive a
/// mutable reference to the protected data. Internally, these methods
/// asynchronously acquire the lock guard, dereference it to obtain the
/// data reference, and pass this reference to the user-provided
/// closure. This design pattern:
///
/// - Hides guard lifetime complexity from users
/// - Ensures locks are automatically released when closures return
/// - Provides a uniform API across different lock implementations
/// - Enables writing generic async code over various lock types
/// - Does not block threads while waiting for locks
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
pub trait AsyncLock<T: ?Sized> {
    /// Acquires the lock asynchronously and executes a closure
    ///
    /// This method awaits until the lock can be acquired without
    /// blocking the thread, then executes the provided closure with
    /// mutable access to the protected data. The lock is
    /// automatically released when the closure returns.
    ///
    /// # How It Works
    ///
    /// 1. Asynchronously awaits the lock guard (e.g., `MutexGuard`)
    /// 2. Dereferences the guard to obtain `&mut T`
    /// 3. Passes this mutable reference to the closure
    /// 4. Automatically releases the lock when the closure completes
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference (`&mut T`) to
    ///   the protected data. This reference is obtained by dereferencing
    ///   the acquired lock guard.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves to the result produced by
    /// the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{AsyncLock, ArcAsyncMutex};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let lock = ArcAsyncMutex::new(42);
    ///     let result = lock.with_lock(|value| {
    ///         *value += 1;
    ///         *value
    ///     }).await;
    ///     assert_eq!(result, 43);
    /// }
    /// ```
    fn with_lock<R, F>(&self, f: F) -> impl Future<Output = R> + Send
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send;

    /// Attempts to acquire the lock without waiting
    ///
    /// This method tries to acquire the lock immediately. If the lock
    /// is currently held by another task, it returns `None` without
    /// waiting. Otherwise, it executes the closure and returns `Some`
    /// containing the result.
    ///
    /// # How It Works
    ///
    /// 1. Attempts to acquire the lock guard without waiting
    /// 2. If successful, dereferences the guard to obtain `&mut T`
    /// 3. Passes this mutable reference to the closure
    /// 4. Returns `Some(result)` after releasing the lock
    /// 5. Returns `None` if the lock is currently held
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference (`&mut T`) to
    ///   the protected data if the lock is successfully acquired.
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was acquired and closure executed
    /// * `None` - If the lock is currently held by another task
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{AsyncLock, ArcAsyncMutex};
    ///
    /// let lock = ArcAsyncMutex::new(42);
    /// if let Some(result) = lock.try_with_lock(|value| *value) {
    ///     println!("Got value: {}", result);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    fn try_with_lock<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R;
}

/// Asynchronous mutex implementation for tokio::sync::Mutex
///
/// This implementation uses Tokio's `Mutex` type to provide an
/// asynchronous lock that can be awaited without blocking threads.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized + Send> AsyncLock<T> for AsyncMutex<T> {
    async fn with_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send,
    {
        let mut guard = self.lock().await;
        f(&mut *guard)
    }

    fn try_with_lock<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if let Ok(mut guard) = self.try_lock() {
            Some(f(&mut *guard))
        } else {
            None
        }
    }
}

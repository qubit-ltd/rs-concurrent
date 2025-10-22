/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Asynchronous Read-Write Lock Trait
//!
//! Defines an asynchronous read-write lock abstraction that supports
//! multiple concurrent readers or a single writer without blocking
//! threads.
//!
//! # Author
//!
//! Haixing Hu
use std::future::Future;
use tokio::sync::RwLock as AsyncRwLock;

/// Asynchronous read-write lock trait
///
/// Provides a unified interface for different types of asynchronous
/// read-write locks. This trait allows multiple concurrent read
/// operations or a single exclusive write operation in async
/// contexts.
///
/// # Design Philosophy
///
/// The core methods of this trait accept closures that receive
/// references to the protected data. Internally, these methods
/// asynchronously acquire the appropriate lock guard (read or write),
/// dereference it to obtain the data reference, and pass this
/// reference to the user-provided closure. This design pattern:
///
/// - Hides guard lifetime complexity from users
/// - Ensures locks are automatically released when closures return
/// - Provides a uniform API across different lock implementations
/// - Enables writing generic async code over various lock types
/// - Allows multiple concurrent readers with immutable access
/// - Provides exclusive writer access with mutable access
/// - Does not block threads while waiting for locks
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
pub trait AsyncReadWriteLock<T: ?Sized> {
    /// Acquires a read lock asynchronously and executes a closure
    ///
    /// This method awaits until a read lock can be acquired without
    /// blocking the thread. Multiple tasks can hold read locks
    /// simultaneously. The closure receives immutable access to the
    /// protected data. The lock is automatically released when the
    /// closure returns.
    ///
    /// # How It Works
    ///
    /// 1. Asynchronously awaits the read lock guard (e.g.,
    ///    `RwLockReadGuard`)
    /// 2. Dereferences the guard to obtain `&T`
    /// 3. Passes this immutable reference to the closure
    /// 4. Automatically releases the lock when the closure completes
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives an immutable reference (`&T`) to
    ///   the protected data. This reference is obtained by dereferencing
    ///   the acquired read lock guard.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves to the result produced by
    /// the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{AsyncReadWriteLock,
    ///                                     ArcAsyncRwLock};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let lock = ArcAsyncRwLock::new(vec![1, 2, 3]);
    ///     let len = lock.with_read_lock(|data| data.len()).await;
    ///     assert_eq!(len, 3);
    /// }
    /// ```
    fn with_read_lock<R, F>(&self, f: F) -> impl Future<Output = R> + Send
    where
        F: FnOnce(&T) -> R + Send,
        R: Send;

    /// Acquires a write lock asynchronously and executes a closure
    ///
    /// This method awaits until a write lock can be acquired without
    /// blocking the thread. Only one task can hold a write lock at
    /// a time, and write locks are mutually exclusive with read
    /// locks. The closure receives mutable access to the protected
    /// data. The lock is automatically released when the closure
    /// returns.
    ///
    /// # How It Works
    ///
    /// 1. Asynchronously awaits the write lock guard (e.g.,
    ///    `RwLockWriteGuard`)
    /// 2. Dereferences the guard to obtain `&mut T`
    /// 3. Passes this mutable reference to the closure
    /// 4. Automatically releases the lock when the closure completes
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference (`&mut T`) to
    ///   the protected data. This reference is obtained by dereferencing
    ///   the acquired write lock guard.
    ///
    /// # Returns
    ///
    /// Returns a future that resolves to the result produced by
    /// the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{AsyncReadWriteLock,
    ///                                     ArcAsyncRwLock};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let lock = ArcAsyncRwLock::new(vec![1, 2, 3]);
    ///     lock.with_write_lock(|data| {
    ///         data.push(4);
    ///     }).await;
    /// }
    /// ```
    fn with_write_lock<R, F>(&self, f: F) -> impl Future<Output = R> + Send
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send;
}

/// Asynchronous read-write lock implementation for tokio::sync::RwLock
///
/// This implementation uses Tokio's `RwLock` type to provide an
/// asynchronous read-write lock that supports multiple concurrent
/// readers or a single writer without blocking threads.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized + Send + Sync> AsyncReadWriteLock<T> for AsyncRwLock<T> {
    async fn with_read_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R + Send,
        R: Send,
    {
        let guard = self.read().await;
        f(&*guard)
    }

    async fn with_write_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R + Send,
        R: Send,
    {
        let mut guard = self.write().await;
        f(&mut *guard)
    }
}

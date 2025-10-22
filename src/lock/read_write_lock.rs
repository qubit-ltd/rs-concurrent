/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Read-Write Lock Trait
//!
//! Defines a synchronous read-write lock abstraction that supports
//! multiple concurrent readers or a single writer.
//!
//! # Author
//!
//! Haixing Hu
use std::sync::RwLock;

/// Synchronous read-write lock trait
///
/// Provides a unified interface for different types of synchronous
/// read-write locks. This trait allows multiple concurrent read
/// operations or a single exclusive write operation.
///
/// # Design Philosophy
///
/// The core methods of this trait accept closures that receive
/// references to the protected data. Internally, these methods acquire
/// the appropriate lock guard (read or write), dereference it to obtain
/// the data reference, and pass this reference to the user-provided
/// closure. This design pattern:
///
/// - Hides guard lifetime complexity from users
/// - Ensures locks are automatically released when closures return
/// - Provides a uniform API across different lock implementations
/// - Enables writing generic code over various lock types
/// - Allows multiple concurrent readers with immutable access
/// - Provides exclusive writer access with mutable access
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
pub trait ReadWriteLock<T: ?Sized> {
    /// Acquires a read lock and executes a closure
    ///
    /// This method blocks the current thread until a read lock can be
    /// acquired. Multiple threads can hold read locks simultaneously.
    /// The closure receives immutable access to the protected data.
    /// The lock is automatically released when the closure returns.
    ///
    /// # How It Works
    ///
    /// 1. Acquires the read lock guard (e.g., `RwLockReadGuard`)
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
    /// Returns the result produced by the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{ReadWriteLock, ArcRwLock};
    ///
    /// let lock = ArcRwLock::new(vec![1, 2, 3]);
    /// let len = lock.with_read_lock(|data| data.len());
    /// assert_eq!(len, 3);
    /// ```
    fn with_read_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// Acquires a write lock and executes a closure
    ///
    /// This method blocks the current thread until a write lock can be
    /// acquired. Only one thread can hold a write lock at a time, and
    /// write locks are mutually exclusive with read locks. The closure
    /// receives mutable access to the protected data. The lock is
    /// automatically released when the closure returns.
    ///
    /// # How It Works
    ///
    /// 1. Acquires the write lock guard (e.g., `RwLockWriteGuard`)
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
    /// Returns the result produced by the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{ReadWriteLock, ArcRwLock};
    ///
    /// let lock = ArcRwLock::new(vec![1, 2, 3]);
    /// lock.with_write_lock(|data| {
    ///     data.push(4);
    /// });
    /// ```
    fn with_write_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;
}

/// Synchronous read-write lock implementation for std::sync::RwLock
///
/// This implementation uses the standard library's `RwLock` type to
/// provide a synchronous read-write lock that supports multiple
/// concurrent readers or a single writer.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized> ReadWriteLock<T> for RwLock<T> {
    fn with_read_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read().unwrap();
        f(&*guard)
    }

    fn with_write_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.write().unwrap();
        f(&mut *guard)
    }
}

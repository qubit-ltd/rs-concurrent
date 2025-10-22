/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Lock Trait
//!
//! Defines a synchronous lock abstraction that supports acquiring locks
//! and executing operations within the locked context.
//!
//! # Author
//!
//! Haixing Hu
use std::sync::Mutex;

/// Synchronous lock trait
///
/// Provides a unified interface for different types of synchronous locks.
/// This trait allows locks to be used in a generic way through closures,
/// avoiding the complexity of explicitly managing lock guards and their
/// lifetimes.
///
/// # Design Philosophy
///
/// The core methods of this trait accept closures that receive a mutable
/// reference to the protected data. Internally, these methods acquire the
/// lock guard, dereference it to obtain the data reference, and pass this
/// reference to the user-provided closure. This design pattern:
///
/// - Hides guard lifetime complexity from users
/// - Ensures locks are automatically released when closures return
/// - Provides a uniform API across different lock implementations
/// - Enables writing generic code over various lock types
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
pub trait Lock<T: ?Sized> {
    /// Acquires the lock and executes a closure within the locked context
    ///
    /// This method blocks the current thread until the lock can be
    /// acquired, then executes the provided closure with mutable access
    /// to the protected data. The lock is automatically released when
    /// the closure returns.
    ///
    /// # How It Works
    ///
    /// 1. Acquires the lock guard (e.g., `MutexGuard`)
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
    /// Returns the result produced by the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{Lock, ArcMutex};
    ///
    /// let lock = ArcMutex::new(42);
    /// let result = lock.with_lock(|value| {
    ///     *value += 1;
    ///     *value
    /// });
    /// assert_eq!(result, 43);
    /// ```
    fn with_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;

    /// Attempts to acquire the lock without blocking
    ///
    /// This method tries to acquire the lock immediately. If the lock
    /// is currently held by another thread, it returns `None` without
    /// blocking. Otherwise, it executes the closure and returns `Some`
    /// containing the result.
    ///
    /// # How It Works
    ///
    /// 1. Attempts to acquire the lock guard without blocking
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
    /// * `None` - If the lock is currently held by another thread
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use prism3_rust_concurrent::lock::{Lock, ArcMutex};
    ///
    /// let lock = ArcMutex::new(42);
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

/// Synchronous mutex implementation of the Lock trait
///
/// This implementation uses the standard library's `Mutex` type to
/// provide a synchronous lock.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized> Lock<T> for Mutex<T> {
    fn with_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock().unwrap();
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

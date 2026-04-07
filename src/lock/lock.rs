/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Lock Trait
//!
//! Defines an unified synchronous lock abstraction that supports acquiring locks
//! and executing operations within the locked context. This trait allows locks to be
//! used in a generic way through closures, avoiding the complexity of
//! explicitly managing lock guards and their lifetimes.
//!
//! # Author
//!
//! Haixing Hu
use std::sync::{
    Mutex,
    RwLock,
};

use parking_lot::Mutex as ParkingLotMutex;

/// Unified synchronous lock trait
///
/// Provides a unified interface for different types of synchronous locks,
/// supporting both read and write operations. This trait allows locks to be
/// used in a generic way through closures, avoiding the complexity of
/// explicitly managing lock guards and their lifetimes.
///
/// # Design Philosophy
///
/// This trait unifies both exclusive locks (like `Mutex`) and read-write
/// locks (like `RwLock`) under a single interface. The key insight is that
/// all locks can be viewed as supporting two operations:
///
/// - **Read operations**: Provide immutable access (`&T`) to the data
/// - **Write operations**: Provide mutable access (`&mut T`) to the data
///
/// For exclusive locks (Mutex), both read and write operations acquire the
/// same exclusive lock, but the API clearly indicates the intended usage.
/// For read-write locks (RwLock), read operations use shared locks while
/// write operations use exclusive locks.
///
/// This design enables:
/// - Unified API across different lock types
/// - Clear semantic distinction between read and write operations
/// - Generic code that works with any lock type
/// - Performance optimization through appropriate lock selection
///
/// # Performance Characteristics
///
/// Different lock implementations have different performance characteristics:
///
/// ## Mutex-based locks (ArcMutex, Mutex)
/// - `read`: Acquires exclusive lock, same performance as write
/// - `write`: Acquires exclusive lock, same performance as read
/// - **Use case**: When you need exclusive access or don't know access patterns
///
/// ## RwLock-based locks (ArcRwLock, RwLock)
/// - `read`: Acquires shared lock, allows concurrent readers
/// - `write`: Acquires exclusive lock, blocks all other operations
/// - **Use case**: Read-heavy workloads where multiple readers can proceed
///   concurrently
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
pub trait Lock<T: ?Sized> {
    /// Acquires a read lock and executes a closure
    ///
    /// This method provides immutable access to the protected data. It ensures
    /// proper memory barriers are established:
    ///
    /// - **Acquire semantics**: Ensures that all subsequent memory operations
    ///   see the effects of previous operations released by the lock release.
    /// - **Release semantics**: Ensures that all previous memory operations are
    ///   visible to subsequent lock acquisitions when the lock is released.
    ///
    /// For exclusive locks (Mutex), this acquires the same exclusive lock as
    /// write operations. For read-write locks (RwLock), this acquires a
    /// shared lock allowing concurrent readers.
    ///
    /// # Use Cases
    ///
    /// - **Data inspection**: Reading values, checking state, validation
    /// - **Read-only operations**: Computing derived values, formatting output
    /// - **Condition checking**: Evaluating predicates without modification
    /// - **Logging and debugging**: Accessing data for diagnostic purposes
    ///
    /// # Performance Notes
    ///
    /// - **Mutex-based locks**: Same performance as write operations
    /// - **RwLock-based locks**: Allows concurrent readers, better for
    ///   read-heavy workloads
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives an immutable reference (`&T`) to the
    ///   protected data
    ///
    /// # Returns
    ///
    /// Returns the result produced by the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{Lock, ArcRwLock};
    ///
    /// let lock = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// // Read operation - allows concurrent readers with RwLock
    /// let len = lock.read(|data| data.len());
    /// assert_eq!(len, 3);
    ///
    /// // Multiple concurrent readers possible with RwLock
    /// let sum = lock.read(|data| data.iter().sum::<i32>());
    /// assert_eq!(sum, 6);
    /// ```
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// Acquires a write lock and executes a closure
    ///
    /// This method provides mutable access to the protected data. It ensures
    /// proper memory barriers are established:
    ///
    /// - **Acquire semantics**: Ensures that all subsequent memory operations
    ///   see the effects of previous operations released by the lock release.
    /// - **Release semantics**: Ensures that all previous memory operations are
    ///   visible to subsequent lock acquisitions when the lock is released.
    ///
    /// For all lock types, this acquires an exclusive lock that blocks all
    /// other operations until the closure completes.
    ///
    /// # Use Cases
    ///
    /// - **Data modification**: Updating values, adding/removing elements
    /// - **State changes**: Transitioning between different states
    /// - **Initialization**: Setting up data structures
    /// - **Cleanup operations**: Releasing resources, resetting state
    ///
    /// # Performance Notes
    ///
    /// - **All lock types**: Exclusive access, blocks all other operations
    /// - **RwLock advantage**: Only blocks during actual writes, not reads
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference (`&mut T`) to the
    ///   protected data
    ///
    /// # Returns
    ///
    /// Returns the result produced by the closure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{Lock, ArcRwLock};
    ///
    /// let lock = ArcRwLock::new(vec![1, 2, 3]);
    ///
    /// // Write operation - exclusive access
    /// lock.write(|data| {
    ///     data.push(4);
    ///     data.sort();
    /// });
    ///
    /// // Verify the changes
    /// let result = lock.read(|data| data.clone());
    /// assert_eq!(result, vec![1, 2, 3, 4]);
    /// ```
    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;

    /// Attempts to acquire a read lock without blocking
    ///
    /// This method tries to acquire a read lock immediately. If the lock
    /// is currently held by another thread in write mode, it returns `None`
    /// without blocking. Otherwise, it executes the closure and returns `Some`
    /// containing the result.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives an immutable reference (`&T`) to the
    ///   protected data if the lock is successfully acquired
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was acquired and closure executed
    /// * `None` - If the lock is currently held in write mode
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{Lock, ArcRwLock};
    ///
    /// let lock = ArcRwLock::new(42);
    /// if let Some(value) = lock.try_read(|data| *data) {
    ///     println!("Got value: {}", value);
    /// } else {
    ///     println!("Lock is busy with write operation");
    /// }
    /// ```
    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R;

    /// Attempts to acquire a write lock without blocking
    ///
    /// This method tries to acquire a write lock immediately. If the lock
    /// is currently held by another thread (in either read or write mode),
    /// it returns `None` without blocking. Otherwise, it executes the closure
    /// and returns `Some` containing the result.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference (`&mut T`) to the
    ///   protected data if the lock is successfully acquired
    ///
    /// # Returns
    ///
    /// * `Some(R)` - If the lock was acquired and closure executed
    /// * `None` - If the lock is currently held by another thread
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use qubit_concurrent::lock::{Lock, ArcMutex};
    ///
    /// let lock = ArcMutex::new(42);
    /// if let Some(result) = lock.try_write(|data| {
    ///     *data += 1;
    ///     *data
    /// }) {
    ///     println!("New value: {}", result);
    /// } else {
    ///     println!("Lock is busy");
    /// }
    /// ```
    fn try_write<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R;
}

/// Synchronous mutex implementation of the Lock trait
///
/// This implementation uses the standard library's `Mutex` type to provide
/// a synchronous lock. Both read and write operations acquire the same
/// exclusive lock, ensuring thread safety at the cost of concurrent access.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized> Lock<T> for Mutex<T> {
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.lock().unwrap();
        f(&*guard)
    }

    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock().unwrap();
        f(&mut *guard)
    }

    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Ok(guard) = self.try_lock() {
            Some(f(&*guard))
        } else {
            None
        }
    }

    fn try_write<R, F>(&self, f: F) -> Option<R>
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

/// Synchronous read-write lock implementation of the Lock trait
///
/// This implementation uses the standard library's `RwLock` type to provide
/// a synchronous read-write lock. Read operations use shared locks allowing
/// concurrent readers, while write operations use exclusive locks that
/// block all other operations.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized> Lock<T> for RwLock<T> {
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read().unwrap();
        f(&*guard)
    }

    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.write().unwrap();
        f(&mut *guard)
    }

    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        if let Ok(guard) = self.try_read() {
            Some(f(&*guard))
        } else {
            None
        }
    }

    fn try_write<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        if let Ok(mut guard) = self.try_write() {
            Some(f(&mut *guard))
        } else {
            None
        }
    }
}

/// High-performance synchronous mutex implementation of the Lock trait
///
/// This implementation uses the `parking_lot` crate's `Mutex` type to provide
/// a high-performance synchronous lock. Both read and write operations acquire
/// the same exclusive lock, ensuring thread safety with better performance
/// than the standard library's Mutex.
///
/// # Type Parameters
///
/// * `T` - The type of data protected by the lock
///
/// # Performance Characteristics
///
/// The parking_lot Mutex is generally faster than std::sync::Mutex due to:
/// - More efficient lock acquisition and release
/// - Better handling of contended locks
/// - Reduced memory overhead
/// - No risk of lock poisoning (panics don't poison the lock)
///
/// # Author
///
/// Haixing Hu
impl<T: ?Sized> Lock<T> for ParkingLotMutex<T> {
    fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.lock();
        f(&*guard)
    }

    fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        f(&mut *guard)
    }

    fn try_read<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        self.try_lock().map(|guard| f(&*guard))
    }

    fn try_write<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        self.try_lock().map(|mut guard| f(&mut *guard))
    }
}

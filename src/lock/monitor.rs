/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Monitor
//!
//! Provides a synchronous monitor built from a mutex and a condition variable.
//! A monitor protects one shared state value and lets threads wait until that
//! state satisfies a predicate.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::{
    Condvar,
    Mutex,
    MutexGuard,
};

/// Shared state protected by a mutex and a condition variable.
///
/// `Monitor` is useful when callers need more than a short critical section.
/// It models the classic monitor pattern: one mutex protects the state, and
/// one condition variable lets threads wait until that state changes. Waiting
/// is always predicate based, so spurious wakeups are handled by rechecking the
/// predicate before the caller's closure runs.
///
/// A poisoned mutex is recovered by taking the inner state. This makes
/// `Monitor` suitable for coordination state that should remain observable
/// after another thread panics while holding the lock.
///
/// # Type Parameters
///
/// * `T` - The state protected by this monitor.
///
/// # Example
///
/// ```rust
/// use std::{
///     sync::Arc,
///     thread,
/// };
///
/// use qubit_concurrent::lock::Monitor;
///
/// let monitor = Arc::new(Monitor::new(false));
/// let waiter_monitor = Arc::clone(&monitor);
///
/// let waiter = thread::spawn(move || {
///     waiter_monitor.wait_until(
///         |ready| *ready,
///         |ready| {
///             *ready = false;
///         },
///     );
/// });
///
/// monitor.write(|ready| {
///     *ready = true;
/// });
/// monitor.notify_all();
///
/// waiter.join().expect("waiter should finish");
/// assert!(!monitor.read(|ready| *ready));
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct Monitor<T> {
    state: Mutex<T>,
    changed: Condvar,
}

impl<T> Monitor<T> {
    /// Creates a monitor protecting the supplied state value.
    ///
    /// # Arguments
    ///
    /// * `state` - Initial state protected by the monitor.
    ///
    /// # Returns
    ///
    /// A monitor initialized with the supplied state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Monitor::new(0_u32);
    /// assert_eq!(monitor.read(|n| *n), 0);
    /// ```
    #[inline]
    pub fn new(state: T) -> Self {
        Self {
            state: Mutex::new(state),
            changed: Condvar::new(),
        }
    }

    /// Acquires the monitor and reads the protected state.
    ///
    /// The closure runs while the mutex is held. Keep the closure short and do
    /// not call code that may block for a long time.
    ///
    /// If the mutex is poisoned, this method recovers the inner state and still
    /// executes the closure.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives an immutable reference to the state.
    ///
    /// # Returns
    ///
    /// The value returned by the closure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Monitor::new(10_i32);
    /// let n = monitor.read(|x| *x);
    /// assert_eq!(n, 10);
    /// ```
    #[inline]
    pub fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.lock_state();
        f(&*guard)
    }

    /// Acquires the monitor and mutates the protected state.
    ///
    /// The closure runs while the mutex is held. This method only changes the
    /// state; callers should explicitly call [`Self::notify_one`] or
    /// [`Self::notify_all`] after changing a condition that waiters may be
    /// observing.
    ///
    /// If the mutex is poisoned, this method recovers the inner state and still
    /// executes the closure.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference to the state.
    ///
    /// # Returns
    ///
    /// The value returned by the closure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Monitor::new(String::new());
    /// let len = monitor.write(|s| {
    ///     s.push_str("hi");
    ///     s.len()
    /// });
    /// assert_eq!(len, 2);
    /// ```
    #[inline]
    pub fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock_state();
        f(&mut *guard)
    }

    /// Waits until the protected state satisfies a predicate, then mutates it.
    ///
    /// The predicate is evaluated while holding the mutex. If it returns
    /// `false`, the current thread blocks on the condition variable and
    /// atomically releases the mutex. After a notification, the mutex is
    /// reacquired and the predicate is evaluated again.
    ///
    /// This method may block indefinitely if no thread changes the state to
    /// satisfy the predicate and sends a notification.
    ///
    /// If the mutex is poisoned before or during the wait, this method recovers
    /// the inner state and continues waiting or executes the closure.
    ///
    /// # Arguments
    ///
    /// * `ready` - Predicate that returns `true` when the state is ready.
    /// * `f` - Closure that receives mutable access to the ready state.
    ///
    /// # Returns
    ///
    /// The value returned by `f` after the predicate has become `true`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Arc::new(Monitor::new(false));
    /// let waiter = {
    ///     let m = Arc::clone(&monitor);
    ///     thread::spawn(move || {
    ///         m.wait_until(
    ///             |ready| *ready,
    ///             |ready| {
    ///                 *ready = false;
    ///             },
    ///         );
    ///     })
    /// };
    ///
    /// monitor.write(|ready| {
    ///     *ready = true;
    /// });
    /// monitor.notify_all();
    /// waiter.join().expect("waiter should finish");
    /// assert!(!monitor.read(|ready| *ready));
    /// ```
    #[inline]
    pub fn wait_until<R, P, F>(&self, mut ready: P, f: F) -> R
    where
        P: FnMut(&T) -> bool,
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock_state();
        while !ready(&*guard) {
            guard = self.wait_state(guard);
        }
        f(&mut *guard)
    }

    /// Wakes one thread waiting in [`Self::wait_until`].
    ///
    /// Notifications do not carry state by themselves. A waiting thread only
    /// proceeds when its predicate observes the protected state as ready.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Arc::new(Monitor::new(0_u32));
    /// let waiter = {
    ///     let m = Arc::clone(&monitor);
    ///     thread::spawn(move || {
    ///         m.wait_until(|n| *n > 0, |n| *n -= 1);
    ///     })
    /// };
    ///
    /// monitor.write(|n| *n = 1);
    /// monitor.notify_one();
    /// waiter.join().expect("waiter should finish");
    /// ```
    #[inline]
    pub fn notify_one(&self) {
        self.changed.notify_one();
    }

    /// Wakes all threads waiting in [`Self::wait_until`].
    ///
    /// Notifications do not carry state by themselves. Every awakened thread
    /// rechecks its predicate before continuing.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::thread;
    ///
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor = Arc::new(Monitor::new(false));
    /// let mut handles = Vec::new();
    /// for _ in 0..2 {
    ///     let m = Arc::clone(&monitor);
    ///     handles.push(thread::spawn(move || {
    ///         m.wait_until(|ready| *ready, |_| ());
    ///     }));
    /// }
    ///
    /// monitor.write(|ready| *ready = true);
    /// monitor.notify_all();
    /// for h in handles {
    ///     h.join().expect("waiter should finish");
    /// }
    /// ```
    #[inline]
    pub fn notify_all(&self) {
        self.changed.notify_all();
    }

    /// Acquires the state mutex and recovers from poisoning.
    fn lock_state(&self) -> MutexGuard<'_, T> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Waits on the condition variable and recovers from poisoning.
    fn wait_state<'a>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        self.changed
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<T: Default> Default for Monitor<T> {
    /// Creates a monitor containing `T::default()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use qubit_concurrent::lock::Monitor;
    ///
    /// let monitor: Monitor<String> = Monitor::default();
    /// assert!(monitor.read(|s| s.is_empty()));
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

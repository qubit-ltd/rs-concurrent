/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Arc Monitor
//!
//! Provides an Arc-wrapped synchronous monitor for condition-based state
//! coordination across threads.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::Arc;

use super::Monitor;

/// Arc-wrapped monitor for shared condition-based state coordination.
///
/// `ArcMonitor` stores a [`Monitor`] behind an [`Arc`], so callers can clone
/// the monitor handle directly without writing `Arc::new(Monitor::new(...))`.
/// It preserves the same predicate-based waiting and poison recovery semantics
/// as [`Monitor`].
///
/// # Type Parameters
///
/// * `T` - The state protected by this monitor.
///
/// # Example
///
/// ```rust
/// use std::thread;
///
/// use qubit_concurrent::lock::ArcMonitor;
///
/// let monitor = ArcMonitor::new(false);
/// let waiter_monitor = monitor.clone();
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
pub struct ArcMonitor<T> {
    /// Shared monitor instance.
    inner: Arc<Monitor<T>>,
}

impl<T> ArcMonitor<T> {
    /// Creates an Arc-wrapped monitor protecting the supplied state value.
    ///
    /// # Arguments
    ///
    /// * `state` - Initial state protected by the monitor.
    ///
    /// # Returns
    ///
    /// A cloneable monitor handle initialized with the supplied state.
    #[inline]
    pub fn new(state: T) -> Self {
        Self {
            inner: Arc::new(Monitor::new(state)),
        }
    }

    /// Acquires the monitor and reads the protected state.
    ///
    /// This delegates to [`Monitor::read`]. The closure runs while the monitor
    /// mutex is held, so keep it short and avoid long blocking work.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives an immutable reference to the state.
    ///
    /// # Returns
    ///
    /// The value returned by `f`.
    #[inline]
    pub fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.inner.read(f)
    }

    /// Acquires the monitor and mutates the protected state.
    ///
    /// This delegates to [`Monitor::write`]. Callers should explicitly invoke
    /// [`Self::notify_one`] or [`Self::notify_all`] after changing state that a
    /// waiting thread may observe.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure that receives a mutable reference to the state.
    ///
    /// # Returns
    ///
    /// The value returned by `f`.
    #[inline]
    pub fn write<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        self.inner.write(f)
    }

    /// Waits until the protected state satisfies a predicate, then mutates it.
    ///
    /// This delegates to [`Monitor::wait_until`]. It may block indefinitely if
    /// no thread changes the state to satisfy the predicate and sends a
    /// notification.
    ///
    /// # Arguments
    ///
    /// * `ready` - Predicate that returns `true` when the state is ready.
    /// * `f` - Closure that receives mutable access to the ready state.
    ///
    /// # Returns
    ///
    /// The value returned by `f`.
    #[inline]
    pub fn wait_until<R, P, F>(&self, ready: P, f: F) -> R
    where
        P: FnMut(&T) -> bool,
        F: FnOnce(&mut T) -> R,
    {
        self.inner.wait_until(ready, f)
    }

    /// Wakes one thread waiting in [`Self::wait_until`].
    ///
    /// Notifications do not carry state by themselves. A waiting thread only
    /// proceeds when its predicate observes the protected state as ready.
    #[inline]
    pub fn notify_one(&self) {
        self.inner.notify_one();
    }

    /// Wakes all threads waiting in [`Self::wait_until`].
    ///
    /// Every awakened thread rechecks its predicate before continuing.
    #[inline]
    pub fn notify_all(&self) {
        self.inner.notify_all();
    }
}

impl<T: Default> Default for ArcMonitor<T> {
    /// Creates an Arc-wrapped monitor containing `T::default()`.
    ///
    /// # Returns
    ///
    /// A cloneable monitor handle protecting the default value for `T`.
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> Clone for ArcMonitor<T> {
    /// Clones this monitor handle.
    ///
    /// The cloned handle shares the same protected state and condition
    /// variable with the original.
    ///
    /// # Returns
    ///
    /// A new handle sharing the same monitor state.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

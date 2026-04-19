/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Locking Fluent API
//!
//! Provides a fluent API for building and executing a double-checked locking
//! operation.
//!
//! # Author
//!
//! Haixing Hu
use super::execution_builder::{ExecutionBuilder, Initial};
use crate::lock::Lock;

/// The entry point for the fluent API of the double-checked locking
/// executor.
///
/// This struct provides a starting point for building and executing a
/// double-checked locking operation in a chainable and readable way.
///
/// # Features
///
/// - **Fluent API**: Provides a chainable interface for configuration
/// - **Double-Checking**: Checks the condition before and after acquiring
///   the lock to minimize unnecessary lock contention
/// - **Flexible Configuration**: Supports prepare actions, prepare commit and
///   rollback actions, and logging
/// - **Type Safety**: Leverages Rust's type system to ensure thread safety
///
/// # Use Cases
///
/// - **Lazy Initialization**: Lazily initialize shared resources in
///   multi-threaded environments
/// - **Conditional Updates**: Update shared data only when specific
///   conditions are met
/// - **Performance Optimization**: Reduce unnecessary lock acquisitions to
///   improve concurrency
/// - **Transactional Operations**: Operations that require prepare lifecycle
///   hooks around a double-checked task
///
/// # Prepare Lifecycle
///
/// A prepare action runs after the first condition check and before acquiring
/// the lock. If it succeeds, the framework owns only the prepare lifecycle:
/// it calls [`ExecutionBuilder::rollback_prepare`] after a failed second check
/// or task error, and [`ExecutionBuilder::commit_prepare`] after task success.
/// These callbacks run after the lock has been released.
///
/// The task closure is responsible for its own transactional behavior. If the
/// task performs multiple steps, mutates protected data, or touches external
/// systems, it must handle its own rollback, cleanup, and commit boundaries.
/// The framework only observes whether the task returned `Ok` or `Err`; it
/// cannot know which task steps have completed.
///
/// # Examples
///
/// ## Simple Read Operation
///
/// Keep a cheap `frozen` flag alongside the mutex. Reading an authoritative
/// balance may be expensive (ledger reconciliation, remote call, etc.); model
/// that as a `read_balance` helper. Use [`ExecutionBuilder::when`] so that when
/// the account is **frozen** you skip both lock acquisition and `read_balance`
/// entirely. The executor still **re-checks** `!frozen` after taking the lock,
/// so if another thread freezes the account in between, you do not pay the
/// costly read on a path that should be rejected.
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// fn read_balance(latest: &i32) -> Result<i32, std::io::Error> {
///     // Expensive work: checksum, cross-service reconciliation, etc.
///     Ok(*latest)
/// }
///
/// let balance = ArcMutex::new(1_000);
/// let frozen = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&balance)
///     .when({
///         let frozen = frozen.clone();
///         move || !frozen.load(Ordering::Acquire)
///     })
///     .call(|cached: &i32| read_balance(cached))
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), 1_000);
/// ```
///
/// ## Simple Write Operation
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let draft = ArcMutex::new(String::from("hello"));
/// // Only allow edits while the document is still a draft (not published).
/// let published = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&draft)
///     .when({
///         let published = published.clone();
///         move || !published.load(Ordering::Acquire)
///     })
///     .call_mut(|content: &mut String| {
///         content.push_str(" world");
///         Ok::<(), std::io::Error>(())
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(draft.read(|s| s.clone()), "hello world");
/// ```
///
/// ## Conditional Execution
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicU32, Ordering};
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let total = ArcMutex::new(0);
/// // Fast path: enforce a daily issuance cap without locking every time.
/// let issued_today = Arc::new(AtomicU32::new(99));
///
/// let result = DoubleCheckedLock::on(&total)
///     .when({
///         let issued_today = issued_today.clone();
///         move || issued_today.load(Ordering::Relaxed) < 100
///     })
///     .call_mut({
///         let issued_today = issued_today.clone();
///         move |n: &mut i32| {
///             issued_today.fetch_add(1, Ordering::Relaxed);
///             *n += 1;
///             Ok::<i32, std::io::Error>(*n)
///         }
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), 1);
/// assert_eq!(issued_today.load(Ordering::Relaxed), 100);
/// ```
///
/// ## Error Handling
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let cart_total = ArcMutex::new(5);
/// // Checkout path: only try validation while the store accepts orders.
/// let checkout_open = Arc::new(AtomicBool::new(true));
///
/// let result = DoubleCheckedLock::on(&cart_total)
///     .when({
///         let checkout_open = checkout_open.clone();
///         move || checkout_open.load(Ordering::Acquire)
///     })
///     .call_mut(|value: &mut i32| {
///         if *value < 10 {
///             Err(std::io::Error::new(
///                 std::io::ErrorKind::Other,
///                 "Minimum order is 10"
///             ))
///         } else {
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .get_result();
///
/// assert!(result.is_failed());
/// ```
///
/// ## Optional logging
///
/// Call [`ExecutionBuilder::logger`] before [`ExecutionBuilder::when`] to emit
/// a message at the given [`log::Level`] when the test condition is **false**
/// on the fast path (outside the lock) or the slow path (inside the lock).
///
/// ```rust
/// use log::Level;
/// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
///
/// let data = ArcMutex::new(42);
///
/// let result = DoubleCheckedLock::on(&data)
///     .logger(Level::Info, "condition not met; skipping read")
///     .when(|| false)
///     .call(|value: &i32| Ok::<i32, std::io::Error>(*value))
///     .get_result();
///
/// assert!(result.is_unmet());
/// ```
///
/// After the first [`ExecutionBuilder::logger`] call, the builder is in the
/// configuring state; you may call [`ExecutionBuilder::logger`] again to
/// override level or message before [`ExecutionBuilder::when`].
///
/// ## Basic Usage - Lazy Initialization
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{
///     DoubleCheckedLock,
///     ArcMutex,
/// };
///
/// fn initialize_resource() -> Result<String, std::io::Error> {
///     // e.g. read config, open handles, warm caches — real work lives here.
///     Ok("Initialized Resource".to_string())
/// }
///
/// // Lazy initialization of a resource
/// let resource = ArcMutex::new(None::<String>);
/// let initialized = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&resource)
///     .when({
///         let initialized = initialized.clone();
///         move || !initialized.load(Ordering::Acquire)
///     })
///     .call_mut({
///         let initialized = initialized.clone();
///         move |data: &mut Option<String>| {
///             *data = Some(initialize_resource()?);
///             initialized.store(true, Ordering::Release);
///             Ok::<(), std::io::Error>(())
///         }
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert!(initialized.load(Ordering::Acquire));
/// ```
///
/// ## Database Transaction with Prepare Lifecycle
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{
///     double_checked::DoubleCheckedLock,
///     lock::ArcMutex,
/// };
///
/// fn begin_transfer_session() -> Result<(), std::io::Error> {
///     // e.g. acquire a slot from the pool driver
///     Ok(())
/// }
///
/// fn open_connection(connection_opened: &AtomicBool) -> Result<(), std::io::Error> {
///     begin_transfer_session()?;
///     connection_opened.store(true, Ordering::Release);
///     Ok(())
/// }
///
/// fn teardown_prepared_connection(connection_opened: &AtomicBool) -> Result<(), std::io::Error> {
///     connection_opened.store(false, Ordering::Release);
///     Ok(())
/// }
///
/// fn rollback_prepare_state(
///     connection_opened: &AtomicBool,
///     transaction_rolled_back: &AtomicBool,
/// ) -> Result<(), std::io::Error> {
///     teardown_prepared_connection(connection_opened)?;
///     transaction_rolled_back.store(true, Ordering::Release);
///     Ok(())
/// }
///
/// fn finalize_prepare_commit(transaction_committed: &AtomicBool) -> Result<(), std::io::Error> {
///     // e.g. flush session metadata after the guarded mutation succeeds
///     transaction_committed.store(true, Ordering::Release);
///     Ok(())
/// }
///
/// fn apply_withdrawal(balance: &mut i32, amount: i32) -> Result<i32, std::io::Error> {
///     if *balance < amount {
///         return Err(std::io::Error::new(
///             std::io::ErrorKind::Other,
///             "Insufficient balance",
///         ));
///     }
///     *balance -= amount;
///     Ok(*balance)
/// }
///
/// fn execute_transfer_task(
///     balance: &mut i32,
///     withdrawal: i32,
///     transaction_active: &AtomicBool,
/// ) -> Result<i32, std::io::Error> {
///     let new_balance = apply_withdrawal(balance, withdrawal)?;
///     transaction_active.store(true, Ordering::Release);
///     Ok(new_balance)
/// }
///
/// let account_balance = ArcMutex::new(1000);
/// let transaction_active = Arc::new(AtomicBool::new(false));
/// let connection_opened = Arc::new(AtomicBool::new(false));
/// let transaction_committed = Arc::new(AtomicBool::new(false));
/// let transaction_rolled_back = Arc::new(AtomicBool::new(false));
///
/// let result = DoubleCheckedLock::on(&account_balance)
///     .when({
///         let transaction_active = transaction_active.clone();
///         move || !transaction_active.load(Ordering::Acquire)
///     })
///     .prepare({
///         let connection_opened = connection_opened.clone();
///         move || open_connection(connection_opened.as_ref())
///     })
///     .rollback_prepare({
///         let connection_opened = connection_opened.clone();
///         let transaction_rolled_back = transaction_rolled_back.clone();
///         move || {
///             rollback_prepare_state(connection_opened.as_ref(), transaction_rolled_back.as_ref())
///         }
///     })
///     .commit_prepare({
///         let transaction_committed = transaction_committed.clone();
///         move || finalize_prepare_commit(transaction_committed.as_ref())
///     })
///     .call_mut({
///         let transaction_active = transaction_active.clone();
///         move |balance: &mut i32| execute_transfer_task(balance, 100, transaction_active.as_ref())
///     })
///     .get_result();
///
/// assert!(result.is_success());
/// assert!(connection_opened.load(Ordering::Acquire));
/// assert!(transaction_committed.load(Ordering::Acquire));
/// assert!(!transaction_rolled_back.load(Ordering::Acquire));
/// ```
///
/// ## Cache Lookup with Read Lock
///
/// A read lock holds an optional **materialized** fare row. A cheap
/// `snapshot_matches_origin` flag tracks whether upstream still considers this
/// slot current (set to `false` on pub/sub invalidation without touching the
/// RwLock). [`ExecutionBuilder::when`] skips the read lock when the snapshot is
/// already stale so you do not pay `RwLock::read` or downstream formatting on
/// a known-dead entry; the second check under the lock catches races where the
/// row was invalidated after the fast path.
///
/// ```rust
/// use std::sync::Arc;
/// use std::sync::atomic::{AtomicBool, Ordering};
/// use qubit_concurrent::{
///     double_checked::DoubleCheckedLock,
///     lock::ArcRwLock,
/// };
///
/// fn price_with_fees_and_tax(base_cents: i32) -> i32 {
///     (base_cents * 108) / 100
/// }
///
/// fn materialize_quote(cached_base: &Option<i32>) -> Result<i32, std::io::Error> {
///     let base = cached_base.as_ref().ok_or_else(|| {
///         std::io::Error::new(std::io::ErrorKind::NotFound, "cache slot empty")
///     })?;
///     Ok(price_with_fees_and_tax(*base))
/// }
///
/// let cached_fare_cents = ArcRwLock::new(Some(499));
/// let snapshot_matches_origin = Arc::new(AtomicBool::new(true));
///
/// let result = DoubleCheckedLock::on(&cached_fare_cents)
///     .when({
///         let snapshot_matches_origin = snapshot_matches_origin.clone();
///         move || snapshot_matches_origin.load(Ordering::Acquire)
///     })
///     .call(|row: &Option<i32>| materialize_quote(row))
///     .get_result();
///
/// assert!(result.is_success());
/// assert_eq!(result.unwrap(), (499 * 108) / 100);
/// ```
///
/// # Author
///
/// Haixing Hu
pub struct DoubleCheckedLock;

impl DoubleCheckedLock {
    /// Starts a double-checked locking operation on the specified lock.
    ///
    /// This is the entry point for all double-checked locking operations.
    /// It returns an `ExecutionBuilder` that allows you to configure the
    /// operation with conditions, actions, and error handling.
    ///
    /// # Examples
    ///
    /// ## Basic read operation
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// fn read_balance(latest: &i32) -> Result<i32, std::io::Error> {
    ///     Ok(*latest)
    /// }
    ///
    /// let balance = ArcMutex::new(42);
    /// let frozen = Arc::new(AtomicBool::new(false));
    ///
    /// let result = DoubleCheckedLock::on(&balance)
    ///     .when({
    ///         let frozen = frozen.clone();
    ///         move || !frozen.load(Ordering::Acquire)
    ///     })
    ///     .call(|cached: &i32| read_balance(cached))
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(result.unwrap(), 42);
    /// ```
    ///
    /// ## Basic write operation
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let draft = ArcMutex::new(0);
    /// let published = Arc::new(AtomicBool::new(false));
    ///
    /// let result = DoubleCheckedLock::on(&draft)
    ///     .when({
    ///         let published = published.clone();
    ///         move || !published.load(Ordering::Acquire)
    ///     })
    ///     .call_mut(|value: &mut i32| {
    ///         *value = 100;
    ///         Ok::<(), std::io::Error>(())
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(draft.read(|v| *v), 100);
    /// ```
    ///
    /// ## Conditional execution
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use std::sync::atomic::{AtomicU32, Ordering};
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let total = ArcMutex::new(0);
    /// let issued_today = Arc::new(AtomicU32::new(99));
    ///
    /// let result = DoubleCheckedLock::on(&total)
    ///     .when({
    ///         let issued_today = issued_today.clone();
    ///         move || issued_today.load(Ordering::Relaxed) < 100
    ///     })
    ///     .call_mut({
    ///         let issued_today = issued_today.clone();
    ///         move |n: &mut i32| {
    ///             issued_today.fetch_add(1, Ordering::Relaxed);
    ///             *n += 1;
    ///             Ok::<i32, std::io::Error>(*n)
    ///         }
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_success());
    /// assert_eq!(result.unwrap(), 1);
    /// ```
    ///
    /// ## Optional logging
    ///
    /// ```rust
    /// use log::Level;
    /// use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};
    ///
    /// let data = ArcMutex::new(0);
    ///
    /// let result = DoubleCheckedLock::on(&data)
    ///     .logger(Level::Debug, "skip: tester returned false")
    ///     .when(|| false)
    ///     .call_mut(|value: &mut i32| {
    ///         *value += 1;
    ///         Ok::<i32, std::io::Error>(*value)
    ///     })
    ///     .get_result();
    ///
    /// assert!(result.is_unmet());
    /// assert_eq!(data.read(|v| *v), 0);
    /// ```
    ///
    /// # Type Parameters
    ///
    /// * `'a` - The lifetime parameter for the lock reference
    /// * `L` - The lock type that must implement the `Lock<T>` trait
    /// * `T` - The type of data protected by the lock
    ///
    /// # Parameters
    ///
    /// * `lock` - A reference to an object that implements the `Lock<T>` trait
    ///
    /// # Returns
    ///
    /// Returns an `ExecutionBuilder` instance in Initial state to configure
    /// and execute the operation
    #[inline]
    pub fn on<'a, L, T>(lock: &'a L) -> ExecutionBuilder<'a, L, T, Initial>
    where
        L: Lock<T>,
    {
        ExecutionBuilder::new(lock)
    }
}

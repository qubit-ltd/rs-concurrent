# Qubit Concurrent

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-concurrent.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-concurrent)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-concurrent/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-concurrent?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-concurrent.svg?color=blue)](https://crates.io/crates/qubit-concurrent)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

> [!IMPORTANT]
> `qubit-concurrent` has been split into smaller, focused crates. New code
> should depend on those crates directly instead of adding `qubit-concurrent`.
> This repository is kept as a legacy reference for users migrating from the
> old combined package.
>
> | Need | Use |
> |------|-----|
> | Lock wrappers, async locks, monitor primitives | `qubit-lock` |
> | Double-checked locking executor | `qubit-dcl` |
> | Core executor traits, task handles, direct/thread-per-task executors | `qubit-executor` |
> | Fixed and dynamic thread pools | `qubit-thread-pool` |
> | Rayon-backed executor service | `qubit-rayon-executor` |
> | Tokio-backed executor service | `qubit-tokio-executor` |
> | Aggregated blocking / CPU / Tokio execution services | `qubit-execution-services` |
> | Higher-level task execution service with task IDs and status tracking | `qubit-task` |

A comprehensive Rust concurrent utilities library providing thread-safe lock wrappers and synchronization primitives for the Qubit Rust libraries.

## Overview

Qubit Concurrent provides easy-to-use wrappers around both synchronous and asynchronous locks, offering a unified interface for concurrent programming in Rust. All lock types have `Arc` built-in internally, so you can clone and share them across threads or tasks directly without additional wrapping. The library provides convenient helper methods for common locking patterns with a closure-based API that ensures proper lock management.

## Features

### 🔒 **Synchronous Locks**
- **ArcMutex**: Thread-safe mutual exclusion lock wrapper with `Arc` integration
- **ArcRwLock**: Thread-safe read-write lock wrapper supporting multiple concurrent readers
- **Monitor / ArcMonitor**: Condition-based state coordination backed by `Mutex` and `Condvar`, with an Arc-integrated wrapper for shared use
- **Convenient API**: `read`/`write` and `try_read`/`try_write` methods for cleaner lock handling
- **Automatic RAII**: Ensures proper lock release through scope-based management

### 🚀 **Asynchronous Locks**
- **ArcAsyncMutex**: Async-aware mutual exclusion lock for use with Tokio runtime
- **ArcAsyncRwLock**: Async-aware read-write lock supporting concurrent async reads
- **Non-blocking**: Designed for async contexts without blocking threads
- **Tokio Integration**: Built on top of Tokio's synchronization primitives

### ⚙️ **Task Execution**
- **Executor**: Execution strategy trait under `task::executor`, with `execute` for `Runnable` tasks and `call` for `Callable` tasks
- **ExecutorService**: Managed task service under `task::service`, with `submit`, `submit_callable`, and graceful shutdown support
- **BlockingExecutorService / ThreadPool**: Managed service for synchronous tasks that may block an OS thread
- **RayonExecutorService**: Managed service for CPU-bound synchronous tasks backed by Rayon
- **TokioBlockingExecutorService**: Tokio-backed blocking service using `spawn_blocking`
- **TokioIoExecutorService**: Tokio-backed async service for `Future<Output = Result<_, _>>` tasks
- **ExecutionServices**: Unified facade that routes work to blocking, CPU, Tokio blocking, and Tokio IO domains
- **FutureExecutor**: Executor specialization whose execution carrier is a future
- **Runnable / Callable**: Fallible reusable task abstractions provided by `qubit-function`
- **Clear acceptance semantics**: `ExecutorService` acceptance is separate from task success

### 🔁 **Double-checked locking**
- **DoubleCheckedLockExecutor**: Reusable executor for test-outside-lock / retest-inside-lock flows, optional prepare / rollback / commit hooks, and `call` / `execute` / `call_with` / `execute_with` tasks
- **ExecutionResult**: Structured outcomes (success, condition unmet, task error, prepare failures)

### 🎯 **Key Benefits**
- **Clone Support**: All lock wrappers implement `Clone` for easy sharing across threads
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Ergonomic API**: Closure-based lock access eliminates common pitfalls
- **Production Ready**: Battle-tested locking patterns with comprehensive test coverage

## Installation

Do not add `qubit-concurrent` to new projects. Select the split crate that
matches the functionality you need:

```toml
[dependencies]
qubit-lock = "0.3"
qubit-dcl = "0.2"
qubit-executor = "0.1"
qubit-thread-pool = "0.1"
qubit-rayon-executor = "0.1"
qubit-tokio-executor = "0.1"
qubit-execution-services = "0.1"
qubit-task = "0.1"
```

Only add the crates you actually use. For example, a project that only needs
`ArcMutex`, `ArcRwLock`, `ArcAsyncMutex`, or `Monitor` should depend on
`qubit-lock` only.

For existing projects that cannot migrate immediately, the retired combined
crate uses this dependency name:

```toml
[dependencies]
qubit-concurrent = "0.6.0"
```

Do not use this dependency in new code.

## Quick Start

### Synchronous Mutex

```rust
use qubit_concurrent::ArcMutex;
use std::thread;

fn main() {
    let counter = ArcMutex::new(0);
    let mut handles = vec![];

    // Spawn multiple threads that increment the counter
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            counter.write(|value| {
                *value += 1;
            });
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Read final value
    let result = counter.read(|value| *value);
    println!("Final counter: {}", result); // Prints: Final counter: 10
}
```

### Synchronous Read-Write Lock

```rust
use qubit_concurrent::ArcRwLock;

fn main() {
    let data = ArcRwLock::new(vec![1, 2, 3]);

    // Multiple concurrent reads
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = std::thread::spawn(move || {
        let len = data1.read(|v| v.len());
        println!("Length from thread 1: {}", len);
    });

    let handle2 = std::thread::spawn(move || {
        let len = data2.read(|v| v.len());
        println!("Length from thread 2: {}", len);
    });

    // Exclusive write access
    data.write(|v| {
        v.push(4);
        println!("Added element, new length: {}", v.len());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
```

### Asynchronous Mutex

```rust
use qubit_concurrent::ArcAsyncMutex;

#[tokio::main]
async fn main() {
    let counter = ArcAsyncMutex::new(0);
    let mut handles = vec![];

    // Spawn multiple async tasks
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = tokio::spawn(async move {
            counter.write(|value| {
                *value += 1;
            }).await;
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }

    // Read final value
    let result = counter.read(|value| *value).await;
    println!("Final counter: {}", result); // Prints: Final counter: 10
}
```

### Asynchronous Read-Write Lock

```rust
use qubit_concurrent::ArcAsyncRwLock;

#[tokio::main]
async fn main() {
    let data = ArcAsyncRwLock::new(String::from("Hello"));

    // Concurrent async reads
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = tokio::spawn(async move {
        let content = data1.read(|s| s.clone()).await;
        println!("Read from task 1: {}", content);
    });

    let handle2 = tokio::spawn(async move {
        let content = data2.read(|s| s.clone()).await;
        println!("Read from task 2: {}", content);
    });

    // Exclusive async write
    data.write(|s| {
        s.push_str(" World!");
        println!("Updated string: {}", s);
    }).await;

    handle1.await.unwrap();
    handle2.await.unwrap();
}
```

### Try Lock (Non-blocking)

```rust
use qubit_concurrent::ArcMutex;

fn main() {
    let mutex = ArcMutex::new(42);

    // Try to acquire lock without blocking
    match mutex.try_read(|value| *value) {
        Some(v) => println!("Got value: {}", v),
        None => println!("Lock is busy"),
    }
}
```

### Condition-Based Monitor

`Monitor<T>` packages a `std::sync::Mutex<T>` and a `std::sync::Condvar`
as one monitor object. It does not replace those standard primitives with a
different synchronization mechanism; instead, it binds the protected state and
the condition variable together so callers do not have to store and match two
separate fields manually.

Use `read` and `write` for short critical sections. Use `wait_while` or
`wait_until` for the common guarded-suspension pattern. For more complex
state machines, call `lock` to get a `MonitorGuard`, then call
`MonitorGuard::wait` or `MonitorGuard::wait_timeout` inside your own loop.

```rust
use std::thread;

use qubit_concurrent::lock::ArcMonitor;

fn main() {
    let monitor = ArcMonitor::new(Vec::<String>::new());
    let worker_monitor = monitor.clone();

    let worker = thread::spawn(move || {
        worker_monitor.wait_until(
            |messages| !messages.is_empty(),
            |messages| messages.pop().expect("message should be ready"),
        )
    });

    monitor.write(|messages| {
        messages.push("ready".to_string());
    });
    monitor.notify_one();

    assert_eq!(
        worker.join().expect("worker should finish"),
        "ready",
    );
}
```

The guard API keeps the same shape as the standard `Condvar` loop, but the
guard already knows which monitor it belongs to:

```rust
use std::{
    thread,
    time::Duration,
};

use qubit_concurrent::lock::{ArcMonitor, WaitTimeoutStatus};

fn main() {
    let monitor = ArcMonitor::new(Vec::<String>::new());
    let worker_monitor = monitor.clone();

    let worker = thread::spawn(move || {
        let mut messages = worker_monitor.lock();
        while messages.is_empty() {
            let (next_messages, status) = messages.wait_timeout(Duration::from_secs(1));
            messages = next_messages;
            if status == WaitTimeoutStatus::TimedOut && messages.is_empty() {
                return None;
            }
        }
        messages.pop()
    });

    monitor.write(|messages| {
        messages.push("ready".to_string());
    });
    monitor.notify_one();

    assert_eq!(
        worker.join().expect("worker should finish"),
        Some("ready".to_string()),
    );
}
```

### Double-checked locking

Skip lock acquisition and expensive reads when a cheap flag already rules them out (for example a **frozen** account). The same predicate is evaluated again under the lock so a race where the account freezes between checks does not run the heavy `read_balance` work.

An optional **`prepare`** action runs after the first condition check succeeds and before the lock is taken. If `prepare` was configured and completed successfully, then after the lock is released: **`commit_prepare`** runs when the task succeeds, and **`rollback_prepare`** runs when the second check fails or the task fails.

Logging goes through the [`log`](https://docs.rs/log/latest/log/) facade; the `log_*` builder methods only emit lines when those events occur. To see output, install a logger implementation in your application (for example depend on `env_logger` in the binary crate and call `env_logger::init()`). Without a backend, logging is a no-op or discarded by the facade.

```rust
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use log::Level;
use qubit_concurrent::{
    ArcMutex,
    DoubleCheckedLockExecutor,
    double_checked::{ExecutionResult, ExecutorError},
    lock::Lock,
};

fn read_balance(latest: &i32) -> Result<i32, io::Error> {
    // Expensive: ledger reconciliation, remote validation, etc.
    Ok(*latest)
}

fn main() {
    let balance = ArcMutex::new(1_000_i32);
    let frozen = Arc::new(AtomicBool::new(false));

    // Counters only illustrate prepare / commit / rollback ordering (replace with reservations, audit writes, etc.)
    let prepare_count = Arc::new(AtomicUsize::new(0));
    let commit_count = Arc::new(AtomicUsize::new(0));
    let rollback_count = Arc::new(AtomicUsize::new(0));

    let executor = DoubleCheckedLockExecutor::builder()
        // Logging: chain before or after .on / .when (grouped here at the start)
        .log_unmet_condition(Level::Warn, "double-check: condition not met")
        .log_prepare_failure(Level::Error, "prepare (before lock) failed")
        .log_prepare_commit_failure(Level::Error, "prepare commit failed")
        .log_prepare_rollback_failure(Level::Error, "prepare rollback failed")
        .on(balance.clone())
        .when({
            let frozen = frozen.clone();
            move || !frozen.load(Ordering::Acquire)
        })
        .prepare({
            let prepare_count = prepare_count.clone();
            move || {
                prepare_count.fetch_add(1, Ordering::AcqRel);
                Ok::<(), io::Error>(())
            }
        })
        .rollback_prepare({
            let rollback_count = rollback_count.clone();
            move || {
                rollback_count.fetch_add(1, Ordering::AcqRel);
                Ok::<(), io::Error>(())
            }
        })
        .commit_prepare({
            let commit_count = commit_count.clone();
            move || {
                commit_count.fetch_add(1, Ordering::AcqRel);
                Ok::<(), io::Error>(())
            }
        })
        .build();

    // Success: prepare → task ok → commit_prepare
    let ok = executor
        .call_with(|latest: &mut i32| read_balance(latest))
        .get_result();
    assert!(matches!(ok, ExecutionResult::Success(1_000)));
    assert_eq!(prepare_count.load(Ordering::Acquire), 1);
    assert_eq!(commit_count.load(Ordering::Acquire), 1);
    assert_eq!(rollback_count.load(Ordering::Acquire), 0);

    // Task error after successful prepare → rollback_prepare; result remains the task failure
    let fail = executor
        .call_with(|_: &mut i32| Err::<i32, _>(io::Error::other("ledger mismatch")))
        .get_result();
    assert!(matches!(
        fail,
        ExecutionResult::Failed(ExecutorError::TaskFailed(_))
    ));
    assert_eq!(prepare_count.load(Ordering::Acquire), 2);
    assert_eq!(commit_count.load(Ordering::Acquire), 1);
    assert_eq!(rollback_count.load(Ordering::Acquire), 1);

    // Predicate false up front: no lock, no task, no prepare / commit / rollback
    frozen.store(true, Ordering::Release);
    let skipped = executor
        .call_with(|latest: &mut i32| read_balance(latest))
        .get_result();
    assert!(matches!(skipped, ExecutionResult::ConditionNotMet));
    assert_eq!(prepare_count.load(Ordering::Acquire), 2);
}
```

### Task Executor

```rust
use qubit_concurrent::task::{
    executor::{DirectExecutor, Executor, TokioExecutor},
    service::{
        BlockingExecutorService,
        ExecutionServices,
        ExecutorService,
        RayonExecutorService,
        ThreadPerTaskExecutorService,
        TokioBlockingExecutorService,
        TokioIoExecutorService,
    },
};
use std::io;

#[tokio::main]
async fn main() {
    let direct = DirectExecutor;
    direct
        .execute(|| Ok::<(), io::Error>(()))
        .expect("direct execution should succeed");

    let tokio_executor = TokioExecutor;
    let value = tokio_executor
        .call(|| Ok::<usize, io::Error>(42))
        .await
        .expect("tokio execution should succeed");
    assert_eq!(value, 42);

    let service = ThreadPerTaskExecutorService::new();
    let handle = service
        .submit(|| Ok::<(), io::Error>(()))
        .expect("submit only reports task acceptance");
    handle.get().expect("task result is observed through the handle");
    service.shutdown();
    service.await_termination().await;

    let blocking = BlockingExecutorService::new(2).expect("blocking service should be created");
    blocking
        .submit(|| Ok::<(), io::Error>(()))
        .expect("blocking service should accept task")
        .get()
        .expect("blocking task should complete");
    blocking.shutdown();
    blocking.await_termination().await;

    let cpu = RayonExecutorService::builder()
        .num_threads(2)
        .build()
        .expect("rayon service should be created");
    let cpu_value = cpu
        .submit_callable(|| Ok::<usize, io::Error>(7 * 6))
        .expect("rayon service should accept task")
        .await
        .expect("cpu task should complete");
    assert_eq!(cpu_value, 42);
    cpu.shutdown();
    cpu.await_termination().await;

    let tokio_blocking = TokioBlockingExecutorService::new();
    let blocking_value = tokio_blocking
        .submit_callable(|| Ok::<usize, io::Error>(40 + 2))
        .expect("tokio blocking service should accept task")
        .await
        .expect("tokio blocking task should complete");
    assert_eq!(blocking_value, 42);
    tokio_blocking.shutdown();
    tokio_blocking.await_termination().await;

    let io_service = TokioIoExecutorService::new();
    let io_value = io_service
        .spawn(async { Ok::<usize, io::Error>(42) })
        .expect("tokio io service should accept future")
        .await
        .expect("tokio io task should complete");
    assert_eq!(io_value, 42);
    io_service.shutdown();
    io_service.await_termination().await;

    let services = ExecutionServices::builder()
        .blocking_pool_size(2)
        .cpu_threads(2)
        .build()
        .expect("execution-services facade should be created");
    let routed_cpu = services
        .submit_cpu_callable(|| Ok::<usize, io::Error>(21 * 2))
        .expect("cpu route should accept task")
        .await
        .expect("cpu route should complete");
    let routed_io = services
        .spawn_io(async { Ok::<usize, io::Error>(42) })
        .expect("io route should accept task")
        .await
        .expect("io route should complete");
    assert_eq!(routed_cpu, 42);
    assert_eq!(routed_io, 42);
    services.shutdown();
    services.await_termination().await;
}
```

## API Reference

### ArcMutex

A synchronous mutual exclusion lock wrapper with `Arc` integration.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.new) - Create a new mutex
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.read) - Acquire read lock and execute closure
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.write) - Acquire write lock and execute closure
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_read) - Try to acquire read lock without blocking
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_write) - Try to acquire write lock without blocking
- [`try_read_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_read_result) - Try to acquire read lock with a detailed error
- [`try_write_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_write_result) - Try to acquire write lock with a detailed error
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.clone) - Clone the Arc reference

### ArcRwLock

A synchronous read-write lock wrapper supporting multiple concurrent readers.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.new) - Create a new read-write lock
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.read) - Acquire read lock
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.write) - Acquire write lock
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_read) - Try to acquire read lock without blocking
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_write) - Try to acquire write lock without blocking
- [`try_read_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_read_result) - Try to acquire read lock with a detailed error
- [`try_write_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_write_result) - Try to acquire write lock with a detailed error
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.clone) - Clone the Arc reference

### Monitor

A synchronous monitor for condition-based state coordination.

`Monitor` combines a `Mutex` with a `Condvar`. Use it when a thread should
sleep until protected state satisfies a predicate, such as waiting for queued
work, a completion flag, or available permits. Poisoned mutexes are recovered
by taking the inner state, so coordination state remains observable after a
thread panics while holding the lock.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.new) - Create a new monitor
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.read) - Read protected state
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.write) - Mutate protected state
- [`wait_timeout(&self, timeout: Duration) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.wait_timeout) - Wait on the condition variable for at most `timeout` (no predicate)
- [`wait_timeout_until<P, F, R>(&self, timeout: Duration, ready: P, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.wait_timeout_until) - Wait until a predicate is true, then mutate the state, with an overall time limit
- [`wait_until<P, F, R>(&self, ready: P, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.wait_until) - Wait until a predicate is true, then mutate the state
- [`notify_one(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.notify_one) - Wake one waiting thread
- [`notify_all(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.notify_all) - Wake all waiting threads

### ArcMonitor

An Arc-integrated monitor wrapper for sharing condition-based state across
threads without writing `Arc::new(Monitor::new(...))`.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.new) - Create a new shared monitor
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.read) - Read protected state
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.write) - Mutate protected state
- [`wait_timeout(&self, timeout: Duration) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.wait_timeout) - Wait on the condition variable for at most `timeout` (no predicate)
- [`wait_timeout_until<P, F, R>(&self, timeout: Duration, ready: P, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.wait_timeout_until) - Wait until a predicate is true, then mutate the state, with an overall time limit
- [`wait_until<P, F, R>(&self, ready: P, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.wait_until) - Wait until a predicate is true, then mutate the state
- [`notify_one(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.notify_one) - Wake one waiting thread
- [`notify_all(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.notify_all) - Wake all waiting threads
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.clone) - Clone the Arc-backed monitor handle

### ArcAsyncMutex

An asynchronous mutual exclusion lock for Tokio runtime.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.new) - Create a new async mutex
- [`async read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.read) - Asynchronously acquire read lock
- [`async write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.write) - Asynchronously acquire write lock
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.try_read) - Try to acquire read lock (non-blocking)
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.try_write) - Try to acquire write lock (non-blocking)
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.clone) - Clone the Arc reference

### ArcAsyncRwLock

An asynchronous read-write lock for Tokio runtime.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.new) - Create a new async read-write lock
- [`async read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.read) - Asynchronously acquire read lock
- [`async write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.write) - Asynchronously acquire write lock
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.try_read) - Try to acquire read lock (non-blocking)
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.try_write) - Try to acquire write lock (non-blocking)
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.clone) - Clone the Arc reference

### Executor

An execution strategy trait for running fallible one-time tasks.

Executor-related types live under `task::executor`.

**Methods:**
- [`execute<T, E>(&self, task: T)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/executor/trait.Executor.html#method.execute) - Execute a `Runnable<E>`
- [`call<C, R, E>(&self, task: C)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/executor/trait.Executor.html#tymethod.call) - Execute a `Callable<R, E>`

### FutureExecutor

A marker trait for executors whose execution carrier is a future resolving to the task result.

`TokioExecutor` implements this model: `execute` and `call` return futures.

### ExecutorService

A managed task service with submission and lifecycle APIs.

Service-related types live under `task::service`.

`submit` and `submit_callable` return `Ok(handle)` when the service accepts the task. That does not mean the task has started or succeeded. Task success, task failure, panic, or cancellation is observed through the returned handle.

Common synchronous service implementations:

- [`BlockingExecutorService`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/type.BlockingExecutorService.html) / [`ThreadPool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/struct.ThreadPool.html) - Managed service for synchronous tasks that may block OS threads
- [`RayonExecutorService`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/struct.RayonExecutorService.html) - Managed service for CPU-bound synchronous tasks
- [`TokioBlockingExecutorService`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/type.TokioBlockingExecutorService.html) / [`TokioExecutorService`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/struct.TokioExecutorService.html) - Tokio-backed blocking service using `spawn_blocking`

Async Future-based service:

- [`TokioIoExecutorService`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/struct.TokioIoExecutorService.html) - Tokio-backed service for `Future<Output = Result<_, _>>` tasks via `spawn`

Unified facade:

- [`ExecutionServices`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/struct.ExecutionServices.html) - Routes blocking, CPU-bound, Tokio blocking, and async IO tasks through dedicated execution domains

**Methods:**
- [`submit<T, E>(&self, task: T)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#method.submit) - Submit a `Runnable<E>` background task
- [`submit_callable<C, R, E>(&self, task: C)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.submit_callable) - Submit a `Callable<R, E>` task
- [`shutdown(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown) - Initiate graceful shutdown
- [`shutdown_now(&self) -> ShutdownReport`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown_now) - Attempt immediate shutdown and return count-based status
- [`is_shutdown(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_shutdown) - Check if the service is shut down
- [`is_terminated(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_terminated) - Check if all tasks completed
- [`await_termination(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.await_termination) - Wait for service termination

### Runnable, Callable, RunnableWith and CallableWith

Task abstractions provided by `qubit-function`.

**Methods:**
- [`run(&mut self) -> Result<(), E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.Runnable.html#tymethod.run) - Execute a fallible reusable action
- [`call(&mut self) -> Result<R, E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.Callable.html#tymethod.call) - Execute a fallible reusable computation
- [`run_with(&mut self, &mut T) -> Result<(), E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.RunnableWith.html#tymethod.run_with) - Execute a fallible reusable action with mutable input
- [`call_with(&mut self, &mut T) -> Result<R, E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.CallableWith.html#tymethod.call_with) - Execute a fallible reusable computation with mutable input
- [`into_box()`](https://docs.rs/qubit-function/latest/qubit_function/trait.Runnable.html#method.into_box) - Convert a task into `BoxRunnable` or `BoxCallable`

### DoubleCheckedLockExecutor

Reusable executor for the double-checked locking API; see [`DoubleCheckedLockExecutor`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html).

**Typical flow:**
- [`DoubleCheckedLockExecutor::builder`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.builder) — create the reusable executor builder
- (Optional) [`ExecutorBuilder::log_unmet_condition`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorBuilder.html#method.log_unmet_condition) / [`log_prepare_failure`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorBuilder.html#method.log_prepare_failure) / [`log_prepare_commit_failure`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorBuilder.html#method.log_prepare_commit_failure) / [`log_prepare_rollback_failure`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorBuilder.html#method.log_prepare_rollback_failure) — emit `log` lines for unmet conditions and prepare lifecycle failures (the same methods exist on `ExecutorLockBuilder` and `ExecutorReadyBuilder` so you can insert them at different builder stages)
- [`ExecutorBuilder::on`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorBuilder.html#method.on) — attach a [`Lock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Lock.html) handle (for example a cloned [`ArcMutex`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html) or [`ArcRwLock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html))
- [`ExecutorLockBuilder::when`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorLockBuilder.html#method.when) — fast-path predicate (evaluated twice: outside and inside the lock)
- Optional [`prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorReadyBuilder.html#method.prepare) / [`rollback_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorReadyBuilder.html#method.rollback_prepare) / [`commit_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorReadyBuilder.html#method.commit_prepare) — fallible `Runnable` hooks
- [`build`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutorReadyBuilder.html#method.build) — create the reusable executor
- [`call`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.call) / [`execute`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.execute) — run zero-argument `Callable` or `Runnable` tasks under the configured double-check flow
- [`call_with`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.call_with) / [`execute_with`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.execute_with) — run tasks that receive mutable access to the protected value
- [`get_result`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionContext.html#method.get_result) — get the [`ExecutionResult`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/enum.ExecutionResult.html)

## Design Patterns

### Closure-Based Lock Access

All locks use closure-based access patterns for several benefits:

1. **Automatic Release**: Lock is automatically released when closure completes
2. **Exception Safety**: Lock is released even if closure panics
3. **Reduced Boilerplate**: No need to manually manage lock guards
4. **Clear Scope**: Lock scope is explicitly defined by closure boundaries

### Arc Integration

**Important**: All `ArcMutex`, `ArcRwLock`, `ArcAsyncMutex`, `ArcAsyncRwLock`, and `ArcMonitor` types already have `Arc` integrated internally. You don't need to wrap them with `Arc` again.

```rust
// ✅ Correct - use directly
let lock = ArcMutex::new(0);
let lock_clone = lock.clone();  // Clones the internal Arc

// ❌ Wrong - unnecessary double wrapping
let lock = Arc::new(ArcMutex::new(0));  // Don't do this!
```

This design provides several benefits:

1. **Easy Cloning**: Share locks across threads/tasks with simple `.clone()`
2. **No Extra Wrapping**: Use directly without additional `Arc` allocation
3. **Reference Counting**: Automatic cleanup when last reference is dropped
4. **Type Safety**: Compiler ensures proper usage patterns

### Monitor Coordination

Use `Monitor` when a thread should wait for a state transition instead of
polling. Update the protected state with `write`, then call `notify_one` or
`notify_all`. Waiting code should use `wait_until` so spurious notifications
cannot let it proceed before the predicate is actually true.

## Use Cases

### 1. Shared Counter

Perfect for maintaining shared state across multiple threads:

```rust
let counter = ArcMutex::new(0);
// Share counter across threads
let counter_clone = counter.clone();
thread::spawn(move || {
    counter_clone.write(|c| *c += 1);
});
```

### 2. Configuration Cache

Read-write locks are ideal for configuration that's read frequently but written rarely:

```rust
let config = ArcRwLock::new(Config::default());

// Many readers
config.read(|cfg| println!("Port: {}", cfg.port));

// Occasional writer
config.write(|cfg| cfg.port = 8080);
```

### 3. Async Task Coordination

Coordinate state between async tasks without blocking threads:

```rust
let state = ArcAsyncMutex::new(TaskState::Idle);
let state_clone = state.clone();

tokio::spawn(async move {
    state_clone.write(|s| *s = TaskState::Running).await;
    // ... do work ...
    state_clone.write(|s| *s = TaskState::Complete).await;
});
```

## Dependencies

- **tokio**: Async runtime and synchronization primitives (features: `sync`)
- **std**: Standard library synchronization primitives (`Mutex`, `RwLock`, `Condvar`, `Arc`)

## Testing & Code Coverage

This project maintains comprehensive test coverage with detailed validation of all functionality.

### Coverage Metrics

Current test coverage statistics:

| Module | Region Coverage | Line Coverage | Function Coverage |
|--------|----------------|---------------|-------------------|
| lock.rs | 100.00% | 100.00% | 100.00% |
| **Total** | **100.00%** | **100.00%** | **100.00%** |

### Test Scenarios

The test suite covers:

- ✅ **Basic lock operations** - Creating and using locks
- ✅ **Clone semantics** - Sharing locks across threads/tasks
- ✅ **Concurrent access patterns** - Multiple threads/tasks accessing shared data
- ✅ **Lock contention scenarios** - Testing under high contention
- ✅ **Try lock operations** - Non-blocking lock attempts
- ✅ **Poison handling** - Synchronous lock poisoning scenarios
- ✅ **Monitor coordination** - Predicate waiting, notifications, and poison recovery

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Generate detailed HTML report
./coverage.sh html
```

### Coverage Tool Information

The coverage statistics are generated using `cargo-llvm-cov`. For more details on how to run coverage tests and interpret results, see:

- [COVERAGE.md](COVERAGE.md) - English coverage documentation
- [COVERAGE.zh_CN.md](COVERAGE.zh_CN.md) - Chinese coverage documentation
- Project coverage reports in `target/llvm-cov/html/`

## Performance Considerations

### Synchronous vs Asynchronous

- **Synchronous locks** (`ArcMutex`, `ArcRwLock`): Use for CPU-bound operations or when already in a thread-based context
- **Asynchronous locks** (`ArcAsyncMutex`, `ArcAsyncRwLock`): Use within async contexts to avoid blocking the executor

### Read-Write Locks

Read-write locks (`ArcRwLock`, `ArcAsyncRwLock`) are beneficial when:
- Read operations significantly outnumber writes
- Read operations are relatively expensive
- Multiple readers can genuinely execute in parallel

For simple, fast operations or equal read/write patterns, regular mutexes may be simpler and faster.

## License

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

When contributing tests, ensure:
- All lock types are tested (synchronous and asynchronous)
- Concurrent scenarios are validated
- Edge cases are covered (try_lock failures, poisoning, etc.)

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

---

For more information about Qubit Rust libraries, visit our [GitHub organization](https://github.com/qubit-ltd).

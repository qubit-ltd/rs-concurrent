# Qubit Concurrent

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-concurrent.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-concurrent)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-concurrent/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-concurrent?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-concurrent.svg?color=blue)](https://crates.io/crates/qubit-concurrent)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A comprehensive Rust concurrent utilities library providing thread-safe lock wrappers and synchronization primitives for the Qubit Rust libraries.

## Overview

Qubit Concurrent provides easy-to-use wrappers around both synchronous and asynchronous locks, offering a unified interface for concurrent programming in Rust. All lock types have `Arc` built-in internally, so you can clone and share them across threads or tasks directly without additional wrapping. The library provides convenient helper methods for common locking patterns with a closure-based API that ensures proper lock management.

## Features

### 🔒 **Synchronous Locks**
- **ArcMutex**: Thread-safe mutual exclusion lock wrapper with `Arc` integration
- **ArcRwLock**: Thread-safe read-write lock wrapper supporting multiple concurrent readers
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
- **FutureExecutor**: Executor specialization whose execution carrier is a future
- **Runnable / Callable**: Fallible one-time task abstractions re-exported from `qubit-function`
- **Clear acceptance semantics**: `ExecutorService` acceptance is separate from task success

### 🔁 **Double-checked locking**
- **DoubleCheckedLock**: Fluent builder for test-outside-lock / retest-inside-lock flows, optional prepare / rollback / commit hooks, and `call` / `call_mut` tasks
- **ExecutionResult**: Structured outcomes (success, condition unmet, task error, prepare failures)

### 🎯 **Key Benefits**
- **Clone Support**: All lock wrappers implement `Clone` for easy sharing across threads
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Ergonomic API**: Closure-based lock access eliminates common pitfalls
- **Production Ready**: Battle-tested locking patterns with comprehensive test coverage

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-concurrent = "0.4.0"
```

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

### Double-checked locking

Skip lock acquisition and expensive reads when a cheap flag already rules them out (for example a **frozen** account). The same predicate is evaluated again under the lock so a race where the account freezes between checks does not run the heavy `read_balance` work.

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use qubit_concurrent::{DoubleCheckedLock, ArcMutex, lock::Lock};

fn read_balance(latest: &i32) -> Result<i32, std::io::Error> {
    // Expensive: ledger reconciliation, remote validation, etc.
    Ok(*latest)
}

fn main() {
    let balance = ArcMutex::new(1_000);
    let frozen = Arc::new(AtomicBool::new(false));

    let result = DoubleCheckedLock::on(&balance)
        .when({
            let frozen = frozen.clone();
            move || !frozen.load(Ordering::Acquire)
        })
        .call(|cached: &i32| read_balance(cached))
        .get_result();

    assert!(result.is_success());
    assert_eq!(result.unwrap(), 1_000);
}
```

### Task Executor

```rust
use qubit_concurrent::task::{
    executor::{DirectExecutor, Executor, TokioExecutor},
    service::{ExecutorService, ThreadPerTaskExecutorService},
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

**Methods:**
- [`submit<T, E>(&self, task: T)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#method.submit) - Submit a `Runnable<E>` background task
- [`submit_callable<C, R, E>(&self, task: C)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.submit_callable) - Submit a `Callable<R, E>` task
- [`shutdown(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown) - Initiate graceful shutdown
- [`shutdown_now(&self) -> ShutdownReport`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown_now) - Attempt immediate shutdown and return count-based status
- [`is_shutdown(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_shutdown) - Check if the service is shut down
- [`is_terminated(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_terminated) - Check if all tasks completed
- [`await_termination(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.await_termination) - Wait for service termination

### Runnable and Callable

Task abstractions re-exported from `qubit-function`.

**Methods:**
- [`run(self) -> Result<(), E>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Runnable.html#tymethod.run) - Execute a fallible one-time action
- [`call(self) -> Result<R, E>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Callable.html#tymethod.call) - Execute a fallible one-time computation
- [`into_box()`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Runnable.html#method.into_box) - Convert a task into `BoxRunnable` or `BoxCallable`

### DoubleCheckedLock

Entry point for the double-checked locking fluent API; see [`DoubleCheckedLock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLock.html) and [`ExecutionBuilder`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html).

**Typical flow:**
- [`DoubleCheckedLock::on`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLock.html#method.on) — attach to a [`Lock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Lock.html) (for example [`ArcMutex`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html) or [`ArcRwLock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html))
- [`ExecutionBuilder::when`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.when-1) — fast-path predicate (evaluated twice: outside and inside the lock)
- Optional [`prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.prepare-1) / [`rollback_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.rollback_prepare-1) / [`commit_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.commit_prepare-1) — fallible `Runnable` hooks
- [`call`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.call-1) or [`call_mut`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.call_mut-1) — task under the lock
- [`get_result`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionBuilder.html#method.get_result-1) — [`ExecutionResult`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/enum.ExecutionResult.html)

## Design Patterns

### Closure-Based Lock Access

All locks use closure-based access patterns for several benefits:

1. **Automatic Release**: Lock is automatically released when closure completes
2. **Exception Safety**: Lock is released even if closure panics
3. **Reduced Boilerplate**: No need to manually manage lock guards
4. **Clear Scope**: Lock scope is explicitly defined by closure boundaries

### Arc Integration

**Important**: All `ArcMutex`, `ArcRwLock`, `ArcAsyncMutex`, and `ArcAsyncRwLock` types already have `Arc` integrated internally. You don't need to wrap them with `Arc` again.

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
- **std**: Standard library synchronization primitives (`Mutex`, `RwLock`, `Arc`)

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

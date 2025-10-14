# Prism3 Concurrent

[![CircleCI](https://circleci.com/gh/3-prism/prism3-rust-concurrent.svg?style=shield)](https://circleci.com/gh/3-prism/prism3-rust-concurrent)
[![Coverage Status](https://coveralls.io/repos/github/3-prism/prism3-rust-concurrent/badge.svg?branch=main)](https://coveralls.io/github/3-prism/prism3-rust-concurrent?branch=main)
[![Crates.io](https://img.shields.io/crates/v/prism3-concurrent.svg?color=blue)](https://crates.io/crates/prism3-concurrent)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

A comprehensive Rust concurrent utilities library providing thread-safe lock wrappers and synchronization primitives for the Prism3 ecosystem.

## Overview

Prism3 Concurrent provides easy-to-use wrappers around both synchronous and asynchronous locks, offering a unified interface for concurrent programming in Rust. All lock types have `Arc` built-in internally, so you can clone and share them across threads or tasks directly without additional wrapping. The library provides convenient helper methods for common locking patterns with a closure-based API that ensures proper lock management.

## Features

### 🔒 **Synchronous Locks**
- **ArcMutex**: Thread-safe mutual exclusion lock wrapper with `Arc` integration
- **ArcRwLock**: Thread-safe read-write lock wrapper supporting multiple concurrent readers
- **Convenient API**: `with_lock` and `try_with_lock` methods for cleaner lock handling
- **Automatic RAII**: Ensures proper lock release through scope-based management

### 🚀 **Asynchronous Locks**
- **ArcAsyncMutex**: Async-aware mutual exclusion lock for use with Tokio runtime
- **ArcAsyncRwLock**: Async-aware read-write lock supporting concurrent async reads
- **Non-blocking**: Designed for async contexts without blocking threads
- **Tokio Integration**: Built on top of Tokio's synchronization primitives

### 🎯 **Key Benefits**
- **Clone Support**: All lock wrappers implement `Clone` for easy sharing across threads
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Ergonomic API**: Closure-based lock access eliminates common pitfalls
- **Production Ready**: Battle-tested locking patterns with comprehensive test coverage

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
prism3-concurrent = "0.1.0"
```

## Quick Start

### Synchronous Mutex

```rust
use prism3_concurrent::ArcMutex;
use std::thread;

fn main() {
    let counter = ArcMutex::new(0);
    let mut handles = vec![];

    // Spawn multiple threads that increment the counter
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            counter.with_lock(|value| {
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
    let result = counter.with_lock(|value| *value);
    println!("Final counter: {}", result); // Prints: Final counter: 10
}
```

### Synchronous Read-Write Lock

```rust
use prism3_concurrent::ArcRwLock;

fn main() {
    let data = ArcRwLock::new(vec![1, 2, 3]);

    // Multiple concurrent reads
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = std::thread::spawn(move || {
        let len = data1.with_read_lock(|v| v.len());
        println!("Length from thread 1: {}", len);
    });

    let handle2 = std::thread::spawn(move || {
        let len = data2.with_read_lock(|v| v.len());
        println!("Length from thread 2: {}", len);
    });

    // Exclusive write access
    data.with_write_lock(|v| {
        v.push(4);
        println!("Added element, new length: {}", v.len());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
```

### Asynchronous Mutex

```rust
use prism3_concurrent::ArcAsyncMutex;

#[tokio::main]
async fn main() {
    let counter = ArcAsyncMutex::new(0);
    let mut handles = vec![];

    // Spawn multiple async tasks
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = tokio::spawn(async move {
            counter.with_lock(|value| {
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
    let result = counter.with_lock(|value| *value).await;
    println!("Final counter: {}", result); // Prints: Final counter: 10
}
```

### Asynchronous Read-Write Lock

```rust
use prism3_concurrent::ArcAsyncRwLock;

#[tokio::main]
async fn main() {
    let data = ArcAsyncRwLock::new(String::from("Hello"));

    // Concurrent async reads
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = tokio::spawn(async move {
        let content = data1.with_read_lock(|s| s.clone()).await;
        println!("Read from task 1: {}", content);
    });

    let handle2 = tokio::spawn(async move {
        let content = data2.with_read_lock(|s| s.clone()).await;
        println!("Read from task 2: {}", content);
    });

    // Exclusive async write
    data.with_write_lock(|s| {
        s.push_str(" World!");
        println!("Updated string: {}", s);
    }).await;

    handle1.await.unwrap();
    handle2.await.unwrap();
}
```

### Try Lock (Non-blocking)

```rust
use prism3_concurrent::ArcMutex;

fn main() {
    let mutex = ArcMutex::new(42);

    // Try to acquire lock without blocking
    match mutex.try_with_lock(|value| *value) {
        Some(v) => println!("Got value: {}", v),
        None => println!("Lock is busy"),
    }
}
```

## API Reference

### ArcMutex

A synchronous mutual exclusion lock wrapper with `Arc` integration.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcMutex.html#method.new) - Create a new mutex
- [`with_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcMutex.html#method.with_lock) - Acquire lock and execute closure
- [`try_with_lock<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcMutex.html#method.try_with_lock) - Try to acquire lock without blocking
- [`clone(&self) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcMutex.html#method.clone) - Clone the Arc reference

### ArcRwLock

A synchronous read-write lock wrapper supporting multiple concurrent readers.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcRwLock.html#method.new) - Create a new read-write lock
- [`with_read_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcRwLock.html#method.with_read_lock) - Acquire read lock
- [`with_write_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcRwLock.html#method.with_write_lock) - Acquire write lock
- [`clone(&self) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcRwLock.html#method.clone) - Clone the Arc reference

### ArcAsyncMutex

An asynchronous mutual exclusion lock for Tokio runtime.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncMutex.html#method.new) - Create a new async mutex
- [`async with_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncMutex.html#method.with_lock) - Asynchronously acquire lock
- [`try_with_lock<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncMutex.html#method.try_with_lock) - Try to acquire lock (non-blocking)
- [`clone(&self) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncMutex.html#method.clone) - Clone the Arc reference

### ArcAsyncRwLock

An asynchronous read-write lock for Tokio runtime.

**Methods:**
- [`new(data: T) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncRwLock.html#method.new) - Create a new async read-write lock
- [`async with_read_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncRwLock.html#method.with_read_lock) - Asynchronously acquire read lock
- [`async with_write_lock<F, R>(&self, f: F) -> R`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncRwLock.html#method.with_write_lock) - Asynchronously acquire write lock
- [`clone(&self) -> Self`](https://docs.rs/prism3-concurrent/latest/prism3_concurrent/struct.ArcAsyncRwLock.html#method.clone) - Clone the Arc reference

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
    counter_clone.with_lock(|c| *c += 1);
});
```

### 2. Configuration Cache

Read-write locks are ideal for configuration that's read frequently but written rarely:

```rust
let config = ArcRwLock::new(Config::default());

// Many readers
config.with_read_lock(|cfg| println!("Port: {}", cfg.port));

// Occasional writer
config.with_write_lock(|cfg| cfg.port = 8080);
```

### 3. Async Task Coordination

Coordinate state between async tasks without blocking threads:

```rust
let state = ArcAsyncMutex::new(TaskState::Idle);
let state_clone = state.clone();

tokio::spawn(async move {
    state_clone.with_lock(|s| *s = TaskState::Running).await;
    // ... do work ...
    state_clone.with_lock(|s| *s = TaskState::Complete).await;
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

Copyright (c) 2025 3-Prism Co. Ltd. All rights reserved.

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

**Hu Haixing** - *3-Prism Co. Ltd.*

---

For more information about the Prism3 ecosystem, visit our [GitHub homepage](https://github.com/3-prism).

# Qubit Concurrent

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-concurrent.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-concurrent)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-concurrent/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-concurrent?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-concurrent.svg?color=blue)](https://crates.io/crates/qubit-concurrent)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Documentation](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

为 Qubit Rust 组件库提供线程安全锁包装器和同步原语的综合性 Rust 并发工具库。

## 概述

Qubit Concurrent 为同步和异步锁提供易于使用的包装器，为 Rust 并发编程提供统一的接口。所有锁类型内部都已集成 `Arc`，因此你可以直接克隆并在线程或任务间共享它们，无需额外包装。该库通过基于闭包的 API 为常见锁模式提供便捷的辅助方法，确保正确的锁管理。

## 特性

### 🔒 **同步锁**
- **ArcMutex**：集成 `Arc` 的线程安全互斥锁包装器
- **ArcRwLock**：支持多个并发读者的线程安全读写锁包装器
- **Monitor / ArcMonitor**：基于 `Mutex` 和 `Condvar` 的条件状态协调原语，并提供集成 `Arc` 的共享包装器
- **便捷 API**：提供 `read`/`write` 与 `try_read`/`try_write` 方法，实现更清晰的锁处理
- **自动 RAII**：通过基于作用域的管理确保正确释放锁

### 🚀 **异步锁**
- **ArcAsyncMutex**：用于 Tokio 运行时的异步感知互斥锁
- **ArcAsyncRwLock**：支持并发异步读取的异步感知读写锁
- **非阻塞**：专为异步上下文设计，不会阻塞线程
- **Tokio 集成**：构建于 Tokio 的同步原语之上

### ⚙️ **任务执行**
- **Executor**：位于 `task::executor` 的执行策略 trait，`execute` 执行 `Runnable`，`call` 执行 `Callable`
- **ExecutorService**：位于 `task::service` 的托管任务服务，提供 `submit`、`submit_callable` 和优雅关闭
- **FutureExecutor**：执行结果载体为 Future 的特殊 Executor
- **Runnable / Callable**：由 `qubit-function` 提供的可复用可失败任务抽象
- **清晰接收语义**：`ExecutorService` 接收任务与任务执行成功是两件事

### 🔁 **双重检查锁**
- **DoubleCheckedLockExecutor**：可复用的双重检查执行器（锁外/锁内两次条件判断、可选 prepare / 回滚 / 提交、`call` / `execute` / `call_with` / `execute_with` 任务）
- **ExecutionResult**：结构化结果（成功、条件未满足、任务或 prepare 错误等）

### 🎯 **主要优势**
- **克隆支持**：所有锁包装器都实现了 `Clone`，便于跨线程共享
- **类型安全**：利用 Rust 的类型系统提供编译时保证
- **人性化 API**：基于闭包的锁访问消除了常见陷阱
- **生产就绪**：经过实战检验的锁模式，具有全面的测试覆盖

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-concurrent = "0.5.0"
```

## 快速开始

### 同步互斥锁

```rust
use qubit_concurrent::ArcMutex;
use std::thread;

fn main() {
    let counter = ArcMutex::new(0);
    let mut handles = vec![];

    // 生成多个线程来增加计数器
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            counter.write(|value| {
                *value += 1;
            });
        });
        handles.push(handle);
    }

    // 等待所有线程
    for handle in handles {
        handle.join().unwrap();
    }

    // 读取最终值
    let result = counter.read(|value| *value);
    println!("最终计数: {}", result); // 输出: 最终计数: 10
}
```

### 同步读写锁

```rust
use qubit_concurrent::ArcRwLock;

fn main() {
    let data = ArcRwLock::new(vec![1, 2, 3]);

    // 多个并发读取
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = std::thread::spawn(move || {
        let len = data1.read(|v| v.len());
        println!("线程 1 读取的长度: {}", len);
    });

    let handle2 = std::thread::spawn(move || {
        let len = data2.read(|v| v.len());
        println!("线程 2 读取的长度: {}", len);
    });

    // 独占写访问
    data.write(|v| {
        v.push(4);
        println!("添加元素后，新长度: {}", v.len());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
```

### 异步互斥锁

```rust
use qubit_concurrent::ArcAsyncMutex;

#[tokio::main]
async fn main() {
    let counter = ArcAsyncMutex::new(0);
    let mut handles = vec![];

    // 生成多个异步任务
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = tokio::spawn(async move {
            counter.write(|value| {
                *value += 1;
            }).await;
        });
        handles.push(handle);
    }

    // 等待所有任务
    for handle in handles {
        handle.await.unwrap();
    }

    // 读取最终值
    let result = counter.read(|value| *value).await;
    println!("最终计数: {}", result); // 输出: 最终计数: 10
}
```

### 异步读写锁

```rust
use qubit_concurrent::ArcAsyncRwLock;

#[tokio::main]
async fn main() {
    let data = ArcAsyncRwLock::new(String::from("你好"));

    // 并发异步读取
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = tokio::spawn(async move {
        let content = data1.read(|s| s.clone()).await;
        println!("任务 1 读取: {}", content);
    });

    let handle2 = tokio::spawn(async move {
        let content = data2.read(|s| s.clone()).await;
        println!("任务 2 读取: {}", content);
    });

    // 独占异步写入
    data.write(|s| {
        s.push_str("，世界！");
        println!("更新后的字符串: {}", s);
    }).await;

    handle1.await.unwrap();
    handle2.await.unwrap();
}
```

### 尝试加锁（非阻塞）

```rust
use qubit_concurrent::ArcMutex;

fn main() {
    let mutex = ArcMutex::new(42);

    // 尝试获取锁而不阻塞
    match mutex.try_read(|value| *value) {
        Some(v) => println!("获取到值: {}", v),
        None => println!("锁正忙"),
    }
}
```

### 基于条件的 Monitor

```rust
use std::thread;

use qubit_concurrent::ArcMonitor;

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

### 双重检查锁

当廉价标志已能排除读路径时（例如账户已**冻结**），可跳过加锁与昂贵的余额读取。同一条件会在加锁后再判断一次，避免在快路径通过之后、持锁前被冻结时仍执行高成本的 `read_balance`。

```rust
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use qubit_concurrent::{DoubleCheckedLockExecutor, ArcMutex, lock::Lock};

fn read_balance(latest: &i32) -> Result<i32, std::io::Error> {
    // 高成本：对账、远程校验等
    Ok(*latest)
}

fn main() {
    let balance = ArcMutex::new(1_000);
    let frozen = Arc::new(AtomicBool::new(false));

    let executor = DoubleCheckedLockExecutor::builder()
        .on(balance.clone())
        .when({
            let frozen = frozen.clone();
            move || !frozen.load(Ordering::Acquire)
        })
        .build();

    let result = executor
        .call_with(|latest: &mut i32| read_balance(latest))
        .get_result();

    assert!(result.is_success());
    assert_eq!(result.unwrap(), 1_000);
}
```

## API 参考

### ArcMutex

集成 `Arc` 的同步互斥锁包装器。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.new) - 创建新的互斥锁
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.read) - 获取读锁并执行闭包
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.write) - 获取写锁并执行闭包
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_read) - 尝试获取读锁而不阻塞
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_write) - 尝试获取写锁而不阻塞
- [`try_read_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_read_result) - 尝试获取读锁并返回详细错误
- [`try_write_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.try_write_result) - 尝试获取写锁并返回详细错误
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html#method.clone) - 克隆 Arc 引用

### ArcRwLock

支持多个并发读者的同步读写锁包装器。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.new) - 创建新的读写锁
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.read) - 获取读锁
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.write) - 获取写锁
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_read) - 尝试获取读锁而不阻塞
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_write) - 尝试获取写锁而不阻塞
- [`try_read_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_read_result) - 尝试获取读锁并返回详细错误
- [`try_write_result<F, R>(&self, f: F) -> Result<R, TryLockError>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.try_write_result) - 尝试获取写锁并返回详细错误
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html#method.clone) - 克隆 Arc 引用

### Monitor

用于基于条件进行状态协调的同步 monitor。

`Monitor` 组合了一个 `Mutex` 和一个 `Condvar`。当线程需要等待受保护
状态满足某个条件时，可以使用它，例如等待队列中出现任务、完成标志变为
真，或许可数量可用。毒化的 mutex 会通过取出内部状态的方式恢复，因此
即使某个线程持锁时 panic，协调状态仍然可以被观察和继续使用。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.new) - 创建新的 monitor
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.read) - 读取受保护状态
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.write) - 修改受保护状态
- [`wait_until<P, F, R>(&self, ready: P, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.wait_until) - 等待条件为真，然后修改状态
- [`notify_one(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.notify_one) - 唤醒一个等待线程
- [`notify_all(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.Monitor.html#method.notify_all) - 唤醒所有等待线程

### ArcMonitor

集成 `Arc` 的 monitor 包装器，用于在线程间共享条件状态，无需手写
`Arc::new(Monitor::new(...))`。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.new) - 创建新的共享 monitor
- [`read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.read) - 读取受保护状态
- [`write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.write) - 修改受保护状态
- [`wait_until<P, F, R>(&self, ready: P, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.wait_until) - 等待条件为真，然后修改状态
- [`notify_one(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.notify_one) - 唤醒一个等待线程
- [`notify_all(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.notify_all) - 唤醒所有等待线程
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/lock/struct.ArcMonitor.html#method.clone) - 克隆 Arc 支持的 monitor 句柄

### ArcAsyncMutex

用于 Tokio 运行时的异步互斥锁。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.new) - 创建新的异步互斥锁
- [`async read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.read) - 异步获取读锁
- [`async write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.write) - 异步获取写锁
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.try_read) - 尝试获取读锁（非阻塞）
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.try_write) - 尝试获取写锁（非阻塞）
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncMutex.html#method.clone) - 克隆 Arc 引用

### ArcAsyncRwLock

用于 Tokio 运行时的异步读写锁。

**方法：**
- [`new(data: T) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.new) - 创建新的异步读写锁
- [`async read<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.read) - 异步获取读锁
- [`async write<F, R>(&self, f: F) -> R`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.write) - 异步获取写锁
- [`try_read<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.try_read) - 尝试获取读锁（非阻塞）
- [`try_write<F, R>(&self, f: F) -> Option<R>`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.try_write) - 尝试获取写锁（非阻塞）
- [`clone(&self) -> Self`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcAsyncRwLock.html#method.clone) - 克隆 Arc 引用

### Executor

用于按执行策略运行一次性可失败任务的 trait。

Executor 相关类型位于 `task::executor` 模块。

**方法：**
- [`execute<T, E>(&self, task: T)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/executor/trait.Executor.html#method.execute) - 执行 `Runnable<E>`
- [`call<C, R, E>(&self, task: C)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/executor/trait.Executor.html#tymethod.call) - 执行 `Callable<R, E>`

### FutureExecutor

执行结果载体为 Future 的特殊 `Executor`。

`TokioExecutor` 采用该模型：`execute` 和 `call` 返回可等待的 Future。

### ExecutorService

带生命周期管理的任务服务。

Service 相关类型位于 `task::service` 模块。

`submit` 和 `submit_callable` 返回 `Ok(handle)` 只表示服务已接收任务，不表示任务已经开始，也不表示任务执行成功。任务成功、任务返回 `Err(E)`、panic 或取消，都必须通过返回的 handle 观察。

**方法：**
- [`submit<T, E>(&self, task: T)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#method.submit) - 提交 `Runnable<E>` 后台任务
- [`submit_callable<C, R, E>(&self, task: C)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.submit_callable) - 提交 `Callable<R, E>` 任务
- [`shutdown(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown) - 启动优雅关闭
- [`shutdown_now(&self) -> ShutdownReport`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.shutdown_now) - 尝试立即关闭并返回计数报告
- [`is_shutdown(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_shutdown) - 检查服务是否已关闭
- [`is_terminated(&self) -> bool`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.is_terminated) - 检查所有任务是否已完成
- [`await_termination(&self)`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/task/service/trait.ExecutorService.html#tymethod.await_termination) - 等待服务终止

### Runnable、Callable、RunnableWith 与 CallableWith

由 `qubit-function` 提供的任务抽象。

**方法：**
- [`run(&mut self) -> Result<(), E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.Runnable.html#tymethod.run) - 执行可复用可失败动作
- [`call(&mut self) -> Result<R, E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.Callable.html#tymethod.call) - 执行可复用可失败计算
- [`run_with(&mut self, &mut T) -> Result<(), E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.RunnableWith.html#tymethod.run_with) - 用可变输入执行可复用可失败动作
- [`call_with(&mut self, &mut T) -> Result<R, E>`](https://docs.rs/qubit-function/latest/qubit_function/trait.CallableWith.html#tymethod.call_with) - 用可变输入执行可复用可失败计算
- [`into_box()`](https://docs.rs/qubit-function/latest/qubit_function/trait.Runnable.html#method.into_box) - 转换为 `BoxRunnable` 或 `BoxCallable`

### DoubleCheckedLockExecutor

可复用的双重检查锁执行器；详见 [`DoubleCheckedLockExecutor`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html)。

**典型步骤：**
- [`DoubleCheckedLockExecutor::builder`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.builder) — 创建可复用执行器的构建器
- [`DoubleCheckedLockExecutorBuilder::on`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorBuilder.html#method.on) — 绑定实现 [`Lock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/trait.Lock.html) 的句柄（例如克隆后的 [`ArcMutex`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcMutex.html)、[`ArcRwLock`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ArcRwLock.html)）
- [`DoubleCheckedLockExecutorLockBuilder::when`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorLockBuilder.html#method.when) — 快路径条件（在锁外与锁内各执行一次）
- 可选 [`prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorReadyBuilder.html#method.prepare) / [`rollback_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorReadyBuilder.html#method.rollback_prepare) / [`commit_prepare`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorReadyBuilder.html#method.commit_prepare) — 可失败 `Runnable` 钩子
- [`build`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutorReadyBuilder.html#method.build) — 创建可复用执行器
- [`call`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.call) / [`execute`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.execute) — 在配置好的双重检查流程下执行零参数 `Callable` 或 `Runnable`
- [`call_with`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.call_with) / [`execute_with`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.DoubleCheckedLockExecutor.html#method.execute_with) — 执行可获得受保护值可变访问权的任务
- [`get_result`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/struct.ExecutionContext.html#method.get_result) — 得到 [`ExecutionResult`](https://docs.rs/qubit-concurrent/latest/qubit_concurrent/enum.ExecutionResult.html)

## 设计模式

### 基于闭包的锁访问

所有锁都使用基于闭包的访问模式，具有以下优势：

1. **自动释放**：闭包完成时自动释放锁
2. **异常安全**：即使闭包发生 panic，锁也会被释放
3. **减少样板代码**：无需手动管理锁守卫
4. **清晰的作用域**：锁的作用域由闭包边界明确定义

### Arc 集成

**重要提示**：所有的 `ArcMutex`、`ArcRwLock`、`ArcAsyncMutex`、`ArcAsyncRwLock` 和 `ArcMonitor` 类型内部已经集成了 `Arc`。你不需要再用 `Arc` 包装它们。

```rust
// ✅ 正确 - 直接使用
let lock = ArcMutex::new(0);
let lock_clone = lock.clone();  // 克隆内部的 Arc

// ❌ 错误 - 不必要的双重包装
let lock = Arc::new(ArcMutex::new(0));  // 不要这样做！
```

这种设计提供了以下优势：

1. **轻松克隆**：通过简单的 `.clone()` 在线程/任务间共享锁
2. **无需额外包装**：直接使用，无需额外的 `Arc` 分配
3. **引用计数**：当最后一个引用被丢弃时自动清理
4. **类型安全**：编译器确保正确的使用模式

### Monitor 协调

当线程应该等待某个状态变化，而不是轮询时，使用 `Monitor`。通过 `write`
更新受保护状态，然后调用 `notify_one` 或 `notify_all`。等待方应使用
`wait_until`，这样即使出现虚假通知，也不会在条件真正满足前继续执行。

## 使用场景

### 1. 共享计数器

非常适合在多个线程间维护共享状态：

```rust
let counter = ArcMutex::new(0);
// 跨线程共享计数器
let counter_clone = counter.clone();
thread::spawn(move || {
    counter_clone.write(|c| *c += 1);
});
```

### 2. 配置缓存

读写锁非常适合频繁读取但很少写入的配置：

```rust
let config = ArcRwLock::new(Config::default());

// 多个读者
config.read(|cfg| println!("端口: {}", cfg.port));

// 偶尔的写入者
config.write(|cfg| cfg.port = 8080);
```

### 3. 异步任务协调

在异步任务之间协调状态而不阻塞线程：

```rust
let state = ArcAsyncMutex::new(TaskState::Idle);
let state_clone = state.clone();

tokio::spawn(async move {
    state_clone.write(|s| *s = TaskState::Running).await;
    // ... 执行工作 ...
    state_clone.write(|s| *s = TaskState::Complete).await;
});
```

## 依赖项

- **tokio**：异步运行时和同步原语（features: `sync`）
- **std**：标准库同步原语（`Mutex`、`RwLock`、`Condvar`、`Arc`）

## 测试与代码覆盖率

本项目保持全面的测试覆盖，详细验证所有功能。

### 覆盖率指标

当前测试覆盖率统计：

| 模块 | 区域覆盖率 | 行覆盖率 | 函数覆盖率 |
|--------|----------------|---------------|-------------------|
| lock.rs | 100.00% | 100.00% | 100.00% |
| **总计** | **100.00%** | **100.00%** | **100.00%** |

### 测试场景

测试套件覆盖：

- ✅ **基本锁操作** - 创建和使用锁
- ✅ **克隆语义** - 跨线程/任务共享锁
- ✅ **并发访问模式** - 多个线程/任务访问共享数据
- ✅ **锁竞争场景** - 高竞争环境下的测试
- ✅ **尝试加锁操作** - 非阻塞锁尝试
- ✅ **毒化处理** - 同步锁毒化场景
- ✅ **Monitor 协调** - 条件等待、通知和毒化恢复

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 生成详细的 HTML 报告
./coverage.sh html
```

### 覆盖率工具信息

覆盖率统计使用 `cargo-llvm-cov` 生成。关于如何运行覆盖率测试和解释结果的更多详情，请参见：

- [COVERAGE.md](COVERAGE.md) - 英文覆盖率文档
- [COVERAGE.zh_CN.md](COVERAGE.zh_CN.md) - 中文覆盖率文档
- `target/llvm-cov/html/` 中的项目覆盖率报告

## 性能考虑

### 同步 vs 异步

- **同步锁**（`ArcMutex`、`ArcRwLock`）：用于 CPU 密集型操作或已经在基于线程的上下文中时
- **异步锁**（`ArcAsyncMutex`、`ArcAsyncRwLock`）：在异步上下文中使用，以避免阻塞执行器

### 读写锁

在以下情况下，读写锁（`ArcRwLock`、`ArcAsyncRwLock`）是有益的：
- 读操作远多于写操作
- 读操作相对昂贵
- 多个读者可以真正并行执行

对于简单、快速的操作或读写模式相当的情况，常规互斥锁可能更简单、更快。

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献！请随时提交 Pull Request。

在贡献测试时，请确保：
- 测试所有锁类型（同步和异步）
- 验证并发场景
- 覆盖边界情况（try_lock 失败、毒化等）

## 作者

**胡海星** - *Qubit Co. Ltd.*

---

有关 Qubit Rust 组件库的更多信息，请访问我们的 [GitHub 组织](https://github.com/qubit-ltd)。

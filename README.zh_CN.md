# Prism3 Concurrent

[![Crates.io](https://img.shields.io/crates/v/prism3-concurrent.svg?color=blue)](https://crates.io/crates/prism3-concurrent)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Documentation](https://docs.rs/prism3-concurrent/badge.svg)](https://docs.rs/prism3-concurrent)
[![English Documentation](https://img.shields.io/badge/docs-English-blue.svg)](README.md)

为 Prism3 生态系统提供线程安全锁包装器和同步原语的综合性 Rust 并发工具库。

## 概述

Prism3 Concurrent 为同步和异步锁提供易于使用的包装器，为 Rust 并发编程提供统一的接口。所有锁类型内部都已集成 `Arc`，因此你可以直接克隆并在线程或任务间共享它们，无需额外包装。该库通过基于闭包的 API 为常见锁模式提供便捷的辅助方法，确保正确的锁管理。

## 特性

### 🔒 **同步锁**
- **ArcMutex**：集成 `Arc` 的线程安全互斥锁包装器
- **ArcRwLock**：支持多个并发读者的线程安全读写锁包装器
- **便捷 API**：提供 `with_lock` 和 `try_with_lock` 方法，实现更清晰的锁处理
- **自动 RAII**：通过基于作用域的管理确保正确释放锁

### 🚀 **异步锁**
- **ArcAsyncMutex**：用于 Tokio 运行时的异步感知互斥锁
- **ArcAsyncRwLock**：支持并发异步读取的异步感知读写锁
- **非阻塞**：专为异步上下文设计，不会阻塞线程
- **Tokio 集成**：构建于 Tokio 的同步原语之上

### 🎯 **主要优势**
- **克隆支持**：所有锁包装器都实现了 `Clone`，便于跨线程共享
- **类型安全**：利用 Rust 的类型系统提供编译时保证
- **人性化 API**：基于闭包的锁访问消除了常见陷阱
- **生产就绪**：经过实战检验的锁模式，具有全面的测试覆盖

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
prism3-concurrent = "0.1.0"
```

## 快速开始

### 同步互斥锁

```rust
use prism3_concurrent::ArcMutex;
use std::thread;

fn main() {
    let counter = ArcMutex::new(0);
    let mut handles = vec![];

    // 生成多个线程来增加计数器
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = thread::spawn(move || {
            counter.with_lock(|value| {
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
    let result = counter.with_lock(|value| *value);
    println!("最终计数: {}", result); // 输出: 最终计数: 10
}
```

### 同步读写锁

```rust
use prism3_concurrent::ArcRwLock;

fn main() {
    let data = ArcRwLock::new(vec![1, 2, 3]);

    // 多个并发读取
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = std::thread::spawn(move || {
        let len = data1.with_read_lock(|v| v.len());
        println!("线程 1 读取的长度: {}", len);
    });

    let handle2 = std::thread::spawn(move || {
        let len = data2.with_read_lock(|v| v.len());
        println!("线程 2 读取的长度: {}", len);
    });

    // 独占写访问
    data.with_write_lock(|v| {
        v.push(4);
        println!("添加元素后，新长度: {}", v.len());
    });

    handle1.join().unwrap();
    handle2.join().unwrap();
}
```

### 异步互斥锁

```rust
use prism3_concurrent::ArcAsyncMutex;

#[tokio::main]
async fn main() {
    let counter = ArcAsyncMutex::new(0);
    let mut handles = vec![];

    // 生成多个异步任务
    for _ in 0..10 {
        let counter = counter.clone();
        let handle = tokio::spawn(async move {
            counter.with_lock(|value| {
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
    let result = counter.with_lock(|value| *value).await;
    println!("最终计数: {}", result); // 输出: 最终计数: 10
}
```

### 异步读写锁

```rust
use prism3_concurrent::ArcAsyncRwLock;

#[tokio::main]
async fn main() {
    let data = ArcAsyncRwLock::new(String::from("你好"));

    // 并发异步读取
    let data1 = data.clone();
    let data2 = data.clone();

    let handle1 = tokio::spawn(async move {
        let content = data1.with_read_lock(|s| s.clone()).await;
        println!("任务 1 读取: {}", content);
    });

    let handle2 = tokio::spawn(async move {
        let content = data2.with_read_lock(|s| s.clone()).await;
        println!("任务 2 读取: {}", content);
    });

    // 独占异步写入
    data.with_write_lock(|s| {
        s.push_str("，世界！");
        println!("更新后的字符串: {}", s);
    }).await;

    handle1.await.unwrap();
    handle2.await.unwrap();
}
```

### 尝试加锁（非阻塞）

```rust
use prism3_concurrent::ArcMutex;

fn main() {
    let mutex = ArcMutex::new(42);

    // 尝试获取锁而不阻塞
    match mutex.try_with_lock(|value| *value) {
        Some(v) => println!("获取到值: {}", v),
        None => println!("锁正忙"),
    }
}
```

## API 参考

### ArcMutex

集成 `Arc` 的同步互斥锁包装器。

**方法：**
- `new(data: T) -> Self` - 创建新的互斥锁
- `with_lock<F, R>(&self, f: F) -> R` - 获取锁并执行闭包
- `try_with_lock<F, R>(&self, f: F) -> Option<R>` - 尝试获取锁而不阻塞
- `clone(&self) -> Self` - 克隆 Arc 引用

### ArcRwLock

支持多个并发读者的同步读写锁包装器。

**方法：**
- `new(data: T) -> Self` - 创建新的读写锁
- `with_read_lock<F, R>(&self, f: F) -> R` - 获取读锁
- `with_write_lock<F, R>(&self, f: F) -> R` - 获取写锁
- `clone(&self) -> Self` - 克隆 Arc 引用

### ArcAsyncMutex

用于 Tokio 运行时的异步互斥锁。

**方法：**
- `new(data: T) -> Self` - 创建新的异步互斥锁
- `async with_lock<F, R>(&self, f: F) -> R` - 异步获取锁
- `try_with_lock<F, R>(&self, f: F) -> Option<R>` - 尝试获取锁（非阻塞）
- `clone(&self) -> Self` - 克隆 Arc 引用

### ArcAsyncRwLock

用于 Tokio 运行时的异步读写锁。

**方法：**
- `new(data: T) -> Self` - 创建新的异步读写锁
- `async with_read_lock<F, R>(&self, f: F) -> R` - 异步获取读锁
- `async with_write_lock<F, R>(&self, f: F) -> R` - 异步获取写锁
- `clone(&self) -> Self` - 克隆 Arc 引用

## 设计模式

### 基于闭包的锁访问

所有锁都使用基于闭包的访问模式，具有以下优势：

1. **自动释放**：闭包完成时自动释放锁
2. **异常安全**：即使闭包发生 panic，锁也会被释放
3. **减少样板代码**：无需手动管理锁守卫
4. **清晰的作用域**：锁的作用域由闭包边界明确定义

### Arc 集成

**重要提示**：所有的 `ArcMutex`、`ArcRwLock`、`ArcAsyncMutex` 和 `ArcAsyncRwLock` 类型内部已经集成了 `Arc`。你不需要再用 `Arc` 包装它们。

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

## 使用场景

### 1. 共享计数器

非常适合在多个线程间维护共享状态：

```rust
let counter = ArcMutex::new(0);
// 跨线程共享计数器
let counter_clone = counter.clone();
thread::spawn(move || {
    counter_clone.with_lock(|c| *c += 1);
});
```

### 2. 配置缓存

读写锁非常适合频繁读取但很少写入的配置：

```rust
let config = ArcRwLock::new(Config::default());

// 多个读者
config.with_read_lock(|cfg| println!("端口: {}", cfg.port));

// 偶尔的写入者
config.with_write_lock(|cfg| cfg.port = 8080);
```

### 3. 异步任务协调

在异步任务之间协调状态而不阻塞线程：

```rust
let state = ArcAsyncMutex::new(TaskState::Idle);
let state_clone = state.clone();

tokio::spawn(async move {
    state_clone.with_lock(|s| *s = TaskState::Running).await;
    // ... 执行工作 ...
    state_clone.with_lock(|s| *s = TaskState::Complete).await;
});
```

## 依赖项

- **tokio**：异步运行时和同步原语（`AsyncMutex`、`AsyncRwLock`）
- **std**：标准库同步原语（`Mutex`、`RwLock`、`Arc`）

## 测试与代码覆盖率

本项目保持全面的测试覆盖，验证所有锁场景，包括：

- 基本锁操作
- 克隆语义
- 并发访问模式
- 锁竞争场景
- 毒化处理（针对同步锁）

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text
```

详细的覆盖率信息请参见 [COVERAGE.md](COVERAGE.md)。

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

本项目采用 Apache License 2.0 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 作者

**胡海星** - *三棱镜有限公司*

---

有关 Prism3 生态系统的更多信息，请访问我们的 [GitHub 仓库](https://github.com/3-prism/prism3-rust-commons)。


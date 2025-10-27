# ExecutionBuilder 使用 ArcTester 的设计说明

## 概述

`ExecutionBuilder` 使用 `prism3_function::ArcTester` 作为条件测试器的类型。通过 `when` 方法，可以接受任何实现了 `Tester` trait 的类型，并自动转换为 `ArcTester`。

## 主要变更

### 1. 类型参数变更

**之前：**
```rust
pub struct ExecutionBuilder<'a, L, T, Tester>
where
    L: Lock<T>,
    Tester: Fn() -> bool,
{
    // ...
}
```

**之后：**
```rust
pub struct ExecutionBuilder<'a, L, T, Tst>
where
    L: Lock<T>,
    Tst: Tester,
{
    // ...
}
```

### 2. 方法调用变更

**之前：**
```rust
if !(self.tester)() {
    // ...
}
```

**之后：**
```rust
if !self.tester.test() {
    // ...
}
```

### 3. 依赖添加

在 `Cargo.toml` 中已经包含了 `prism3-function` 依赖：
```toml
[dependencies]
prism3-function = "0.4.0"
```

## 使用方式

### 1. 使用闭包（自动实现 Tester trait）

最简单的方式，闭包会自动实现 `Tester` trait：

```rust
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_concurrent::{DoubleCheckedLockExecutor, lock::ArcMutex};

let running = Arc::new(AtomicBool::new(true));
let data = ArcMutex::new(42);

let result = DoubleCheckedLockExecutor::on(&data)
    .when({
        let running = running.clone();
        move || running.load(Ordering::Acquire)
    })
    .call_mut(|value| {
        *value += 1;
        Ok::<_, std::io::Error>(*value)
    });
```

### 2. 使用 BoxTester

适用于单次使用和构建器模式：

```rust
use prism3_function::BoxTester;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_concurrent::{DoubleCheckedLockExecutor, lock::ArcMutex};

let running = Arc::new(AtomicBool::new(true));
let data = ArcMutex::new(42);

let tester = {
    let running = running.clone();
    BoxTester::new(move || running.load(Ordering::Acquire))
};

let result = DoubleCheckedLockExecutor::on(&data)
    .when(tester)
    .call_mut(|value| {
        *value += 1;
        Ok::<_, std::io::Error>(*value)
    });
```

### 3. 使用 ArcTester

适用于需要跨线程共享的场景：

```rust
use prism3_function::ArcTester;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_concurrent::{DoubleCheckedLockExecutor, lock::ArcMutex};

let running = Arc::new(AtomicBool::new(true));
let data = ArcMutex::new(42);

let tester = {
    let running = running.clone();
    ArcTester::new(move || running.load(Ordering::Acquire))
};

// ArcTester 可以被克隆和在多个线程间共享
let tester_clone = tester.clone();

let result = DoubleCheckedLockExecutor::on(&data)
    .when(tester)
    .call_mut(|value| {
        *value += 1;
        Ok::<_, std::io::Error>(*value)
    });
```

### 4. 组合多个 Tester

使用逻辑操作符组合多个条件：

```rust
use prism3_function::BoxTester;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_concurrent::{DoubleCheckedLockExecutor, lock::ArcMutex};

let running = Arc::new(AtomicBool::new(true));
let ready = Arc::new(AtomicBool::new(true));
let data = ArcMutex::new(42);

let running_tester = {
    let running = running.clone();
    BoxTester::new(move || running.load(Ordering::Acquire))
};

let ready_tester = {
    let ready = ready.clone();
    move || ready.load(Ordering::Acquire)
};

// 组合两个 tester：两者都必须为 true
let combined_tester = running_tester.and(ready_tester);

let result = DoubleCheckedLockExecutor::on(&data)
    .when(combined_tester)
    .call_mut(|value| {
        *value += 1;
        Ok::<_, std::io::Error>(*value)
    });
```

## Tester 类型对比

| 类型 | 所有权 | 可克隆 | 线程安全 | 开销 | 适用场景 |
|------|--------|--------|----------|------|----------|
| 闭包 | 单一 | 否 | 取决于闭包 | 零 | 简单场景 |
| `BoxTester` | 单一 | 否 | 否 | 零 | 构建器模式 |
| `ArcTester` | 共享 | 是 | 是 | 原子引用计数 | 跨线程共享 |
| `RcTester` | 共享 | 是 | 否 | 引用计数 | 单线程共享 |

## 优势

1. **类型安全**：使用 trait 而不是裸闭包提供了更好的类型安全性
2. **灵活性**：支持多种 Tester 实现，适应不同场景
3. **可组合性**：可以使用逻辑操作符（`and`、`or`、`not`）组合多个 Tester
4. **一致性**：与 `prism3_function` 库中的其他函数式抽象保持一致

## 向后兼容性

由于 `Fn() -> bool` 自动实现了 `Tester` trait，现有的使用闭包的代码无需修改即可继续工作。

## 示例

完整的示例代码请参见：
- `examples/double_checked_lock_demo.rs` - 基本用法（使用闭包）
- `examples/double_checked_lock_with_tester_demo.rs` - 使用不同类型的 Tester

## 测试

所有现有测试都已通过，无需修改。运行测试：

```bash
cargo test
```

运行示例：

```bash
cargo run --example double_checked_lock_demo
cargo run --example double_checked_lock_with_tester_demo
```


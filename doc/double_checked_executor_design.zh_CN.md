# 双重检查锁执行器（Double-Checked Lock Executor）设计文档

## 1. 概述

本文档描述了如何将 Java 版本的 `DoubleCheckedLockExecutor` 移植到 Rust，充分利用 Rust 的类型系统、所有权模型和并发安全特性。

### 1.1 目标

- 提供与 Java 版本功能等价的 Rust 实现
- 利用 Rust 的编译期保证实现更强的线程安全
- 集成现有的 `prism3-rust-function` 和 `prism3-rust-clock` 组件
- 提供符合 Rust 习惯的 API 设计

### 1.2 适用场景

双重检查锁执行器适用于以下场景：

- 共享状态会在运行时发生变化（例如：服务的启动/停止状态）
- 大部分情况下条件满足，只有少数情况下条件不满足
- 需要在条件满足时执行需要同步保护的操作
- 需要最小化锁竞争，提高并发性能

## 2. Java 版本功能分析

### 2.1 核心功能

1. **双重检查锁模式**
   - 第一次检查：在锁外快速失败
   - 获取锁
   - 第二次检查：在锁内确认条件
   - 执行任务

2. **条件测试器**
   - `BooleanSupplier tester`：用于检查执行条件（Java）
   - Rust 移植：使用 `ArcTester`（来自 `prism3-rust-function`）
   - 依赖的共享状态必须通过线程安全类型（如 `Arc<AtomicBool>`、`Arc<Mutex<T>>`）保证可见性

3. **灵活的错误处理**
   - 可选的日志记录（logger、level、message）
   - 可选的异常抛出（errorSupplier）
   - 支持无栈异常以优化性能

4. **回滚机制**
   - `outsideAction`：锁外准备操作
   - `rollbackAction`：失败时的回滚操作
   - 自动处理回滚异常

5. **多种任务类型**
   - `execute()`：无返回值的任务（Runnable）
   - `call()`：有返回值的任务（Callable）
   - `executeIo()`：可能抛出 IOException 的任务
   - `callIo()`：可能抛出 IOException 且有返回值的任务

### 2.2 关键设计特性

- **Builder 模式**：流式 API 构建执行器
- **异常工厂**：避免并发场景下复用异常导致的栈覆盖问题
- **锁抽象**：支持普通 Lock、ReadLock、WriteLock
- **Result 包装**：封装成功/失败状态和返回值

## 3. Rust 移植方案

### 3.1 总体架构

```
DoubleCheckedLockExecutor
├── 核心配置
│   ├── tester: 条件测试函数
│   ├── logger: 日志配置（可选）
│   └── error_supplier: `ArcReadonlySupplier` 错误工厂（可选）
├── Builder 模式
│   └── 流式 API 构建
└── 执行方法
    ├── execute/call: 基础版本
    └── execute_with_rollback/call_with_rollback: 带回滚版本
```

### 3.2 模块结构

```
prism3-rust-concurrent/
├── src/
│   ├── lib.rs
│   ├── executor.rs           # 执行器接口抽象 trait（Runnable、Callable、Executor 等）
│   ├── lock.rs              # 锁包装器（ArcMutex、ArcRwLock、ArcAsyncMutex 等）
│   ├── double_checked_lock/   # 新增：双重检查锁执行器模块
│   │   ├── mod.rs           # 模块导出
│   │   ├── executor.rs      # 双重检查锁执行器实现
│   │   ├── builder.rs       # Builder 实现
│   │   ├── config.rs        # 配置结构体
│   │   ├── error.rs         # 错误类型定义
│   │   └── result.rs        # ExecutionResult 类型
│   └── traits/               # 可选：锁的抽象 trait
│       └── lock_trait.rs    # 锁的抽象 trait（可选）
├── tests/
│   └── double_checked_lock_tests.rs
├── examples/
│   └── double_checked_lock_demo.rs
└── doc/
    └── double_checked_executor_design.zh_CN.md  # 本文档
```

**现有代码说明：**
- `executor.rs` - 定义了执行器相关的抽象 trait，类似 JDK 的 Executor 接口
- `lock.rs` - 提供了同步和异步锁的包装器，简化锁的使用
- 新增的 `double_checked_lock/` 模块将实现双重检查锁执行器功能

## 4. 类型系统设计

### 4.1 核心结构体

```rust
use prism3_rust_function::ArcTester;

/// 双重检查锁执行器
///
/// 泛型参数：
/// - `E`: 条件不满足时返回的错误类型
pub struct DoubleCheckedLockExecutor<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// 条件测试器 - 测试执行条件是否满足
    /// 注意：此测试器依赖的共享状态必须通过 Arc<AtomicXxx> 或 Arc<Mutex<T>> 等
    /// 线程安全类型来保证可见性
    tester: ArcTester,

    /// 日志配置（可选）
    logger: Option<LogConfig>,

    /// 错误工厂 - 用于创建错误实例（可选）
    error_supplier: Option<ArcReadonlySupplier<E>>,

    /// 执行器配置
    config: ExecutorConfig,
}

/// 日志配置
pub struct LogConfig {
    /// 日志级别
    level: log::Level,

    /// 日志消息
    message: String,
}

/// 执行器配置
pub struct ExecutorConfig {
    /// 是否启用性能度量
    enable_metrics: bool,

    /// 是否禁用错误回溯（用于高性能场景）
    disable_backtrace: bool,
}
```

#### 为何使用 `ArcTester` 而不是 `BoxTester`？

`prism3-rust-function` 提供了三种 Tester 实现：`BoxTester`、`RcTester` 和 `ArcTester`。在 `DoubleCheckedLockExecutor` 中必须使用 `ArcTester`，原因如下：

**1. 线程安全保证（关键因素）**

```rust
// BoxTester - 不保证线程安全
pub struct BoxTester {
    func: Box<dyn Fn() -> bool>,  // ❌ 没有 Send + Sync 约束
}

// ArcTester - 编译期保证线程安全
pub struct ArcTester {
    func: Arc<dyn Fn() -> bool + Send + Sync>,  // ✅ 强制 Send + Sync
}
```

`ArcTester` 的 `Send + Sync` 约束确保在编译期捕获线程安全问题，防止意外捕获非线程安全的数据（如 `Rc`、`RefCell`）。

**2. 跨线程使用需求**

```rust
// 典型使用场景
pub struct Service {
    executor: DoubleCheckedLockExecutor<ServiceError>,  // 需要 Send
}

// 在多线程中使用
let service = Service::new();
std::thread::spawn(move || {
    service.set_pool_size(10);  // executor 必须实现 Send
});
```

**3. Arc 共享场景**

```rust
// 多线程共享同一个 Service
let service = Arc::new(Service::new());  // Service 必须实现 Sync

let s1 = service.clone();
let t1 = std::thread::spawn(move || s1.set_pool_size(10));

let s2 = service.clone();
let t2 = std::thread::spawn(move || s2.set_cache_size(20));
```

**对比总结**

| 特性 | BoxTester | ArcTester | 需求 |
|------|-----------|-----------|------|
| **Send** | ❌ 不保证 | ✅ 保证 | ✅ 必需 |
| **Sync** | ❌ 不保证 | ✅ 保证 | ✅ 必需 |
| **编译期检查** | ❌ 无 | ✅ 有 | ✅ 关键 |
| **Clone** | ❌ 不支持 | ✅ 支持 | ⚖️ 附带好处 |

**注意**：选择 `ArcTester` 的核心原因是**编译期线程安全保证**，而非克隆能力。实际使用中，`DoubleCheckedLockExecutor` 作为结构体字段只创建一次，不需要克隆。`Arc` 的引用计数开销在这个场景中完全可以接受。

### 4.2 结果类型

```rust
/// 任务执行结果
///
/// 类似 Java 版本的 `Result<T>` 类，但为了避免与 Rust 标准库的 `Result` 混淆，
/// 命名为 `ExecutionResult`
pub struct ExecutionResult<T> {
    /// 执行是否成功
    pub success: bool,

    /// 成功时的返回值（仅当 success = true 时有值）
    pub value: Option<T>,

    /// 失败时的错误信息（仅当 success = false 时有值）
    pub error: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl<T> ExecutionResult<T> {
    /// 创建成功结果
    pub fn succeed(value: T) -> Self {
        Self {
            success: true,
            value: Some(value),
            error: None,
        }
    }

    /// 创建失败结果
    pub fn fail() -> Self {
        Self {
            success: false,
            value: None,
            error: None,
        }
    }

    /// 创建带错误信息的失败结果
    pub fn fail_with_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            success: false,
            value: None,
            error: Some(Box::new(error)),
        }
    }
}
```

### 4.3 错误类型

```rust
/// 执行器错误类型
#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    /// 条件不满足
    #[error("Double-checked lock condition not met")]
    ConditionNotMet,

    /// 条件不满足，带自定义消息
    #[error("Double-checked lock condition not met: {0}")]
    ConditionNotMetWithMessage(String),

    /// 任务执行失败
    #[error("Task execution failed: {0}")]
    TaskFailed(String),

    /// 回滚操作失败
    #[error("Rollback failed: original error = {original}, rollback error = {rollback}")]
    RollbackFailed {
        original: String,
        rollback: String,
    },

    /// 锁中毒（Mutex/RwLock poison）
    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),
}

/// Builder 错误类型
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    /// 缺少必需的 tester 参数
    #[error("Tester function is required")]
    MissingTester,
}
```

## 5. 锁抽象设计

### 5.1 现有 Lock 模块设计

本项目的 `lock` 模块已经提供了一套完善的锁抽象系统，包括：

**Trait 定义：**
- `Lock<T>` - 同步锁 trait（为 `std::sync::Mutex<T>` 实现）
- `ReadWriteLock<T>` - 同步读写锁 trait（为 `std::sync::RwLock<T>` 实现）
- `AsyncLock<T>` - 异步锁 trait（为 `tokio::sync::Mutex<T>` 实现）
- `AsyncReadWriteLock<T>` - 异步读写锁 trait（为 `tokio::sync::RwLock<T>` 实现）

**包装器实现：**
- `ArcMutex<T>` - Arc 包装的同步互斥锁
- `ArcRwLock<T>` - Arc 包装的同步读写锁
- `ArcAsyncMutex<T>` - Arc 包装的异步互斥锁
- `ArcAsyncRwLock<T>` - Arc 包装的异步读写锁

**设计哲学：**
- 通过闭包隐藏 Guard 生命周期复杂性
- RAII 自动释放锁
- 提供统一的 API 接口
- 支持编写泛型代码

### 5.2 Lock Trait API

```rust
/// 同步锁 trait
pub trait Lock<T: ?Sized> {
    /// 获取锁并执行闭包
    fn with_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;

    /// 尝试获取锁（非阻塞）
    fn try_with_lock<R, F>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R;
}

/// 同步读写锁 trait
pub trait ReadWriteLock<T: ?Sized> {
    /// 获取读锁并执行闭包
    fn with_read_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R;

    /// 获取写锁并执行闭包
    fn with_write_lock<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R;
}
```

### 5.3 双重检查锁执行器集成策略

**方案选择**：直接使用 `Lock` 和 `ReadWriteLock` trait 作为泛型约束

**优点**：
- 充分利用现有的锁抽象
- 支持所有实现了这些 trait 的锁类型
- API 设计一致，易于使用和理解
- 自动处理锁的生命周期

**支持的锁类型**：
```rust
// 标准库类型（通过 trait 实现）
use std::sync::{Mutex, RwLock};

// 包装器类型（推荐用于跨线程共享）
use crate::lock::{ArcMutex, ArcRwLock};

// 异步版本（未来扩展）
use crate::lock::{ArcAsyncMutex, ArcAsyncRwLock};
```

## 6. API 设计

### 6.1 Builder API

```rust
use prism3_rust_function::{ArcTester, Tester};

impl<E> DoubleCheckedLockExecutor<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// 创建 Builder
    pub fn builder() -> Builder<E> {
        Builder::default()
    }
}

/// Builder 构建器
pub struct Builder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    tester: Option<ArcTester>,
    logger: Option<LogConfig>,
    error_supplier: Option<ArcReadonlySupplier<E>>,
    config: ExecutorConfig,
}

impl<E> Builder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// 设置条件测试器（必需）
    ///
    /// 接受一个 `ArcTester` 实例，用于测试执行条件是否满足。
    ///
    /// **重要**：测试器依赖的共享状态必须通过 `Arc<AtomicBool>`、`Arc<Mutex<T>>`
    /// 或 `Arc<RwLock<T>>` 等线程安全类型来保证跨线程可见性
    ///
    /// # 参数
    ///
    /// * `tester` - 条件测试器
    ///
    /// # 示例
    ///
    /// ```rust
    /// use prism3_rust_function::ArcTester;
    /// use std::sync::{Arc, RwLock};
    ///
    /// let state = Arc::new(RwLock::new(State::Running));
    /// let state_clone = state.clone();
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester(ArcTester::new(move || {
    ///         matches!(*state_clone.read().unwrap(), State::Running)
    ///     }))
    ///     .build()?;
    /// ```
    pub fn tester(mut self, tester: ArcTester) -> Self {
        self.tester = Some(tester);
        self
    }

    /// 设置条件测试闭包（便捷方法）
    ///
    /// 接受一个闭包，内部自动创建 `ArcTester`。
    ///
    /// # 参数
    ///
    /// * `f` - 测试条件的闭包
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    ///
    /// let running = Arc::new(AtomicBool::new(true));
    /// let running_clone = running.clone();
    ///
    /// let executor = DoubleCheckedLockExecutor::builder()
    ///     .tester_fn(move || running_clone.load(Ordering::Acquire))
    ///     .build()?;
    /// ```
    pub fn tester_fn<F>(mut self, f: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.tester = Some(ArcTester::new(f));
        self
    }

    /// 设置日志记录器（可选）
    pub fn logger(mut self, level: log::Level, message: impl Into<String>) -> Self {
        self.logger = Some(LogConfig {
            level,
            message: message.into(),
        });
        self
    }

    /// 设置错误工厂（可选）
    pub fn error_supplier<F>(mut self, f: F) -> Self
    where
        F: Fn() -> E + Send + Sync + 'static,
    {
        self.error_supplier = Some(ArcReadonlySupplier::new(f));
        self
    }

    /// 设置错误消息（便捷方法，用于简单场景）
    pub fn error_message(mut self, message: impl Into<String>) -> Self
    where
        E: From<ExecutorError>,
    {
        let msg = message.into();
        self.error_supplier = Some(ArcReadonlySupplier::new(move || {
            E::from(ExecutorError::ConditionNotMetWithMessage(msg.clone()))
        }));
        self
    }

    /// 启用性能度量（可选）
    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.config.enable_metrics = enable;
        self
    }

    /// 禁用错误回溯以提升性能（可选）
    pub fn disable_backtrace(mut self, disable: bool) -> Self {
        self.config.disable_backtrace = disable;
        self
    }

    /// 构建执行器
    pub fn build(self) -> Result<DoubleCheckedLockExecutor<E>, BuilderError> {
        let tester = self.tester.ok_or(BuilderError::MissingTester)?;

        Ok(DoubleCheckedLockExecutor {
            tester,
            logger: self.logger,
            error_supplier: self.error_supplier,
            config: self.config,
        })
    }
}
```

### 6.2 执行方法

```rust
use crate::lock::{Lock, ReadWriteLock};

impl<E> DoubleCheckedLockExecutor<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    // ==================== 基础版本（使用 Lock trait）====================

    /// 使用互斥锁执行无返回值的任务
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `Lock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `Lock<T>` trait 的锁引用
    /// - `task`: 要执行的任务，返回 `Result<(), Box<dyn Error>>`
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<()>`，其中 `success` 字段指示是否成功执行
    ///
    /// # 执行流程
    /// 1. 第一次检查条件（锁外快速失败）
    /// 2. 获取锁
    /// 3. 第二次检查条件（锁内确认）
    /// 4. 执行任务
    pub fn execute<L, T, F>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<()>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    {
        self.call(lock, |t| {
            task(t)?;
            Ok(())
        })
    }

    /// 使用互斥锁执行有返回值的任务
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `Lock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `R`: 任务返回值类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `Lock<T>` trait 的锁引用
    /// - `task`: 要执行的任务，返回 `Result<R, Box<dyn Error>>`
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<R>`，包含任务的返回值（如果成功）
    pub fn call<L, T, F, R>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<R>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        // 第一次检查：锁外快速失败
        if !(self.tester)() {
            self.handle_condition_not_met();
            return ExecutionResult::fail();
        }

        // 使用 Lock trait 的 with_lock 方法获取锁并执行
        lock.with_lock(|data| {
            // 第二次检查：锁内再次确认
            if !(self.tester)() {
                self.handle_condition_not_met();
                return ExecutionResult::fail();
            }

            // 执行任务
            match task(data) {
                Ok(value) => ExecutionResult::succeed(value),
                Err(e) => ExecutionResult::fail_with_error(
                    ExecutorError::TaskFailed(e.to_string())
                ),
            }
        })
    }

    // ==================== 读写锁版本 ====================

    /// 使用读写锁的写锁执行任务
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `ReadWriteLock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `R`: 任务返回值类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `ReadWriteLock<T>` trait 的锁引用
    /// - `task`: 要执行的任务，返回 `Result<R, Box<dyn Error>>`
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<R>`，包含任务的返回值（如果成功）
    pub fn call_with_write_lock<L, T, F, R>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<R>
    where
        L: ReadWriteLock<T>,
        F: FnOnce(&mut T) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        // 第一次检查：锁外快速失败
        if !(self.tester)() {
            self.handle_condition_not_met();
            return ExecutionResult::fail();
        }

        // 使用 ReadWriteLock trait 的 with_write_lock 方法
        lock.with_write_lock(|data| {
            // 第二次检查：锁内再次确认
            if !(self.tester)() {
                self.handle_condition_not_met();
                return ExecutionResult::fail();
            }

            // 执行任务
            match task(data) {
                Ok(value) => ExecutionResult::succeed(value),
                Err(e) => ExecutionResult::fail_with_error(
                    ExecutorError::TaskFailed(e.to_string())
                ),
            }
        })
    }

    /// 使用读写锁的读锁执行任务（适用于只读操作）
    ///
    /// # 注意
    /// 此方法使用读锁，任务闭包只能读取数据，不能修改
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `ReadWriteLock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `R`: 任务返回值类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `ReadWriteLock<T>` trait 的锁引用
    /// - `task`: 要执行的任务，返回 `Result<R, Box<dyn Error>>`
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<R>`，包含任务的返回值（如果成功）
    pub fn call_with_read_lock<L, T, F, R>(
        &self,
        lock: &L,
        task: F,
    ) -> ExecutionResult<R>
    where
        L: ReadWriteLock<T>,
        F: FnOnce(&T) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        // 第一次检查：锁外快速失败
        if !(self.tester)() {
            self.handle_condition_not_met();
            return ExecutionResult::fail();
        }

        // 使用 ReadWriteLock trait 的 with_read_lock 方法
        lock.with_read_lock(|data| {
            // 第二次检查：锁内再次确认
            if !(self.tester)() {
                self.handle_condition_not_met();
                return ExecutionResult::fail();
            }

            // 执行任务
            match task(data) {
                Ok(value) => ExecutionResult::succeed(value),
                Err(e) => ExecutionResult::fail_with_error(
                    ExecutorError::TaskFailed(e.to_string())
                ),
            }
        })
    }

    // ==================== 带回滚版本 ====================

    /// 使用互斥锁执行任务，并提供回滚机制
    ///
    /// # 执行流程
    /// 1. 检查条件是否满足，若不满足则失败
    /// 2. 若条件满足，先执行 `outside_action`
    /// 3. 然后获取锁，再次检查条件：
    ///    - 若不满足，则释放锁后执行 `rollback_action` 并失败
    ///    - 若满足，则执行 `task`
    /// 4. 若 `task` 执行时抛出异常，则释放锁后执行 `rollback_action`
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `Lock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `O`: 锁外操作闭包类型
    /// - `R`: 回滚操作闭包类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `Lock<T>` trait 的锁引用
    /// - `task`: 要执行的核心任务
    /// - `outside_action`: 锁外准备操作
    /// - `rollback_action`: 失败时的回滚操作
    ///
    /// # 死锁警告
    /// `outside_action` 在获取锁**之前**执行，因此**禁止**在此操作中尝试获取
    /// 相同的锁或任何可能形成锁环的其他锁，否则会导致死锁！
    pub fn execute_with_rollback<L, T, F, O, R>(
        &self,
        lock: &L,
        task: F,
        outside_action: O,
        rollback_action: R,
    ) -> ExecutionResult<()>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
        O: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
        R: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    {
        self.call_with_rollback(
            lock,
            |t| {
                task(t)?;
                Ok(())
            },
            outside_action,
            rollback_action,
        )
    }

    /// 使用互斥锁执行有返回值的任务，并提供回滚机制
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `Lock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `O`: 锁外操作闭包类型
    /// - `Rb`: 回滚操作闭包类型
    /// - `V`: 任务返回值类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `Lock<T>` trait 的锁引用
    /// - `task`: 要执行的核心任务
    /// - `outside_action`: 锁外准备操作
    /// - `rollback_action`: 失败时的回滚操作
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<V>`，包含任务的返回值（如果成功）
    pub fn call_with_rollback<L, T, F, O, Rb, V>(
        &self,
        lock: &L,
        task: F,
        outside_action: O,
        rollback_action: Rb,
    ) -> ExecutionResult<V>
    where
        L: Lock<T>,
        F: FnOnce(&mut T) -> Result<V, Box<dyn std::error::Error + Send + Sync>>,
        O: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
        Rb: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    {
        // 第一次检查
        if !(self.tester)() {
            self.handle_condition_not_met();
            return ExecutionResult::fail();
        }

        // 执行锁外操作
        if let Err(e) = outside_action() {
            log::error!("Outside action failed: {}", e);
            return ExecutionResult::fail_with_error(
                ExecutorError::TaskFailed(e.to_string())
            );
        }

        // 使用 Lock trait 的 with_lock 方法
        // 注意：这里无法直接在闭包内部处理回滚，需要特殊处理
        let result = lock.with_lock(|data| {
            // 第二次检查
            if !(self.tester)() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }

            // 执行任务
            task(data).map_err(|e| ExecutorError::TaskFailed(e.to_string()))
        });

        // 处理结果和回滚
        match result {
            Ok(value) => ExecutionResult::succeed(value),
            Err(e) => {
                let error_msg = e.to_string();
                self.run_rollback(&rollback_action, Some(&error_msg));
                ExecutionResult::fail_with_error(e)
            }
        }
    }

    /// 使用读写锁的写锁执行任务，并提供回滚机制
    ///
    /// # 泛型参数
    /// - `L`: 实现了 `ReadWriteLock<T>` trait 的锁类型
    /// - `T`: 被锁保护的数据类型
    /// - `F`: 任务闭包类型
    /// - `O`: 锁外操作闭包类型
    /// - `Rb`: 回滚操作闭包类型
    /// - `V`: 任务返回值类型
    ///
    /// # 参数
    /// - `lock`: 实现了 `ReadWriteLock<T>` trait 的锁引用
    /// - `task`: 要执行的核心任务
    /// - `outside_action`: 锁外准备操作
    /// - `rollback_action`: 失败时的回滚操作
    ///
    /// # 返回值
    /// 返回 `ExecutionResult<V>`，包含任务的返回值（如果成功）
    pub fn call_with_rollback_write_lock<L, T, F, O, Rb, V>(
        &self,
        lock: &L,
        task: F,
        outside_action: O,
        rollback_action: Rb,
    ) -> ExecutionResult<V>
    where
        L: ReadWriteLock<T>,
        F: FnOnce(&mut T) -> Result<V, Box<dyn std::error::Error + Send + Sync>>,
        O: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
        Rb: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    {
        // 第一次检查
        if !(self.tester)() {
            self.handle_condition_not_met();
            return ExecutionResult::fail();
        }

        // 执行锁外操作
        if let Err(e) = outside_action() {
            log::error!("Outside action failed: {}", e);
            return ExecutionResult::fail_with_error(
                ExecutorError::TaskFailed(e.to_string())
            );
        }

        // 使用 ReadWriteLock trait 的 with_write_lock 方法
        let result = lock.with_write_lock(|data| {
            // 第二次检查
            if !(self.tester)() {
                self.handle_condition_not_met();
                return Err(ExecutorError::ConditionNotMet);
            }

            // 执行任务
            task(data).map_err(|e| ExecutorError::TaskFailed(e.to_string()))
        });

        // 处理结果和回滚
        match result {
            Ok(value) => ExecutionResult::succeed(value),
            Err(e) => {
                let error_msg = e.to_string();
                self.run_rollback(&rollback_action, Some(&error_msg));
                ExecutionResult::fail_with_error(e)
            }
        }
    }

    // ==================== 内部辅助方法 ====================

    /// 处理条件不满足的情况
    fn handle_condition_not_met(&self) {
        // 记录日志
        if let Some(ref log_config) = self.logger {
            log::log!(log_config.level, "{}", log_config.message);
        }

        // 抛出错误（如果配置了 `error_supplier`/`ArcReadonlySupplier`）
        // 注意：Rust 中不能直接抛出异常，这里通过返回值传递错误
        // 实际实现中，可以将 `error_supplier` 或 `ArcReadonlySupplier` 的结果存储到 ExecutionResult 中
    }

    /// 执行回滚操作
    fn run_rollback<R>(
        &self,
        rollback_action: R,
        original_error: Option<&str>,
    ) where
        R: FnOnce() -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
    {
        if let Err(e) = rollback_action() {
            if let Some(original) = original_error {
                log::warn!(
                    "Rollback failed during error recovery: {}. Original error: {}",
                    e,
                    original
                );
            } else {
                log::error!("Rollback failed: {}", e);
            }
        }
    }
}
```

### 6.3 集成 prism3-rust-clock

```rust
use prism3_rust_clock::meter::TimeMeter;

impl<E> DoubleCheckedLockExecutor<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// 执行任务并度量时间
    pub fn call_with_metrics_mutex<T, F, R>(
        &self,
        mutex: &Mutex<T>,
        task: F,
        meter: &mut TimeMeter,
    ) -> ExecutionResult<R>
    where
        F: FnOnce(&mut T) -> Result<R, Box<dyn std::error::Error + Send + Sync>>,
    {
        meter.start();
        let result = self.call_mutex(mutex, task);
        meter.stop();
        result
    }
}
```

### 6.4 集成 prism3-rust-function

```rust
use prism3_rust_function::{Predicate, Supplier, Consumer};

impl<E> Builder<E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    /// 使用 Predicate trait 设置测试函数
    pub fn tester_predicate<P>(mut self, predicate: P) -> Self
    where
        P: Predicate<Input = ()> + Send + Sync + 'static,
    {
        self.tester = Some(Arc::new(move || predicate.test(&())));
        self
    }
}
```

## 7. 使用示例

### 7.1 基础用法

```rust
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_rust_concurrent::{
    DoubleCheckedLockExecutor,
    lock::{ArcMutex, ArcRwLock, Lock, ReadWriteLock},
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Stopped,
    Running,
    Stopping,
}

struct Service {
    // 使用 AtomicBool 作为运行状态（双重检查的条件）
    running: Arc<AtomicBool>,
    // 使用 ArcMutex 保护可变状态
    pool_size: ArcMutex<i32>,
    cache_size: ArcMutex<i32>,
    // 双重检查锁执行器
    executor: DoubleCheckedLockExecutor<ServiceError>,
}

#[derive(Debug, thiserror::Error)]
enum ServiceError {
    #[error("Service is not running")]
    NotRunning,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

impl Service {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let running = Arc::new(AtomicBool::new(false));
        let running_clone = running.clone();

        // 构建执行器
        let executor = DoubleCheckedLockExecutor::builder()
            .tester_fn(move || running_clone.load(Ordering::Acquire))
            .logger(
                log::Level::Error,
                "Cannot modify service while it is not running",
            )
            .error_supplier(|| ServiceError::NotRunning)
            .build()?;

        Ok(Self {
            running,
            pool_size: ArcMutex::new(0),
            cache_size: ArcMutex::new(0),
            executor,
        })
    }

    /// 启动服务
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// 停止服务
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// 设置线程池大小
    pub fn set_pool_size(&self, size: i32) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.executor.execute(&self.pool_size, |pool_size| {
            if size <= 0 {
                return Err(Box::new(ServiceError::InvalidParameter(
                    "pool size must be positive".to_string()
                )) as Box<dyn std::error::Error + Send + Sync>);
            }
            *pool_size = size;
            Ok(())
        });

        if result.success {
            Ok(())
        } else {
            Err("Failed to set pool size".into())
        }
    }

    /// 获取线程池大小
    pub fn get_pool_size(&self) -> Option<i32> {
        let result = self.executor.call(&self.pool_size, |pool_size| {
            Ok(*pool_size)
        });

        result.value
    }

    /// 设置缓存大小
    pub fn set_cache_size(&self, size: i32) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.executor.execute(&self.cache_size, |cache_size| {
            if size <= 0 {
                return Err(Box::new(ServiceError::InvalidParameter(
                    "cache size must be positive".to_string()
                )) as Box<dyn std::error::Error + Send + Sync>);
            }
            *cache_size = size;
            Ok(())
        });

        if result.success {
            Ok(())
        } else {
            Err("Failed to set cache size".into())
        }
    }
}

// 使用示例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = Arc::new(Service::new()?);

    // 启动服务
    service.start();

    // 在多个线程中并发修改配置
    let s1 = service.clone();
    let t1 = std::thread::spawn(move || {
        s1.set_pool_size(10).unwrap();
    });

    let s2 = service.clone();
    let t2 = std::thread::spawn(move || {
        s2.set_cache_size(100).unwrap();
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("Pool size: {:?}", service.get_pool_size());

    // 停止服务后尝试修改配置（应该失败）
    service.stop();

    match service.set_pool_size(20) {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected failure: {}", e),
    }

    Ok(())
}
```

### 7.2 带回滚机制的用法

```rust
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex};
use std::cell::RefCell;
use prism3_rust_concurrent::{
    DoubleCheckedLockExecutor,
    lock::{ArcMutex, Lock},
};

#[derive(Clone, Debug)]
struct Resource {
    id: u64,
    // ... 其他字段
}

impl Resource {
    /// 分配资源（可能耗时）
    fn allocate() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 模拟资源分配
        Ok(Resource { id: 1 })
    }

    /// 释放资源
    fn deallocate(self) {
        // 模拟资源释放
        println!("Releasing resource {}", self.id);
    }
}

struct ResourceManager {
    active: Arc<AtomicBool>,
    resources: ArcMutex<Vec<Resource>>,
    executor: DoubleCheckedLockExecutor<ManagerError>,
}

#[derive(Debug, thiserror::Error)]
enum ManagerError {
    #[error("Manager is not active")]
    NotActive,
}

impl ResourceManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let active = Arc::new(AtomicBool::new(true));
        let active_clone = active.clone();

        let executor = DoubleCheckedLockExecutor::builder()
            .tester_fn(move || active_clone.load(Ordering::Acquire))
            .logger(log::Level::Error, "Resource manager is not active")
            .error_supplier(|| ManagerError::NotActive)
            .build()?;

        Ok(Self {
            active,
            resources: ArcMutex::new(Vec::new()),
            executor,
        })
    }

    /// 分配资源
    pub fn allocate_resource(&self) -> Result<Resource, Box<dyn std::error::Error>> {
        // 使用 RefCell 在闭包间共享临时资源
        let temp_resource = RefCell::new(None);

        let result = self.executor.call_with_rollback(
            &self.resources,
            |resources| {
                // 锁内操作：将预分配的资源添加到资源池
                let resource = temp_resource.borrow_mut().take().unwrap();
                resources.push(resource.clone());
                Ok(resource)
            },
            || {
                // 锁外操作：预分配资源（可能耗时）
                let resource = Resource::allocate()?;
                *temp_resource.borrow_mut() = Some(resource);
                Ok(())
            },
            || {
                // 回滚操作：释放预分配的资源
                if let Some(resource) = temp_resource.borrow_mut().take() {
                    resource.deallocate();
                }
                Ok(())
            },
        );

        result.value.ok_or_else(|| "Failed to allocate resource".into())
    }

    /// 激活管理器
    pub fn activate(&self) {
        self.active.store(true, Ordering::Release);
    }

    /// 停用管理器
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }
}

// 使用示例
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Arc::new(ResourceManager::new()?);

    // 正常分配资源
    match manager.allocate_resource() {
        Ok(resource) => println!("Allocated resource: {:?}", resource),
        Err(e) => println!("Failed to allocate: {}", e),
    }

    // 停用后尝试分配（应该失败，且触发回滚）
    manager.deactivate();

    match manager.allocate_resource() {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!("Expected failure: {}", e),
    }

    Ok(())
}
```

### 7.3 集成时间度量

```rust
use prism3_rust_clock::meter::TimeMeter;
use prism3_rust_concurrent::{
    DoubleCheckedLockExecutor,
    lock::{ArcMutex, Lock},
};

// 时间度量可以在任务闭包内部进行
fn measure_execution() {
    let executor = DoubleCheckedLockExecutor::builder()
        .tester_fn(|| true)
        .build()
        .unwrap();

    let data = ArcMutex::new(vec![1, 2, 3, 4, 5]);
    let mut meter = TimeMeter::new();

    meter.start();
    let result = executor.call(&data, |vec| {
        // 执行耗时操作
        vec.iter().sum::<i32>();
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(vec.len())
    });
    meter.stop();

    if result.success {
        println!("Execution time: {:?}", meter.elapsed());
        println!("Result: {:?}", result.value);
    }
}
```

### 7.4 使用读写锁

```rust
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_rust_concurrent::{
    DoubleCheckedLockExecutor,
    lock::{ArcRwLock, ReadWriteLock},
};

struct Cache {
    active: Arc<AtomicBool>,
    data: ArcRwLock<HashMap<String, String>>,
    executor: DoubleCheckedLockExecutor<CacheError>,
}

#[derive(Debug, thiserror::Error)]
enum CacheError {
    #[error("Cache is not active")]
    NotActive,
}

impl Cache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let active = Arc::new(AtomicBool::new(true));
        let active_clone = active.clone();

        let executor = DoubleCheckedLockExecutor::builder()
            .tester_fn(move || active_clone.load(Ordering::Acquire))
            .error_supplier(|| CacheError::NotActive)
            .build()?;

        Ok(Self {
            active,
            data: ArcRwLock::new(HashMap::new()),
            executor,
        })
    }

    /// 读取缓存（使用读锁）
    pub fn get(&self, key: &str) -> Option<String> {
        let key = key.to_string();
        let result = self.executor.call_with_read_lock(&self.data, |cache| {
            Ok(cache.get(&key).cloned())
        });

        result.value.flatten()
    }

    /// 写入缓存（使用写锁）
    pub fn set(&self, key: String, value: String) -> Result<(), Box<dyn std::error::Error>> {
        let result = self.executor.call_with_write_lock(&self.data, |cache| {
            cache.insert(key, value);
            Ok(())
        });

        if result.success {
            Ok(())
        } else {
            Err("Failed to set cache value".into())
        }
    }
}
```

## 8. Java vs Rust 关键差异对照

| 特性 | Java 版本 | Rust 版本 | 说明 |
|-----|---------|---------|------|
| **异常处理** | 异常（Exception） | `Result<T, E>` | Rust 使用显式错误处理，更安全 |
| **锁类型** | `Lock` 接口 | `Lock<T>` / `ReadWriteLock<T>` trait | Rust 使用 trait 统一锁接口 |
| **锁实现** | `ReentrantLock` 等 | `Mutex<T>` / `RwLock<T>` | Rust 的锁直接保护数据 |
| **锁包装** | 无需包装 | `ArcMutex<T>` / `ArcRwLock<T>` | Rust 需要 Arc 包装以便跨线程共享 |
| **锁卫士** | 手动 lock/unlock | RAII Guard（隐藏在 trait 中） | Rust 自动释放锁，避免忘记 unlock |
| **闭包风格** | Lambda 接受锁卫士 | 闭包接受数据引用 | Rust 通过 trait 隐藏 Guard 生命周期 |
| **闭包类型** | Lambda 表达式 | `Fn` / `FnOnce` / `FnMut` | Rust 需要明确所有权和生命周期 |
| **Volatile** | `volatile` 字段 | `Arc<AtomicXxx>` | Rust 通过类型系统保证可见性 |
| **线程安全** | `synchronized` / `volatile` | `Send` + `Sync` trait | 编译期检查，零运行时开销 |
| **日志** | SLF4J | `log` crate | Rust 生态标准 |
| **Builder** | 可选参数 | `Option<T>` | 必须显式处理 None 情况 |
| **错误栈** | `fillInStackTrace()` | `std::backtrace` | Rust 1.65+ 稳定 |
| **泛型** | 类型擦除 | 单态化 | Rust 保留类型信息，性能更好 |
| **API 风格** | 方法重载 | 不同方法名 | Rust 不支持重载，使用泛型约束 |

## 9. 重要注意事项

### 9.1 使用 Lock Trait 的优势

使用 `Lock` 和 `ReadWriteLock` trait 的好处：

```rust
// ✅ 推荐：使用 trait，自动管理锁的生命周期
use prism3_rust_concurrent::lock::{Lock, ArcMutex};

let data = ArcMutex::new(42);
executor.call(&data, |value| {
    *value += 1;  // 闭包自动接收 &mut i32
    Ok(*value)
}); // 锁自动释放

// ❌ 不推荐：手动管理锁（容易出错）
let guard = data.lock().unwrap();
*guard += 1;
drop(guard);  // 可能忘记释放
```

### 9.2 Volatile 替代方案

Java 的 `volatile` 在 Rust 中没有直接对应物，需要使用以下方案：

```rust
// ✅ 推荐方案1：AtomicBool（适用于简单布尔状态）
use std::sync::atomic::{AtomicBool, Ordering};
let state = Arc::new(AtomicBool::new(false));
builder.tester_fn(move || state.load(Ordering::Acquire))

// ✅ 推荐方案2：ArcMutex（适用于复杂状态）
use prism3_rust_concurrent::lock::ArcMutex;
let state = ArcMutex::new(State::Running);
let state_clone = state.clone();
builder.tester_fn(move || {
    state_clone.with_lock(|s| *s == State::Running)
})

// ✅ 推荐方案3：ArcRwLock（读多写少场景）
use prism3_rust_concurrent::lock::ArcRwLock;
let state = ArcRwLock::new(State::Running);
let state_clone = state.clone();
builder.tester_fn(move || {
    state_clone.with_read_lock(|s| *s == State::Running)
})

// ❌ 错误方案：普通 Rc<RefCell<T>>（不是线程安全的）
let state = Rc::new(RefCell::new(State::Running)); // 编译错误！
```

### 9.3 选择合适的锁类型

根据使用场景选择锁类型：

| 场景 | 推荐锁类型 | 原因 |
|-----|----------|------|
| 简单互斥访问 | `ArcMutex<T>` | 简单直接，性能好 |
| 读多写少 | `ArcRwLock<T>` | 允许并发读取 |
| 条件状态检查 | `Arc<AtomicBool>` | 无锁，性能最优 |
| 异步场景 | `ArcAsyncMutex<T>` | 不阻塞异步运行时 |

### 9.4 死锁预防

在使用 `execute_with_rollback` 或 `call_with_rollback` 时：

- ✅ `outside_action` 中可以进行文件 I/O、网络请求等无锁操作
- ✅ `outside_action` 中可以获取**不同**的锁（注意锁顺序）
- ❌ `outside_action` 中**禁止**获取 `lock` 参数指定的锁
- ❌ `outside_action` 中**禁止**获取可能形成锁环的其他锁

### 9.5 性能考虑

1. **高并发失败场景**：如果预期条件检查会频繁失败，考虑：
   - 使用 `disable_backtrace(true)` 禁用回溯
   - 避免使用 `error_supplier`，仅通过返回值判断

2. **锁竞争**：任务应尽量简短，避免在锁内执行耗时操作

3. **日志开销**：在性能敏感场景，考虑使用条件编译或运行时开关控制日志

4. **使用 Lock Trait 的性能**：通过 trait 的闭包方式与手动管理锁性能相当，因为：
   - 闭包会被内联优化
   - RAII Guard 在编译期优化
   - 零成本抽象

### 9.6 错误处理最佳实践

```rust
use prism3_rust_concurrent::lock::{Lock, ArcMutex};

// ✅ 推荐：使用 ExecutionResult 检查成功状态
let data = ArcMutex::new(42);
let result = executor.call(&data, |value| Ok(*value));
if result.success {
    let value = result.value.unwrap();
    println!("Value: {}", value);
} else {
    if let Some(error) = result.error {
        log::error!("Execution failed: {}", error);
    }
}

// ✅ 推荐：转换为标准 Result
impl<T> ExecutionResult<T> {
    pub fn into_result(self) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        if self.success {
            Ok(self.value.unwrap())
        } else {
            Err(self.error.unwrap_or_else(|| "Unknown error".into()))
        }
    }
}

// 使用
let value = executor.call(&data, |v| Ok(*v)).into_result()?;
```

## 10. 实施计划

### 10.1 第一阶段：基础实现

- [ ] 创建模块结构
- [ ] 实现 `ExecutionResult` 类型
- [ ] 实现 `ExecutorError` 和 `BuilderError`
- [ ] 实现 `LogConfig` 和 `ExecutorConfig`
- [ ] 实现 `DoubleCheckedLockExecutor` 结构体
- [ ] 实现 `Builder` 及其方法

### 10.2 第二阶段：核心功能

- [ ] 实现 `execute()` 方法（使用 `Lock` trait）
- [ ] 实现 `call()` 方法（使用 `Lock` trait）
- [ ] 实现 `call_with_write_lock()` 方法（使用 `ReadWriteLock` trait）
- [ ] 实现 `call_with_read_lock()` 方法（使用 `ReadWriteLock` trait）
- [ ] 实现内部辅助方法（`handle_condition_not_met`、`run_rollback`）

### 10.3 第三阶段：高级功能

- [ ] 实现 `execute_with_rollback()` 方法
- [ ] 实现 `call_with_rollback()` 方法
- [ ] 实现 `call_with_rollback_write_lock()` 方法
- [ ] 集成 `prism3-rust-clock` 的 `TimeMeter`（可选）
- [ ] 实现异步版本支持（`AsyncLock` trait）（未来）

### 10.4 第四阶段：测试和文档

- [ ] 编写单元测试
- [ ] 编写集成测试
- [ ] 编写并发测试
- [ ] 编写使用示例
- [ ] 编写 API 文档注释
- [ ] 编写 README

### 10.5 第五阶段：优化和完善

- [ ] 性能基准测试
- [ ] 代码覆盖率测试
- [ ] 错误处理完善
- [ ] 日志输出优化
- [ ] 用户反馈收集

## 11. 开放问题

### 11.1 已解决的设计问题

1. **锁抽象层级** ✅ 已解决
   - 使用 `Lock<T>` 和 `ReadWriteLock<T>` trait 统一接口
   - 通过泛型约束支持所有实现了这些 trait 的锁类型
   - 闭包风格 API 隐藏 Guard 生命周期复杂性

2. **错误类型设计** ✅ 已确定
   - 使用 `thiserror` 定义精确的错误类型（`ExecutorError`、`BuilderError`）
   - 任务闭包返回 `Box<dyn Error + Send + Sync>` 保持灵活性
   - `ExecutionResult<T>` 封装成功/失败状态

3. **日志框架** ✅ 已确定
   - 使用 `log` facade，保持框架中立
   - 用户可以选择任何兼容的日志实现

### 11.2 待确认的技术细节

1. **异步支持**
   - 是否需要提供 async 版本（使用 `AsyncLock` trait）？
   - 如果需要，是作为单独的类型还是通过特性门控？
   - 异步版本的 API 设计是否需要特殊处理？

2. **parking_lot 支持**
   - 是否需要为 `parking_lot` 的锁类型提供 trait 实现？
   - 优点：更好的性能和更多特性
   - 缺点：增加依赖

3. **高级功能**
   - 是否需要提供无锁版本（仅用于基准测试对比）？
   - 是否需要提供宏来简化常见用法？
   - 是否需要支持超时机制？

4. **错误处理增强**
   - 是否需要错误消息的国际化（i18n）支持？
   - 是否需要提供错误重试机制？

## 12. 参考资料

### 12.1 项目内部资料

- [Java 版本源码](../external/common-java/src/main/java/ltd/qubit/commons/concurrent/DoubleCheckedLockExecutor.java)
- [prism3-rust-concurrent Lock 模块](../src/lock/)
  - [Lock trait](../src/lock/lock.rs)
  - [ReadWriteLock trait](../src/lock/read_write_lock.rs)
  - [AsyncLock trait](../src/lock/async_lock.rs)
  - [AsyncReadWriteLock trait](../src/lock/async_read_write_lock.rs)
  - [ArcMutex 包装器](../src/lock/arc_mutex.rs)
  - [ArcRwLock 包装器](../src/lock/arc_rw_lock.rs)
- [prism3-rust-function 文档](../prism3-rust-function/README.md)
- [prism3-rust-clock 文档](../prism3-rust-clock/README.md)

### 12.2 外部资料

- [Rust 并发模型](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [Rust Atomics and Locks](https://marabos.nl/atomics/)
- [std::sync 模块文档](https://doc.rust-lang.org/std/sync/)
- [tokio::sync 模块文档](https://docs.rs/tokio/latest/tokio/sync/)

## 13. 变更历史

| 版本 | 日期 | 作者 | 变更说明 |
|-----|------|------|---------|
| 1.0 | 2025-01-XX | AI Assistant | 初始版本 |
| 1.1 | 2025-01-22 | AI Assistant | 根据重构后的 lock 模块更新设计 |

**主要变更内容（v1.1）：**
- 更新锁抽象设计，使用 `Lock<T>` 和 `ReadWriteLock<T>` trait
- 更新 API 设计，使用闭包风格隐藏 Guard 生命周期
- 添加 `ArcMutex` 和 `ArcRwLock` 包装器的使用说明
- 更新所有示例代码以反映新的 API 设计
- 添加性能考虑和最佳实践
- 更新 Java vs Rust 对照表
- 添加选择锁类型的指导

---

**文档状态**：草案
**最后更新**：2025-01-22
**审阅者**：待定


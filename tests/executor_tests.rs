/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/

use prism3_concurrent::{AsyncExecutor, Callable, Executor, ExecutorService, Runnable};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// 简单的同步执行器，用于测试
struct SimpleExecutor {
    task_count: Arc<AtomicUsize>,
}

impl SimpleExecutor {
    fn new() -> Self {
        Self {
            task_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }
}

impl Executor for SimpleExecutor {
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        self.task_count.fetch_add(1, Ordering::SeqCst);
        task();
    }
}

/// 简单的异步执行器，用于测试
struct SimpleAsyncExecutor {
    task_count: Arc<AtomicUsize>,
}

impl SimpleAsyncExecutor {
    fn new() -> Self {
        Self {
            task_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }
}

impl AsyncExecutor for SimpleAsyncExecutor {
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.task_count.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(future);
    }
}

/// 可关闭的执行器，用于测试 ExecutorService
struct ShutdownableExecutor {
    task_count: Arc<AtomicUsize>,
    is_shutdown: Arc<AtomicBool>,
    is_terminated: Arc<AtomicBool>,
}

impl ShutdownableExecutor {
    fn new() -> Self {
        Self {
            task_count: Arc::new(AtomicUsize::new(0)),
            is_shutdown: Arc::new(AtomicBool::new(false)),
            is_terminated: Arc::new(AtomicBool::new(false)),
        }
    }

    fn task_count(&self) -> usize {
        self.task_count.load(Ordering::SeqCst)
    }
}

impl Executor for ShutdownableExecutor {
    fn execute(&self, task: Box<dyn FnOnce() + Send + 'static>) {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return; // 拒绝新任务
        }
        self.task_count.fetch_add(1, Ordering::SeqCst);
        task();
    }
}

impl ExecutorService for ShutdownableExecutor {
    fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    fn shutdown_now(&self) -> Vec<Box<dyn Runnable>> {
        self.is_shutdown.store(true, Ordering::SeqCst);
        self.is_terminated.store(true, Ordering::SeqCst);
        Vec::new() // 简化实现，不返回待执行任务
    }

    fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::SeqCst)
    }

    fn is_terminated(&self) -> bool {
        self.is_terminated.load(Ordering::SeqCst)
    }

    fn await_termination(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        let is_terminated = self.is_terminated.clone();
        Box::pin(async move {
            // 简化实现，立即标记为已终止
            is_terminated.store(true, Ordering::SeqCst);
        })
    }
}

/// 测试 Executor trait 的基本功能
///
/// 测试数据：创建一个简单的执行器，提交多个任务
/// 预期结果：所有任务都能正确执行，任务计数正确
#[test]
fn test_executor_execute() {
    let executor = SimpleExecutor::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // 提交3个任务
    for _ in 0..3 {
        let counter_clone = counter.clone();
        executor.execute(Box::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));
    }

    assert_eq!(executor.task_count(), 3);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

/// 测试 AsyncExecutor trait 的 spawn 方法
///
/// 测试数据：创建一个异步执行器，生成多个异步任务
/// 预期结果：所有异步任务都能正确生成，任务计数正确
#[tokio::test]
async fn test_async_executor_spawn() {
    let executor = SimpleAsyncExecutor::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // 生成3个异步任务
    for _ in 0..3 {
        let counter_clone = counter.clone();
        executor.spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
    }

    // 等待一小段时间让任务执行
    tokio::time::sleep(Duration::from_millis(10)).await;

    assert_eq!(executor.task_count(), 3);
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

/// 测试 ExecutorService trait 的 shutdown 方法
///
/// 测试数据：创建一个可关闭的执行器，提交任务后关闭
/// 预期结果：关闭前的任务正常执行，关闭后的任务被拒绝
#[test]
fn test_executor_service_shutdown() {
    let executor = ShutdownableExecutor::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // 提交任务
    let counter_clone = counter.clone();
    executor.execute(Box::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    }));

    assert_eq!(executor.task_count(), 1);
    assert!(!executor.is_shutdown());

    // 关闭执行器
    executor.shutdown();
    assert!(executor.is_shutdown());

    // 尝试提交新任务，应该被拒绝
    let counter_clone = counter.clone();
    executor.execute(Box::new(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    }));

    // 任务计数仍为1，新任务被拒绝
    assert_eq!(executor.task_count(), 1);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

/// 测试 ExecutorService trait 的 shutdown_now 方法
///
/// 测试数据：创建一个可关闭的执行器，使用 shutdown_now 立即关闭
/// 预期结果：执行器立即关闭并标记为已终止
#[test]
fn test_executor_service_shutdown_now() {
    let executor = ShutdownableExecutor::new();

    assert!(!executor.is_shutdown());
    assert!(!executor.is_terminated());

    // 立即关闭
    let pending_tasks = executor.shutdown_now();

    assert!(executor.is_shutdown());
    assert!(executor.is_terminated());
    assert_eq!(pending_tasks.len(), 0);
}

/// 测试 ExecutorService trait 的 await_termination 方法
///
/// 测试数据：创建一个可关闭的执行器，等待终止
/// 预期结果：等待后执行器标记为已终止
#[tokio::test]
async fn test_executor_service_await_termination() {
    let executor = ShutdownableExecutor::new();

    executor.shutdown();
    assert!(executor.is_shutdown());
    assert!(!executor.is_terminated());

    // 等待终止
    executor.await_termination().await;

    assert!(executor.is_terminated());
}

/// 测试实现 Runnable trait 的自定义任务
///
/// 测试数据：创建一个实现 Runnable trait 的任务类型
/// 预期结果：任务能正确执行
#[test]
fn test_runnable_trait() {
    struct CounterTask {
        counter: Arc<AtomicUsize>,
        increment: usize,
    }

    impl Runnable for CounterTask {
        fn run(&self) {
            self.counter.fetch_add(self.increment, Ordering::SeqCst);
        }
    }

    let counter = Arc::new(AtomicUsize::new(0));
    let task = CounterTask {
        counter: counter.clone(),
        increment: 5,
    };

    task.run();
    assert_eq!(counter.load(Ordering::SeqCst), 5);

    task.run();
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

/// 测试执行器的多线程安全性
///
/// 测试数据：从多个线程同时提交任务到执行器
/// 预期结果：所有任务都能正确执行，无数据竞争
#[test]
fn test_executor_thread_safety() {
    let executor = Arc::new(SimpleExecutor::new());
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // 启动10个线程，每个线程提交10个任务
    for _ in 0..10 {
        let executor_clone = executor.clone();
        let counter_clone = counter.clone();
        let handle = std::thread::spawn(move || {
            for _ in 0..10 {
                let counter_clone2 = counter_clone.clone();
                executor_clone.execute(Box::new(move || {
                    counter_clone2.fetch_add(1, Ordering::SeqCst);
                }));
            }
        });
        handles.push(handle);
    }

    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(executor.task_count(), 100);
    assert_eq!(counter.load(Ordering::SeqCst), 100);
}

/// 测试 Callable trait 的成功情况
///
/// 测试数据：创建一个返回成功结果的 Callable 任务
/// 预期结果：任务能正确执行并返回计算结果
#[test]
fn test_callable_success() {
    struct AddTask {
        x: i32,
        y: i32,
    }

    impl Callable<i32, String> for AddTask {
        fn call(&self) -> Result<i32, String> {
            Ok(self.x + self.y)
        }
    }

    let task = AddTask { x: 10, y: 20 };
    let result = task.call();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 30);
}

/// 测试 Callable trait 的错误情况
///
/// 测试数据：创建一个返回错误的 Callable 任务
/// 预期结果：任务能正确执行并返回错误信息
#[test]
fn test_callable_error() {
    struct DivideTask {
        x: i32,
        y: i32,
    }

    impl Callable<i32, String> for DivideTask {
        fn call(&self) -> Result<i32, String> {
            if self.y == 0 {
                Err(String::from("除数不能为零"))
            } else {
                Ok(self.x / self.y)
            }
        }
    }

    // 测试除数为零的情况
    let task = DivideTask { x: 10, y: 0 };
    let result = task.call();

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "除数不能为零");

    // 测试正常情况
    let task = DivideTask { x: 10, y: 2 };
    let result = task.call();

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
}

/// 测试 Callable trait 的复杂计算
///
/// 测试数据：创建一个执行复杂计算的 Callable 任务
/// 预期结果：任务能正确执行并返回计算结果
#[test]
fn test_callable_complex_computation() {
    struct FactorialTask {
        n: u32,
    }

    impl Callable<u64, String> for FactorialTask {
        fn call(&self) -> Result<u64, String> {
            if self.n > 20 {
                return Err(String::from("输入值过大，可能导致溢出"));
            }

            let mut result: u64 = 1;
            for i in 1..=self.n {
                result *= i as u64;
            }
            Ok(result)
        }
    }

    // 测试正常计算
    let task = FactorialTask { n: 5 };
    let result = task.call();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 120); // 5! = 120

    // 测试边界情况
    let task = FactorialTask { n: 0 };
    let result = task.call();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1); // 0! = 1

    // 测试错误情况
    let task = FactorialTask { n: 25 };
    let result = task.call();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "输入值过大，可能导致溢出");
}

/// 测试 Callable trait 的泛型支持
///
/// 测试数据：创建返回不同类型结果的 Callable 任务
/// 预期结果：支持不同的返回类型和错误类型
#[test]
fn test_callable_generic_types() {
    // 返回 String 类型的 Callable
    struct StringTask {
        name: String,
    }

    impl Callable<String, ()> for StringTask {
        fn call(&self) -> Result<String, ()> {
            Ok(format!("Hello, {}!", self.name))
        }
    }

    let task = StringTask {
        name: String::from("World"),
    };
    let result = task.call();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, World!");

    // 返回 Vec 类型的 Callable
    struct VecTask {
        size: usize,
    }

    impl Callable<Vec<usize>, String> for VecTask {
        fn call(&self) -> Result<Vec<usize>, String> {
            if self.size > 1000 {
                return Err(String::from("大小超出限制"));
            }
            Ok((0..self.size).collect())
        }
    }

    let task = VecTask { size: 5 };
    let result = task.call();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), vec![0, 1, 2, 3, 4]);
}

/// 测试 Executor 和 AsyncExecutor 的职责分离
///
/// 测试数据：同时使用同步和异步执行器
/// 预期结果：两个执行器各自独立工作，互不干扰
#[tokio::test]
async fn test_executor_separation() {
    let sync_executor = SimpleExecutor::new();
    let async_executor = SimpleAsyncExecutor::new();
    let sync_counter = Arc::new(AtomicUsize::new(0));
    let async_counter = Arc::new(AtomicUsize::new(0));

    // 使用同步执行器
    for _ in 0..5 {
        let counter = sync_counter.clone();
        sync_executor.execute(Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
    }

    // 使用异步执行器
    for _ in 0..5 {
        let counter = async_counter.clone();
        async_executor.spawn(async move {
            counter.fetch_add(1, Ordering::SeqCst);
        });
    }

    // 等待异步任务完成
    tokio::time::sleep(Duration::from_millis(10)).await;

    // 验证两个执行器独立工作
    assert_eq!(sync_executor.task_count(), 5);
    assert_eq!(async_executor.task_count(), 5);
    assert_eq!(sync_counter.load(Ordering::SeqCst), 5);
    assert_eq!(async_counter.load(Ordering::SeqCst), 5);
}

/// 测试 AsyncExecutor 的并发能力
///
/// 测试数据：创建多个并发的异步任务
/// 预期结果：异步任务能够高效并发执行
#[tokio::test]
async fn test_async_executor_concurrency() {
    let executor = SimpleAsyncExecutor::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // 生成100个异步任务
    for i in 0..100 {
        let counter_clone = counter.clone();
        executor.spawn(async move {
            // 模拟异步操作
            tokio::time::sleep(Duration::from_micros(i % 10)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
    }

    // 等待足够的时间让所有任务完成
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(executor.task_count(), 100);
    assert_eq!(counter.load(Ordering::SeqCst), 100);
}


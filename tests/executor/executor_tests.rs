/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for built-in executor implementations.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use qubit_concurrent::{
    AsyncExecutor, AsyncExecutorService, DirectExecutor, Executor, ExecutorService,
    ThreadPerTaskExecutor, ThreadPerTaskExecutorService, TokioExecutor, TokioExecutorService,
};

#[cfg(test)]
#[allow(clippy::module_inception)]
mod executor_tests {
    use super::*;

    /// Creates a current-thread Tokio runtime for driving async termination APIs in sync tests.
    fn create_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for executor tests")
    }

    #[test]
    fn test_direct_executor_execute_runs_inline() {
        let executor = DirectExecutor;
        let value = Arc::new(AtomicUsize::new(0));
        let value_for_task = Arc::clone(&value);

        executor.execute(Box::new(move || {
            value_for_task.fetch_add(1, Ordering::AcqRel);
        }));

        assert_eq!(value.load(Ordering::Acquire), 1);
    }

    #[test]
    fn test_thread_per_task_executor_execute_runs_task() {
        let executor = ThreadPerTaskExecutor;
        let (sender, receiver) = mpsc::channel();

        executor.execute(Box::new(move || {
            sender
                .send(42usize)
                .expect("Failed to send task result to test thread");
        }));

        let received = receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("Task did not complete within timeout");
        assert_eq!(received, 42);
    }

    #[test]
    fn test_thread_per_task_executor_service_shutdown_blocks_new_tasks() {
        let executor = ThreadPerTaskExecutorService::new();
        let (sender, receiver) = mpsc::channel();
        executor.shutdown();

        executor.execute(Box::new(move || {
            let _ = sender.send(1usize);
        }));

        assert!(
            receiver.recv_timeout(Duration::from_millis(200)).is_err(),
            "Task should not run after shutdown"
        );
        assert!(executor.is_shutdown());
    }

    #[test]
    fn test_thread_per_task_executor_service_await_termination_waits_for_tasks() {
        let executor = ThreadPerTaskExecutorService::new();
        let completed = Arc::new(AtomicBool::new(false));
        let completed_for_task = Arc::clone(&completed);

        executor.execute(Box::new(move || {
            std::thread::sleep(Duration::from_millis(80));
            completed_for_task.store(true, Ordering::Release);
        }));

        executor.shutdown();
        create_runtime().block_on(executor.await_termination());

        assert!(executor.is_terminated());
        assert!(completed.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_tokio_executor_spawn_runs_task() {
        let executor = TokioExecutor;
        let value = Arc::new(AtomicUsize::new(0));
        let value_for_task = Arc::clone(&value);

        executor.spawn(async move {
            value_for_task.fetch_add(1, Ordering::AcqRel);
        });

        tokio::task::yield_now().await;
        assert_eq!(value.load(Ordering::Acquire), 1);
    }

    #[tokio::test]
    async fn test_tokio_executor_service_shutdown_blocks_new_tasks() {
        let executor = TokioExecutorService::new();
        let ran = Arc::new(AtomicBool::new(false));
        let ran_for_task = Arc::clone(&ran);
        executor.shutdown();

        executor.spawn(async move {
            ran_for_task.store(true, Ordering::Release);
        });

        tokio::task::yield_now().await;
        assert!(!ran.load(Ordering::Acquire));
        assert!(executor.is_shutdown());
        assert!(executor.is_terminated());
    }

    #[tokio::test]
    async fn test_tokio_executor_service_shutdown_now_aborts_running_tasks() {
        let executor = TokioExecutorService::new();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_for_task = Arc::clone(&finished);

        executor.spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            finished_for_task.store(true, Ordering::Release);
        });

        tokio::task::yield_now().await;
        executor.shutdown_now();
        executor.await_termination().await;

        assert!(executor.is_shutdown());
        assert!(executor.is_terminated());
        assert!(!finished.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn test_tokio_executor_service_await_termination_waits_for_tasks() {
        let executor = TokioExecutorService::new();
        let completed = Arc::new(AtomicUsize::new(0));

        for _ in 0..3 {
            let completed_for_task = Arc::clone(&completed);
            executor.spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                completed_for_task.fetch_add(1, Ordering::AcqRel);
            });
        }

        executor.shutdown();
        executor.await_termination().await;

        assert_eq!(completed.load(Ordering::Acquire), 3);
        assert!(executor.is_shutdown());
        assert!(executor.is_terminated());
    }
}

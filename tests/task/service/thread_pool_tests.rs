/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`ThreadPool`](qubit_concurrent::task::service::ThreadPool).

use std::{
    io,
    sync::mpsc,
    time::Duration,
};

use qubit_concurrent::task::{
    TaskExecutionError,
    service::{
        ExecutorService,
        RejectedExecution,
        ThreadPool,
        ThreadPoolBuildError,
    },
};

/// Creates a current-thread Tokio runtime for driving async termination APIs in sync tests.
fn create_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime for thread pool tests")
}

/// Creates a single-worker pool for deterministic queue tests.
fn create_single_worker_pool() -> ThreadPool {
    ThreadPool::new(1).expect("thread pool should be created")
}

/// Waits until a blocking task reports that it has started.
fn wait_started(receiver: mpsc::Receiver<()>) {
    receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("task should start within timeout");
}

#[test]
fn test_thread_pool_submit_acceptance_is_not_task_success() {
    let pool = ThreadPool::new(2).expect("thread pool should be created");

    let handle = pool
        .submit(|| Err::<(), _>(io::Error::other("task failed")))
        .expect("thread pool should accept runnable");

    let err = handle
        .get()
        .expect_err("accepted runnable should report task failure through handle");
    assert!(matches!(err, TaskExecutionError::Failed(_)));
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_submit_callable_returns_value() {
    let pool = ThreadPool::new(2).expect("thread pool should be created");

    let handle = pool
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("thread pool should accept callable");

    assert_eq!(
        handle.get().expect("callable should complete successfully"),
        42,
    );
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[tokio::test]
async fn test_thread_pool_handle_can_be_awaited() {
    let pool = ThreadPool::new(2).expect("thread pool should be created");

    let handle = pool
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("thread pool should accept callable");

    assert_eq!(handle.await.expect("handle should await result"), 42);
    pool.shutdown();
    pool.await_termination().await;
}

#[test]
fn test_thread_pool_shutdown_rejects_new_tasks() {
    let pool = ThreadPool::new(1).expect("thread pool should be created");

    pool.shutdown();
    let result = pool.submit(|| Ok::<(), io::Error>(()));

    assert!(matches!(result, Err(RejectedExecution::Shutdown)));
    create_runtime().block_on(pool.await_termination());
    assert!(pool.is_shutdown());
    assert!(pool.is_terminated());
}

#[test]
fn test_thread_pool_bounded_queue_rejects_when_saturated() {
    let pool = ThreadPool::builder()
        .worker_count(1)
        .queue_capacity(1)
        .build()
        .expect("thread pool should be created");
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            started_tx
                .send(())
                .expect("test should receive task start signal");
            release_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should be accepted");
    wait_started(started_rx);

    let second = pool
        .submit(|| Ok::<(), io::Error>(()))
        .expect("second task should fill the queue");
    let third = pool.submit(|| Ok::<(), io::Error>(()));

    assert!(matches!(third, Err(RejectedExecution::Saturated)));
    release_tx
        .send(())
        .expect("blocking task should receive release signal");
    first
        .get()
        .expect("first task should complete successfully");
    second
        .get()
        .expect("queued task should complete successfully");
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_shutdown_drains_queued_tasks() {
    let pool = create_single_worker_pool();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            started_tx
                .send(())
                .expect("test should receive task start signal");
            release_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should be accepted");
    wait_started(started_rx);
    let second = pool
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("queued task should be accepted");

    pool.shutdown();
    let rejected = pool.submit(|| Ok::<(), io::Error>(()));
    release_tx
        .send(())
        .expect("blocking task should receive release signal");
    first
        .get()
        .expect("first task should complete successfully");

    assert!(matches!(rejected, Err(RejectedExecution::Shutdown)));
    assert_eq!(second.get().expect("queued task should still run"), 42);
    create_runtime().block_on(pool.await_termination());
    assert!(pool.is_terminated());
}

#[test]
fn test_thread_pool_shutdown_now_cancels_queued_tasks() {
    let pool = create_single_worker_pool();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            started_tx
                .send(())
                .expect("test should receive task start signal");
            release_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should be accepted");
    wait_started(started_rx);
    let queued = pool
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("queued task should be accepted");

    let report = pool.shutdown_now();

    assert_eq!(report.queued, 1);
    assert_eq!(report.running, 1);
    assert_eq!(report.cancelled, 1);
    assert!(matches!(queued.get(), Err(TaskExecutionError::Cancelled),));
    release_tx
        .send(())
        .expect("blocking task should receive release signal");
    first.get().expect("running task should complete normally");
    create_runtime().block_on(pool.await_termination());
    assert!(pool.is_terminated());
}

#[test]
fn test_thread_pool_cancel_before_start_reports_cancelled() {
    let pool = create_single_worker_pool();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            started_tx
                .send(())
                .expect("test should receive task start signal");
            release_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should be accepted");
    wait_started(started_rx);
    let queued = pool
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("queued task should be accepted");

    assert!(queued.cancel());
    assert!(queued.is_done());
    assert!(matches!(queued.get(), Err(TaskExecutionError::Cancelled),));
    pool.shutdown();
    release_tx
        .send(())
        .expect("blocking task should receive release signal");
    first.get().expect("running task should complete normally");
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_builder_rejects_invalid_configuration() {
    assert!(matches!(
        ThreadPool::builder().worker_count(0).build(),
        Err(ThreadPoolBuildError::ZeroWorkers),
    ));
    assert!(matches!(
        ThreadPool::builder().queue_capacity(0).build(),
        Err(ThreadPoolBuildError::ZeroQueueCapacity),
    ));
    assert!(matches!(
        ThreadPool::builder().stack_size(0).build(),
        Err(ThreadPoolBuildError::ZeroStackSize),
    ));
}

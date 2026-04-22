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
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
        mpsc,
    },
    time::{
        Duration,
        Instant,
    },
};

use qubit_concurrent::task::{
    TaskExecutionError,
    service::{
        ExecutorService,
        PoolJob,
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

/// Waits until a condition becomes true or fails the test.
fn wait_until<F>(mut condition: F)
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(condition(), "condition should become true within timeout");
}

fn ok_unit_task() -> Result<(), io::Error> {
    Ok(())
}

fn ok_usize_task() -> Result<usize, io::Error> {
    Ok(42)
}

#[test]
fn test_thread_pool_submit_acceptance_is_not_task_success() {
    let pool = ThreadPool::new(2).expect("thread pool should be created");

    pool.submit(ok_unit_task as fn() -> Result<(), io::Error>)
        .expect("thread pool should accept shared runnable")
        .get()
        .expect("shared runnable should complete successfully");

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
        .submit_callable(ok_usize_task as fn() -> Result<usize, io::Error>)
        .expect("thread pool should accept callable");

    assert_eq!(
        handle.get().expect("callable should complete successfully"),
        42,
    );
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_submit_job_runs_type_erased_job() {
    let pool = ThreadPool::new(1).expect("thread pool should be created");
    let ran = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));

    pool.submit_job(PoolJob::new(
        {
            let ran = Arc::clone(&ran);
            Box::new(move || {
                ran.store(true, Ordering::Release);
            })
        },
        {
            let cancelled = Arc::clone(&cancelled);
            Box::new(move || {
                cancelled.store(true, Ordering::Release);
            })
        },
    ))
    .expect("type-erased pool job should be accepted");

    pool.shutdown();
    create_runtime().block_on(pool.await_termination());

    assert!(ran.load(Ordering::Acquire));
    assert!(!cancelled.load(Ordering::Acquire));
}

#[tokio::test]
async fn test_thread_pool_handle_can_be_awaited() {
    let pool = ThreadPool::new(2).expect("thread pool should be created");

    let handle = pool
        .submit_callable(ok_usize_task as fn() -> Result<usize, io::Error>)
        .expect("thread pool should accept callable");

    assert_eq!(handle.await.expect("handle should await result"), 42);
    pool.shutdown();
    pool.await_termination().await;
}

#[test]
fn test_thread_pool_shutdown_rejects_new_tasks() {
    let pool = ThreadPool::new(1).expect("thread pool should be created");

    pool.shutdown();
    let result = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);

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
        .submit(ok_unit_task as fn() -> Result<(), io::Error>)
        .expect("second task should fill the queue");
    let third = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);

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
        .submit_callable(ok_usize_task as fn() -> Result<usize, io::Error>)
        .expect("queued task should be accepted");

    pool.shutdown();
    let rejected = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);
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
        .submit_callable(ok_usize_task as fn() -> Result<usize, io::Error>)
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
        .submit_callable(ok_usize_task as fn() -> Result<usize, io::Error>)
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
fn test_thread_pool_grows_above_core_when_queue_is_full() {
    let pool = ThreadPool::builder()
        .core_pool_size(1)
        .maximum_pool_size(2)
        .queue_capacity(1)
        .keep_alive(Duration::from_millis(50))
        .build()
        .expect("thread pool should be created");
    let (first_started_tx, first_started_rx) = mpsc::channel();
    let (third_started_tx, third_started_rx) = mpsc::channel();
    let (release_first_tx, release_first_rx) = mpsc::channel();
    let (release_third_tx, release_third_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            first_started_tx
                .send(())
                .expect("test should receive first start signal");
            release_first_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should start on the core worker");
    wait_started(first_started_rx);

    let second = pool
        .submit(ok_unit_task as fn() -> Result<(), io::Error>)
        .expect("second task should be queued");
    let third = pool
        .submit(move || {
            third_started_tx
                .send(())
                .expect("test should receive third start signal");
            release_third_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("third task should create a non-core worker");
    wait_started(third_started_rx);

    let fourth = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);

    assert!(matches!(fourth, Err(RejectedExecution::Saturated)));
    assert_eq!(pool.stats().live_workers, 2);
    release_third_tx
        .send(())
        .expect("third task should receive release signal");
    third
        .get()
        .expect("third task should complete successfully");
    wait_until(|| pool.stats().live_workers == 1);
    release_first_tx
        .send(())
        .expect("first task should receive release signal");
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
fn test_thread_pool_excess_workers_retire_after_maximum_size_decreases() {
    let pool = ThreadPool::builder()
        .core_pool_size(1)
        .maximum_pool_size(2)
        .queue_capacity(1)
        .keep_alive(Duration::from_secs(5))
        .build()
        .expect("thread pool should be created");
    let (first_started_tx, first_started_rx) = mpsc::channel();
    let (third_started_tx, third_started_rx) = mpsc::channel();
    let (release_first_tx, release_first_rx) = mpsc::channel();
    let (release_third_tx, release_third_rx) = mpsc::channel();

    let first = pool
        .submit(move || {
            first_started_tx
                .send(())
                .expect("test should receive first start signal");
            release_first_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("first task should start on the core worker");
    wait_started(first_started_rx);

    let second = pool
        .submit(ok_unit_task as fn() -> Result<(), io::Error>)
        .expect("second task should be queued");
    let third = pool
        .submit(move || {
            third_started_tx
                .send(())
                .expect("test should receive third start signal");
            release_third_rx
                .recv()
                .map_err(|err| io::Error::other(err.to_string()))?;
            Ok::<(), io::Error>(())
        })
        .expect("third task should create an extra worker");
    wait_started(third_started_rx);

    assert_eq!(pool.worker_count(), 2);
    pool.set_maximum_pool_size(1)
        .expect("maximum size should shrink to current core size");
    release_third_tx
        .send(())
        .expect("third task should receive release signal");

    third
        .get()
        .expect("third task should complete successfully");
    wait_until(|| pool.worker_count() == 1);
    release_first_tx
        .send(())
        .expect("first task should receive release signal");
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
fn test_thread_pool_prestarts_core_workers() {
    let pool = ThreadPool::builder()
        .worker_count(2)
        .prestart_core_threads()
        .build()
        .expect("thread pool should be created");

    assert_eq!(pool.worker_count(), 2);
    assert_eq!(pool.stats().live_workers, 2);
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_prestart_core_thread_reports_state() {
    let pool = ThreadPool::builder()
        .worker_count(1)
        .build()
        .expect("thread pool should be created");

    assert!(pool.prestart_core_thread().expect("worker should start"));
    assert!(
        !pool
            .prestart_core_thread()
            .expect("no worker should be needed")
    );
    pool.shutdown();
    assert!(matches!(
        pool.prestart_core_thread(),
        Err(RejectedExecution::Shutdown),
    ));
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_prestart_all_core_threads_reports_state() {
    let pool = ThreadPool::builder()
        .worker_count(2)
        .build()
        .expect("thread pool should be created");

    assert_eq!(
        pool.prestart_all_core_threads()
            .expect("all core workers should start"),
        2,
    );
    assert_eq!(
        pool.prestart_all_core_threads()
            .expect("all core workers already started"),
        0,
    );
    pool.shutdown();
    assert!(matches!(
        pool.prestart_all_core_threads(),
        Err(RejectedExecution::Shutdown),
    ));
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_core_threads_can_timeout() {
    let pool = ThreadPool::builder()
        .worker_count(1)
        .keep_alive(Duration::from_millis(80))
        .allow_core_thread_timeout(true)
        .prestart_core_threads()
        .build()
        .expect("thread pool should be created");

    assert_eq!(pool.worker_count(), 1);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(pool.worker_count(), 1);
    wait_until(|| pool.worker_count() == 0);
    assert!(!pool.is_terminated());
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_accessors_and_dynamic_settings() {
    let pool = ThreadPool::builder()
        .core_pool_size(0)
        .maximum_pool_size(2)
        .queue_capacity(1)
        .build()
        .expect("thread pool should be created");

    assert_eq!(pool.queued_count(), 0);
    assert_eq!(pool.running_count(), 0);
    assert_eq!(pool.worker_count(), 0);
    assert_eq!(pool.core_pool_size(), 0);
    assert_eq!(pool.maximum_pool_size(), 2);
    assert!(pool.set_core_pool_size(1).is_ok());
    assert!(pool.set_maximum_pool_size(3).is_ok());
    assert!(pool.set_keep_alive(Duration::from_millis(25)).is_ok());
    pool.allow_core_thread_timeout(true);
    assert_eq!(pool.core_pool_size(), 1);
    assert_eq!(pool.maximum_pool_size(), 3);
    assert!(matches!(
        pool.set_core_pool_size(4),
        Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum { .. }),
    ));
    assert!(matches!(
        pool.set_maximum_pool_size(0),
        Err(ThreadPoolBuildError::ZeroMaximumPoolSize),
    ));
    assert!(pool.set_core_pool_size(2).is_ok());
    assert!(matches!(
        pool.set_maximum_pool_size(1),
        Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum { .. }),
    ));
    assert!(matches!(
        pool.set_keep_alive(Duration::ZERO),
        Err(ThreadPoolBuildError::ZeroKeepAlive),
    ));
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_builder_sets_unbounded_queue_and_thread_options() {
    let pool = ThreadPool::builder()
        .core_pool_size(0)
        .maximum_pool_size(1)
        .queue_capacity(1)
        .unbounded_queue()
        .thread_name_prefix("custom-pool")
        .stack_size(2 * 1024 * 1024)
        .keep_alive(Duration::from_millis(20))
        .allow_core_thread_timeout(true)
        .build()
        .expect("thread pool should be created");

    let name = pool
        .submit_callable(|| {
            Ok::<String, io::Error>(
                std::thread::current()
                    .name()
                    .expect("worker thread should have a name")
                    .to_owned(),
            )
        })
        .expect("task should be accepted")
        .get()
        .expect("task should complete");

    assert!(name.starts_with("custom-pool-"));
    wait_until(|| pool.worker_count() == 0);
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_reports_worker_spawn_failure() {
    let pool = ThreadPool::builder()
        .worker_count(1)
        .stack_size(usize::MAX)
        .build()
        .expect("thread pool should be created lazily");

    let result = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);

    assert!(matches!(
        result,
        Err(RejectedExecution::WorkerSpawnFailed { .. }),
    ));
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_cancels_queued_job_when_initial_worker_spawn_fails() {
    let pool = ThreadPool::builder()
        .core_pool_size(0)
        .maximum_pool_size(1)
        .queue_capacity(1)
        .stack_size(usize::MAX)
        .build()
        .expect("thread pool should be created lazily");

    let result = pool.submit(ok_unit_task as fn() -> Result<(), io::Error>);

    assert!(matches!(
        result,
        Err(RejectedExecution::WorkerSpawnFailed { .. }),
    ));
    assert_eq!(pool.queued_count(), 0);
    pool.shutdown();
    create_runtime().block_on(pool.await_termination());
}

#[test]
fn test_thread_pool_prestart_reports_build_spawn_failure() {
    let result = ThreadPool::builder()
        .worker_count(1)
        .stack_size(usize::MAX)
        .prestart_core_threads()
        .build();

    assert!(matches!(
        result,
        Err(ThreadPoolBuildError::SpawnWorker { .. })
    ));
}

#[test]
fn test_thread_pool_build_error_from_rejected_execution_shutdown() {
    let error: ThreadPoolBuildError = RejectedExecution::Shutdown.into();

    let ThreadPoolBuildError::SpawnWorker { source, .. } = error else {
        panic!("expected spawn worker build error");
    };
    assert_eq!(source.to_string(), "thread pool shut down during prestart");
}

#[test]
fn test_thread_pool_build_error_from_rejected_execution_saturated() {
    let error: ThreadPoolBuildError = RejectedExecution::Saturated.into();

    let ThreadPoolBuildError::SpawnWorker { source, .. } = error else {
        panic!("expected spawn worker build error");
    };
    assert_eq!(source.to_string(), "thread pool saturated during prestart");
}

#[test]
fn test_rejected_execution_compares_by_variant() {
    let left = RejectedExecution::WorkerSpawnFailed {
        source: Arc::new(io::Error::other("left")),
    };
    let right = RejectedExecution::WorkerSpawnFailed {
        source: Arc::new(io::Error::other("right")),
    };

    assert_eq!(left, right);
    assert_ne!(RejectedExecution::Shutdown, RejectedExecution::Saturated);
    assert_eq!(
        RejectedExecution::Saturated.to_string(),
        "task rejected because the executor service is saturated",
    );
}

#[test]
fn test_thread_pool_shutdown_now_after_shutdown_is_idempotent() {
    let pool = ThreadPool::new(1).expect("thread pool should be created");

    pool.shutdown();
    let report = pool.shutdown_now();

    assert_eq!(report.queued, 0);
    assert_eq!(report.running, 0);
    assert_eq!(report.cancelled, 0);
    create_runtime().block_on(pool.await_termination());
    assert!(pool.is_terminated());
}

#[test]
fn test_thread_pool_builder_rejects_invalid_configuration() {
    assert!(matches!(
        ThreadPool::builder().worker_count(0).build(),
        Err(ThreadPoolBuildError::ZeroMaximumPoolSize),
    ));
    assert!(matches!(
        ThreadPool::builder().queue_capacity(0).build(),
        Err(ThreadPoolBuildError::ZeroQueueCapacity),
    ));
    assert!(matches!(
        ThreadPool::builder().stack_size(0).build(),
        Err(ThreadPoolBuildError::ZeroStackSize),
    ));
    assert!(matches!(
        ThreadPool::builder()
            .core_pool_size(2)
            .maximum_pool_size(1)
            .build(),
        Err(ThreadPoolBuildError::CorePoolSizeExceedsMaximum { .. }),
    ));
    assert!(matches!(
        ThreadPool::builder().keep_alive(Duration::ZERO).build(),
        Err(ThreadPoolBuildError::ZeroKeepAlive),
    ));
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`ExecutionServices`](qubit_concurrent::task::service::ExecutionServices).

use std::{io, time::Duration};

use qubit_concurrent::task::{
    TaskExecutionError,
    service::{ExecutionServices, ExecutionServicesBuildError, RejectedExecution},
};

fn create_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime for execution services tests")
}

#[test]
fn test_execution_services_builder_rejects_invalid_blocking_domain() {
    let error = match ExecutionServices::builder()
        .blocking_maximum_pool_size(0)
        .build()
    {
        Ok(_) => panic!("builder should reject invalid blocking domain"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        ExecutionServicesBuildError::Blocking { .. }
    ));
}

#[test]
fn test_execution_services_builder_rejects_invalid_cpu_domain() {
    let error = match ExecutionServices::builder().cpu_threads(0).build() {
        Ok(_) => panic!("builder should reject invalid cpu domain"),
        Err(error) => error,
    };

    assert!(matches!(error, ExecutionServicesBuildError::Cpu { .. }));
}

#[test]
fn test_execution_services_submit_blocking_and_cpu_tasks() {
    let services = ExecutionServices::builder()
        .blocking_pool_size(1)
        .cpu_threads(1)
        .build()
        .expect("execution services should be created");

    let blocking = services
        .submit_blocking_callable(|| Ok::<usize, io::Error>(40 + 2))
        .expect("blocking domain should accept callable");
    let cpu = services
        .submit_cpu_callable(|| Ok::<usize, io::Error>(6 * 7))
        .expect("cpu domain should accept callable");

    assert_eq!(
        blocking
            .get()
            .expect("blocking task should complete successfully"),
        42,
    );
    assert_eq!(
        cpu.get().expect("cpu task should complete successfully"),
        42
    );
    services.shutdown();
    create_runtime().block_on(services.await_termination());
    assert!(services.is_shutdown());
    assert!(services.is_terminated());
}

#[tokio::test]
async fn test_execution_services_submit_tokio_blocking_and_io_tasks() {
    let services = ExecutionServices::new().expect("execution services should be created");

    let blocking = services
        .submit_tokio_blocking_callable(|| Ok::<usize, io::Error>(40 + 2))
        .expect("tokio blocking domain should accept callable");
    let io = services
        .spawn_io(async { Ok::<usize, io::Error>(42) })
        .expect("io domain should accept future");

    assert_eq!(
        blocking
            .await
            .expect("tokio blocking task should complete successfully"),
        42,
    );
    assert_eq!(io.await.expect("io task should complete successfully"), 42);
    services.shutdown();
    services.await_termination().await;
}

#[tokio::test]
async fn test_execution_services_shutdown_now_aggregates_reports() {
    let services = ExecutionServices::builder()
        .blocking_pool_size(1)
        .cpu_threads(1)
        .build()
        .expect("execution services should be created");

    let blocking = services
        .submit_tokio_blocking(|| {
            std::thread::sleep(Duration::from_secs(1));
            Ok::<(), io::Error>(())
        })
        .expect("tokio blocking domain should accept task");
    let io = services
        .spawn_io(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<(), io::Error>(())
        })
        .expect("io domain should accept task");

    tokio::task::yield_now().await;
    let report = services.shutdown_now();
    services.await_termination().await;

    assert!(report.total_running() >= 2);
    assert!(report.total_cancelled() >= 2);
    assert!(services.is_shutdown());
    assert!(services.is_terminated());
    assert!(matches!(blocking.await, Err(TaskExecutionError::Cancelled)));
    assert!(matches!(io.await, Err(TaskExecutionError::Cancelled)));
}

#[tokio::test]
async fn test_execution_services_shutdown_rejects_new_tasks() {
    let services = ExecutionServices::new().expect("execution services should be created");

    services.shutdown();
    let result = services.spawn_io(async { Ok::<(), io::Error>(()) });

    assert!(matches!(result, Err(RejectedExecution::Shutdown)));
    services.await_termination().await;
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`TokioExecutorService`](qubit_concurrent::task::service::TokioExecutorService).

use std::{
    io,
    time::Duration,
};

use qubit_concurrent::task::{
    TaskExecutionError,
    service::{
        ExecutorService,
        RejectedExecution,
        TokioExecutorService,
    },
};

#[tokio::test]
async fn test_tokio_executor_service_submit_acceptance_is_not_task_success() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit(|| Err::<(), _>(io::Error::other("task failed")))
        .expect("service should accept the runnable");

    let err = handle
        .await
        .expect_err("accepted runnable should report task failure through handle");
    assert!(matches!(err, TaskExecutionError::Failed(_)));
}

#[tokio::test]
async fn test_tokio_executor_service_submit_callable_returns_value() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit_callable(|| Ok::<usize, io::Error>(42))
        .expect("service should accept the callable");

    assert_eq!(
        handle.await.expect("callable should complete successfully"),
        42,
    );
}

#[tokio::test]
async fn test_tokio_executor_service_shutdown_rejects_new_tasks() {
    let service = TokioExecutorService::new();
    service.shutdown();

    let result = service.submit(|| Ok::<(), io::Error>(()));

    assert!(matches!(result, Err(RejectedExecution::Shutdown)));
    assert!(service.is_shutdown());
    assert!(service.is_terminated());
}

#[tokio::test]
async fn test_tokio_executor_service_await_termination_waits_for_tasks() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit(|| {
            std::thread::sleep(Duration::from_millis(50));
            Ok::<(), io::Error>(())
        })
        .expect("service should accept task");

    service.shutdown();
    service.await_termination().await;

    handle.await.expect("task should complete successfully");
    assert!(service.is_shutdown());
    assert!(service.is_terminated());
}

#[tokio::test]
async fn test_tokio_executor_service_shutdown_now_aborts_running_task_handle() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit(|| {
            std::thread::sleep(Duration::from_secs(1));
            Ok::<(), io::Error>(())
        })
        .expect("service should accept task");

    tokio::task::yield_now().await;
    let report = service.shutdown_now();
    service.await_termination().await;

    assert!(report.cancelled >= 1);
    assert!(service.is_shutdown());
    assert!(service.is_terminated());
    assert!(matches!(handle.await, Err(TaskExecutionError::Cancelled)));
}

#[tokio::test]
async fn test_tokio_task_handle_cancel_requests_abort() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit(|| {
            std::thread::sleep(Duration::from_secs(1));
            Ok::<(), io::Error>(())
        })
        .expect("service should accept task");

    assert!(handle.cancel());
    tokio::task::yield_now().await;
    assert!(handle.is_done());
    assert!(matches!(handle.await, Err(TaskExecutionError::Cancelled)));
    service.shutdown();
    service.await_termination().await;
}

#[tokio::test]
async fn test_tokio_task_handle_reports_panicked_task() {
    let service = TokioExecutorService::new();

    let handle = service
        .submit(|| -> Result<(), io::Error> { panic!("tokio service panic") })
        .expect("service should accept panicking task");

    assert!(matches!(handle.await, Err(TaskExecutionError::Panicked)));
    service.shutdown();
    service.await_termination().await;
}

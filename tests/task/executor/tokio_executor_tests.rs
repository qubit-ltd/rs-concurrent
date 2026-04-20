/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`TokioExecutor`](qubit_concurrent::task::executor::TokioExecutor).

use std::io;
use std::time::Duration;

use qubit_concurrent::task::executor::{
    Executor,
    TokioExecutor,
};

#[tokio::test]
async fn test_tokio_executor_execute_returns_future_result() {
    let executor = TokioExecutor;

    executor
        .execute(|| Ok::<(), io::Error>(()))
        .await
        .expect("tokio executor should run runnable successfully");
}

#[tokio::test]
async fn test_tokio_executor_call_returns_future_value() {
    let executor = TokioExecutor;

    let value = executor
        .call(|| Ok::<usize, io::Error>(42))
        .await
        .expect("tokio executor should return callable value");

    assert_eq!(value, 42);
}

#[tokio::test]
async fn test_tokio_execution_is_finished_reports_completion() {
    let executor = TokioExecutor;

    let mut execution = executor.call(|| {
        std::thread::sleep(Duration::from_millis(25));
        Ok::<usize, io::Error>(42)
    });

    assert!(!execution.is_finished());
    assert_eq!(
        (&mut execution)
            .await
            .expect("tokio execution should complete"),
        42,
    );
}

#[tokio::test]
#[should_panic(expected = "tokio executor panic")]
async fn test_tokio_execution_resumes_task_panic() {
    let executor = TokioExecutor;

    executor
        .call(|| -> Result<(), io::Error> { panic!("tokio executor panic") })
        .await
        .expect("panic should be resumed before this result is observed");
}

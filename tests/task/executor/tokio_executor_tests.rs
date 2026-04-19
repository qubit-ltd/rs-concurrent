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

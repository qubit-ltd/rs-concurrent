/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`ThreadPerTaskExecutor`](qubit_concurrent::task::executor::ThreadPerTaskExecutor).

use std::io;

use qubit_concurrent::task::executor::{
    Executor,
    ThreadPerTaskExecutor,
};

#[test]
fn test_thread_per_task_executor_execute_runs_task() {
    let executor = ThreadPerTaskExecutor;

    let handle = executor.execute(|| Ok::<(), io::Error>(()));

    handle
        .get()
        .expect("thread-per-task executor should run task successfully");
}

#[test]
fn test_thread_per_task_executor_call_returns_value() {
    let executor = ThreadPerTaskExecutor;

    let handle = executor.call(|| Ok::<usize, io::Error>(42));

    assert_eq!(
        handle
            .get()
            .expect("thread-per-task executor should return callable value"),
        42,
    );
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`TaskHandle`](qubit_concurrent::task::TaskHandle).

use std::{
    io,
    sync::mpsc,
    thread,
    time::Duration,
};

use qubit_concurrent::task::executor::{
    Executor,
    ThreadPerTaskExecutor,
};

#[tokio::test]
async fn test_task_handle_await_returns_value() {
    let executor = ThreadPerTaskExecutor;

    let handle = executor.call(|| Ok::<usize, io::Error>(42));

    assert_eq!(
        handle.await.expect("task handle should await task result"),
        42,
    );
}

#[test]
fn test_task_handle_is_done_reports_completion() {
    let executor = ThreadPerTaskExecutor;

    let handle = executor.call(|| Ok::<usize, io::Error>(42));
    for _ in 0..100 {
        if handle.is_done() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(handle.is_done());
    assert_eq!(handle.get().expect("task should complete successfully"), 42);
}

#[test]
fn test_task_handle_cancel_after_start_returns_false() {
    let executor = ThreadPerTaskExecutor;
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();

    let handle = executor.call(move || {
        started_tx
            .send(())
            .expect("test should receive start signal");
        release_rx
            .recv()
            .map_err(|err| io::Error::other(err.to_string()))?;
        Ok::<usize, io::Error>(42)
    });
    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("task should start within timeout");

    assert!(!handle.cancel());
    release_tx
        .send(())
        .expect("task should receive release signal");
    assert_eq!(handle.get().expect("task should complete"), 42);
}

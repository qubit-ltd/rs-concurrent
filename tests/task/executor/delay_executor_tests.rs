/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Tests for [`DelayExecutor`](qubit_concurrent::task::executor::DelayExecutor).

use std::{
    io,
    sync::mpsc,
    time::{
        Duration,
        Instant,
    },
};

use qubit_concurrent::task::executor::{
    DelayExecutor,
    Executor,
};

#[test]
fn test_delay_executor_delays_task_start() {
    let executor = DelayExecutor::new(Duration::from_millis(80));
    let (started_tx, started_rx) = mpsc::channel();
    let start = Instant::now();

    assert_eq!(executor.delay(), Duration::from_millis(80));
    let handle = executor.execute(move || {
        started_tx
            .send(Instant::now())
            .expect("test should receive start time");
        Ok::<(), io::Error>(())
    });

    assert!(
        started_rx.recv_timeout(Duration::from_millis(30)).is_err(),
        "task should not start before the configured delay",
    );
    let started_at = started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("task should start after delay");
    handle.get().expect("delayed task should complete");
    assert!(started_at.duration_since(start) >= Duration::from_millis(70));
}

#[test]
fn test_delay_executor_returns_callable_value() {
    let executor = DelayExecutor::new(Duration::ZERO);

    let handle = executor.call(|| Ok::<usize, io::Error>(42));

    assert_eq!(handle.get().expect("callable should complete"), 42);
}

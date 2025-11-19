/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Lock Executor Demo
//!
//! Demonstrates the usage of the double-checked lock executor.
//!
//! # Author
//!
//! Haixing Hu

use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};

use prism3_concurrent::{
    lock::{
        ArcMutex,
        Lock,
    },
    DoubleCheckedLock,
};

#[derive(Debug, thiserror::Error)]
enum ServiceError {
    #[error("Service is not running")]
    NotRunning,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create shared state
    let running = Arc::new(AtomicBool::new(false));
    let data = ArcMutex::new(42);

    println!(
        "Initial state: running = {}",
        running.load(Ordering::Acquire)
    );
    println!("Initial data: {}", data.read(|d| *d));

    // Try to execute when service is not running (should fail)
    let result = DoubleCheckedLock::on(&data)
        .when({
            let running = running.clone();
            move || running.load(Ordering::Acquire)
        })
        .call_mut(|value: &mut i32| {
            *value += 1;
            Ok::<_, ServiceError>(*value)
        })
        .get_result();

    if result.success {
        println!("Unexpected success: {}", result.value.unwrap());
    } else {
        println!("Expected failure: Condition not met.");
    }

    // Start the service
    running.store(true, Ordering::Release);
    println!(
        "Service started: running = {}",
        running.load(Ordering::Acquire)
    );

    // Now execute should succeed
    let result = DoubleCheckedLock::on(&data)
        .when({
            let running = running.clone();
            move || running.load(Ordering::Acquire)
        })
        .call_mut(|value: &mut i32| {
            *value += 1;
            Ok::<_, ServiceError>(*value)
        })
        .get_result();

    if result.success {
        println!("Success: new value = {}", result.value.unwrap());
    } else {
        println!("Unexpected failure: {:?}", result.error);
    }

    // Verify the data was updated
    println!("Final data: {}", data.read(|d| *d));

    // Stop the service
    running.store(false, Ordering::Release);
    println!(
        "Service stopped: running = {}",
        running.load(Ordering::Acquire)
    );

    // Try to execute when service is stopped (should fail)
    let result = DoubleCheckedLock::on(&data)
        .when({
            let running = running.clone();
            move || running.load(Ordering::Acquire)
        })
        .call_mut(|value: &mut i32| {
            *value += 1;
            Ok::<_, ServiceError>(*value)
        })
        .get_result();

    if result.success {
        println!("Unexpected success: {}", result.value.unwrap());
    } else {
        println!("Expected failure: Condition not met.");
    }

    Ok(())
}

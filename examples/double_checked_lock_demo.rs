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

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use prism3_concurrent::{
    DoubleCheckedLockExecutor,
    lock::{ArcMutex, Lock},
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

    // Create executor
    let running_clone = running.clone();
    let executor = DoubleCheckedLockExecutor::builder()
        .tester_fn(move || running_clone.load(Ordering::Acquire))
        .logger(log::Level::Error, "Service is not running")
        .build()?;

    println!("Initial state: running = {}", running.load(Ordering::Acquire));
    println!("Initial data: {}", data.read(|d| *d));

    // Try to execute when service is not running (should fail)
    let result = executor.call_mut(&data, |value| {
        *value += 1;
        Ok::<_, ServiceError>(*value)
    });

    match result {
        Ok(value) => println!("Unexpected success: {}", value),
        Err(e) => println!("Expected failure: {}", e),
    }

    // Start the service
    running.store(true, Ordering::Release);
    println!("Service started: running = {}", running.load(Ordering::Acquire));

    // Now execute should succeed
    let result = executor.call_mut(&data, |value| {
        *value += 1;
        Ok::<_, ServiceError>(*value)
    });

    match result {
        Ok(value) => println!("Success: new value = {}", value),
        Err(e) => println!("Unexpected failure: {}", e),
    }

    // Verify the data was updated
    println!("Final data: {}", data.read(|d| *d));

    // Stop the service
    running.store(false, Ordering::Release);
    println!("Service stopped: running = {}", running.load(Ordering::Acquire));

    // Try to execute when service is stopped (should fail)
    let result = executor.call_mut(&data, |value| {
        *value += 1;
        Ok::<_, ServiceError>(*value)
    });

    match result {
        Ok(value) => println!("Unexpected success: {}", value),
        Err(e) => println!("Expected failure: {}", e),
    }

    Ok(())
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Task Execution
//!
//! Provides task-oriented abstractions for executing, submitting, and tracking
//! fallible one-time tasks.
//!
//! # Author
//!
//! Haixing Hu

pub mod executor;
pub mod service;
mod task_execution_error;
mod task_handle;
mod task_runner;

pub use qubit_function::{
    BoxCallable,
    BoxRunnable,
    Callable,
    Runnable,
};
pub use task_execution_error::{
    TaskExecutionError,
    TaskResult,
};
pub use task_handle::TaskHandle;

/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Double-Checked Lock Executor
//!
//! Provides a double-checked lock executor for executing tasks with condition
//! checking and rollback support.
//!
//! # Author
//!
//! Haixing Hu

pub mod config;
pub mod error;
pub mod result;
pub mod executor;

pub use config::{ExecutorConfig, LogConfig};
pub use error::{BuilderError, ExecutorError, ExecutionResult};

// Re-export the main executor and builder
pub use executor::{
    DoubleCheckedLockExecutor, DoubleCheckedLockExecutorBuilder,
};

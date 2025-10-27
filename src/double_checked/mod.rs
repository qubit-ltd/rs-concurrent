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

pub mod builder;
pub mod config;
pub mod error;
pub mod executor;
pub mod result;

pub use builder::ExecutionBuilder;
pub use config::{ExecutorConfig, LogConfig};
pub use error::{BuilderError, ExecutorError};
pub use executor::DoubleCheckedLockExecutor;
pub use result::ExecutionResult;

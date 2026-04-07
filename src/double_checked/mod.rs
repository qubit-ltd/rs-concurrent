/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
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
pub mod lock;
pub mod result;
pub mod states;

pub use builder::ExecutionBuilder;
pub use config::{
    ExecutorConfig,
    LogConfig,
};
pub use error::{
    BuilderError,
    ExecutorError,
};
pub use lock::DoubleCheckedLock;
pub use result::{
    ExecutionContext,
    ExecutionResult,
};
pub use states::{
    Conditioned,
    Configuring,
    Initial,
};

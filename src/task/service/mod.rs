/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Managed task services and lifecycle-related types.
//!
//! This module contains APIs that accept tasks into a managed service and expose
//! lifecycle control. Plain execution strategies live in
//! [`executor`](super::executor).
//!
//! # Author
//!
//! Haixing Hu

mod executor_service;
mod rejected_execution;
mod shutdown_report;
mod task_execution_service;
mod thread_per_task_executor_service;
mod thread_pool;
mod tokio_executor_service;
mod tokio_task_handle;

pub use executor_service::ExecutorService;
pub use rejected_execution::RejectedExecution;
pub use shutdown_report::ShutdownReport;
pub use task_execution_service::{
    TaskExecutionService,
    TaskExecutionServiceBuilder,
    TaskExecutionServiceError,
    TaskExecutionStats,
    TaskId,
    TaskStatus,
};
pub use thread_per_task_executor_service::ThreadPerTaskExecutorService;
pub use thread_pool::{
    ThreadPool,
    ThreadPoolBuildError,
    ThreadPoolBuilder,
    ThreadPoolStats,
};
pub use tokio_executor_service::TokioExecutorService;
pub use tokio_task_handle::TokioTaskHandle;

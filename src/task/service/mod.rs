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
//! The module exposes several execution domains:
//!
//! - [`BlockingExecutorService`] for synchronous tasks that may block an OS
//!   thread.
//! - [`RayonExecutorService`] for CPU-bound synchronous tasks.
//! - [`TokioBlockingExecutorService`] for blocking callables routed through
//!   Tokio's `spawn_blocking`.
//! - [`TokioIoExecutorService`] for async futures scheduled on Tokio's async
//!   runtime.
//!
//! # Author
//!
//! Haixing Hu

mod execution_services;
mod executor_service;
mod rayon_executor_service;
mod rejected_execution;
mod shutdown_report;
mod thread_per_task_executor_service;
mod thread_pool;
mod tokio_executor_service;
mod tokio_io_executor_service;
mod tokio_task_handle;

pub use execution_services::{
    ExecutionServices, ExecutionServicesBuildError, ExecutionServicesBuilder,
    ExecutionServicesShutdownReport,
};
pub use executor_service::ExecutorService;
pub use rayon_executor_service::{
    RayonExecutorService, RayonExecutorServiceBuildError, RayonExecutorServiceBuilder,
    RayonTaskHandle,
};
pub use rejected_execution::RejectedExecution;
pub use shutdown_report::ShutdownReport;
pub use thread_per_task_executor_service::ThreadPerTaskExecutorService;
pub use thread_pool::{
    PoolJob, ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder, ThreadPoolStats,
};
pub use tokio_executor_service::TokioExecutorService;
pub use tokio_io_executor_service::TokioIoExecutorService;
pub use tokio_task_handle::TokioTaskHandle;

/// Default managed service for synchronous tasks that may block an OS thread.
pub type BlockingExecutorService = ThreadPool;

/// Builder alias for configuring [`BlockingExecutorService`].
pub type BlockingExecutorServiceBuilder = ThreadPoolBuilder;

/// Tokio-backed blocking executor service routed through `spawn_blocking`.
pub type TokioBlockingExecutorService = TokioExecutorService;

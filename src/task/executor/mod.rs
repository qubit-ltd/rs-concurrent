/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Execution strategy abstractions and executor implementations.
//!
//! This module contains executors that describe how one task is executed. It
//! deliberately does not contain lifecycle or queue-management APIs; those live
//! in [`service`](super::service).
//!
//! # Author
//!
//! Haixing Hu

mod delay_executor;
mod direct_executor;
mod executor_trait;
mod future_executor;
mod thread_per_task_executor;
mod tokio_execution;
mod tokio_executor;

pub use delay_executor::DelayExecutor;
pub use direct_executor::DirectExecutor;
pub use executor_trait::Executor;
pub use future_executor::FutureExecutor;
pub use thread_per_task_executor::ThreadPerTaskExecutor;
pub use tokio_execution::TokioExecution;
pub use tokio_executor::TokioExecutor;

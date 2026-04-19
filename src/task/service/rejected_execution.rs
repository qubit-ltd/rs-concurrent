/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
use thiserror::Error;

/// Error returned when an executor service refuses to accept a task.
///
/// This error is about task acceptance only. It does not describe task
/// execution success or failure; accepted tasks report their final result
/// through the handle returned by the service.
///
/// # Author
///
/// Haixing Hu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RejectedExecution {
    /// The service has been shut down and no longer accepts new tasks.
    #[error("task rejected because the executor service is shut down")]
    Shutdown,

    /// The service is saturated and cannot accept more tasks.
    #[error("task rejected because the executor service is saturated")]
    Saturated,
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Configuration Types
//!
//! Provides configuration structures for the double-checked lock executor.
//!
//! # Author
//!
//! Haixing Hu

/// Log configuration
///
/// Configures logging behavior for the executor, including log level
/// and message content.
///
/// # Examples
///
/// ```rust,ignore
/// use log::Level;
/// use qubit_concurrent::double_checked::LogConfig;
///
/// let config = LogConfig {
///     level: Level::Info,
///     message: "Service is not running".to_string(),
/// };
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level for the message
    pub level: log::Level,

    /// Log message content
    pub message: String,
}

/// Executor configuration
///
/// Configures various execution options for the double-checked lock
/// executor, including performance metrics and error handling.
///
/// # Examples
///
/// ```rust,ignore
/// use log::Level;
/// use qubit_concurrent::double_checked::{ExecutorConfig, LogConfig};
///
/// let log_config = LogConfig {
///     level: Level::Warn,
///     message: "Service is not running".to_string(),
/// };
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Whether to enable performance metrics collection
    pub enable_metrics: bool,

    /// Whether to disable error backtrace for performance
    pub disable_backtrace: bool,
}

impl Default for ExecutorConfig {
    /// Creates a default executor configuration
    ///
    /// # Returns
    ///
    /// Returns a default configuration with metrics disabled and
    /// backtrace enabled.
    #[inline]
    fn default() -> Self {
        Self {
            enable_metrics: false,
            disable_backtrace: false,
        }
    }
}

/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Builder Error
//!
//! Provides builder error types for the double-checked lock executor.
//!
//! # Author
//!
//! Haixing Hu
use thiserror::Error;

/// Builder error types
///
/// Defines error conditions that can occur during executor builder
/// construction, such as missing required parameters.
///
/// # Examples
///
/// ```rust
/// use qubit_concurrent::double_checked::BuilderError;
///
/// let error = BuilderError::MissingTester;
/// println!("Builder error: {}", error);
/// ```
///
/// # Author
///
/// Haixing Hu
///
#[derive(Debug, Error)]
pub enum BuilderError {
    /// Missing required tester parameter
    #[error("Tester function is required")]
    MissingTester,
}

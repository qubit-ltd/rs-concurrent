/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Type State Markers
//!
//! Provides zero-sized type markers for the type state pattern.
//!
//! # Author
//!
//! Haixing Hu

/// Initial state: lock object has been set
///
/// This state indicates that the lock object has been set via the `on()`
/// method.
///
/// You can choose to configure logging or directly set test conditions.
///
/// # Author
///
/// Haixing Hu
pub struct Initial;

/// Configuration state: can set logger and other configuration items
///
/// This state indicates that the `logger()` method has been called.
/// You can continue configuration or set test conditions.
///
/// # Author
///
/// Haixing Hu
pub struct Configuring;

/// Condition set: when() has been called, can execute tasks
///
/// This state indicates that test conditions have been set via the `when()`
/// method.
///
/// You can choose to set preparation actions or directly execute tasks.
///
/// # Author
///
/// Haixing Hu
pub struct Conditioned;

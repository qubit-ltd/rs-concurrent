/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Lock Module
//!
//! Provides synchronous and asynchronous lock abstractions along
//! with their implementations. This module offers unified interfaces
//! for different types of locks through traits, making it easier to
//! write generic code that works with multiple lock types.
//!
//! # Author
//!
//! Haixing Hu

// Trait definitions
mod async_lock;
mod lock;

// Implementations
mod arc_async_mutex;
mod arc_async_rw_lock;
mod arc_mutex;
mod arc_rw_lock;
mod arc_std_mutex;

// Re-export traits
// Re-export implementations
pub use arc_async_mutex::ArcAsyncMutex;
pub use arc_async_rw_lock::ArcAsyncRwLock;
pub use arc_mutex::ArcMutex;
pub use arc_rw_lock::ArcRwLock;
pub use arc_std_mutex::ArcStdMutex;
pub use async_lock::AsyncLock;
pub use lock::Lock;

/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
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
mod lock;
mod async_lock;

// Implementations
mod arc_mutex;
mod arc_rw_lock;
mod arc_async_mutex;
mod arc_async_rw_lock;

// Re-export traits
pub use lock::Lock;
pub use async_lock::AsyncLock;

// Re-export implementations
pub use arc_mutex::ArcMutex;
pub use arc_rw_lock::ArcRwLock;
pub use arc_async_mutex::ArcAsyncMutex;
pub use arc_async_rw_lock::ArcAsyncRwLock;


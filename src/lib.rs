/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Prism3 Concurrent - Concurrency Utilities Library
//!
//! # Author
//!
//! Haixing Hu

pub mod executor;
pub mod lock;
pub mod double_checked_lock;

// Re-export main types and functions
pub use executor::{AsyncExecutor, AsyncExecutorService, Callable, Executor, ExecutorService, Runnable};
pub use lock::{
    ArcAsyncMutex, ArcAsyncRwLock, ArcMutex, ArcRwLock,
    AsyncLock, Lock,
};
pub use double_checked_lock::{
    DoubleCheckedLockExecutor, DoubleCheckedLockExecutorBuilder,
    ExecutionResult, ExecutorError, BuilderError,
    ExecutorConfig, LogConfig,
};

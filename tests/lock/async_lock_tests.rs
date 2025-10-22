/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # AsyncLock Trait Tests
//!
//! Tests for the AsyncLock trait and its implementation for tokio::sync::Mutex

use tokio::sync::Mutex as AsyncMutex;
use prism3_concurrent::AsyncLock;

#[cfg(test)]
mod async_lock_trait_tests {
    use super::*;

    #[tokio::test]
    async fn test_async_mutex_with_lock_basic_operations() {
        let async_mutex = AsyncMutex::new(0);

        // Test basic lock and modify
        let result = async_mutex
            .with_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);

        // Verify the value was persisted
        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_async_mutex_with_lock_returns_closure_result() {
        let async_mutex = AsyncMutex::new(vec![1, 2, 3]);

        let length = async_mutex.with_lock(|v| v.len()).await;
        assert_eq!(length, 3);

        let sum = async_mutex.with_lock(|v| v.iter().sum::<i32>()).await;
        assert_eq!(sum, 6);
    }

    #[tokio::test]
    async fn test_async_mutex_try_with_lock_success() {
        let async_mutex = AsyncMutex::new(42);

        // Should successfully acquire the lock
        let result = async_mutex.try_with_lock(|value| *value);
        assert_eq!(result, Some(42));

        // Should be able to modify
        let result = async_mutex.try_with_lock(|value| {
            *value += 1;
            *value
        });
        assert_eq!(result, Some(43));
    }

    #[tokio::test]
    async fn test_async_mutex_try_with_lock_returns_none_when_locked() {
        use std::sync::Arc;

        let async_mutex = Arc::new(AsyncMutex::new(0));

        // First, acquire the lock
        let mut guard = async_mutex.lock().await;

        // Create a new reference to try acquiring in parallel
        let async_mutex_clone = async_mutex.clone();

        // Spawn a task that will try to acquire the lock
        let handle = tokio::spawn(async move {
            // Try to acquire lock, should return None since it's held
            async_mutex_clone.try_with_lock(|value| *value)
        });

        // Wait for the spawned task
        let result = handle.await.unwrap();

        // Should be None because we're holding the lock
        assert!(
            result.is_none(),
            "Expected None when lock is held by another task"
        );

        // Modify the value
        *guard += 1;
        drop(guard);

        // Now should be able to successfully acquire the lock
        let result = async_mutex.try_with_lock(|value| *value);
        assert_eq!(result, Some(1));
    }

    #[tokio::test]
    async fn test_async_mutex_concurrent_access() {
        use std::sync::Arc;

        let async_mutex = Arc::new(AsyncMutex::new(0));
        let mut handles = vec![];

        // Create multiple tasks accessing the lock concurrently
        for _ in 0..10 {
            let async_mutex = Arc::clone(&async_mutex);
            let handle = tokio::spawn(async move {
                async_mutex
                    .with_lock(|value| {
                        *value += 1;
                    })
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final value
        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_async_mutex_with_lock_complex_types() {
        let async_mutex = AsyncMutex::new(String::from("Hello"));

        async_mutex
            .with_lock(|s| {
                s.push_str(" World");
            })
            .await;

        let result = async_mutex.with_lock(|s| s.clone()).await;
        assert_eq!(result, "Hello World");
    }

    #[tokio::test]
    async fn test_async_mutex_nested_operations() {
        let async_mutex = AsyncMutex::new(vec![1, 2, 3]);

        let result = async_mutex
            .with_lock(|v| {
                v.push(4);
                v.push(5);
                v.iter().map(|&x| x * 2).collect::<Vec<_>>()
            })
            .await;

        assert_eq!(result, vec![2, 4, 6, 8, 10]);

        // Verify original was modified
        let original = async_mutex.with_lock(|v| v.clone()).await;
        assert_eq!(original, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn test_async_mutex_fairness() {
        use std::sync::Arc;

        let async_mutex = Arc::new(AsyncMutex::new(Vec::new()));
        let mut handles = vec![];

        // Spawn multiple tasks that append their ID
        for i in 0..5 {
            let async_mutex = Arc::clone(&async_mutex);
            let handle = tokio::spawn(async move {
                async_mutex
                    .with_lock(|v| {
                        v.push(i);
                    })
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all tasks completed
        let result = async_mutex.with_lock(|v| v.len()).await;
        assert_eq!(result, 5);
    }

    #[tokio::test]
    async fn test_async_mutex_does_not_block_executor() {
        use std::sync::Arc;

        let async_mutex = Arc::new(AsyncMutex::new(0));
        let async_mutex_clone = async_mutex.clone();

        // Hold lock in one task
        let handle1 = tokio::spawn(async move {
            async_mutex_clone
                .with_lock(|value| {
                    *value += 1;
                    // Simulate long operation
                    std::thread::sleep(std::time::Duration::from_millis(50));
                })
                .await;
        });

        // Try to acquire lock in another task (should wait without blocking)
        let async_mutex_clone2 = async_mutex.clone();
        let handle2 = tokio::spawn(async move {
            // This should wait for lock to be released
            async_mutex_clone2
                .with_lock(|value| {
                    *value += 1;
                })
                .await;
        });

        // Both tasks should complete
        handle1.await.unwrap();
        handle2.await.unwrap();

        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 2);
    }

    #[tokio::test]
    async fn test_async_mutex_with_result_types() {
        let async_mutex = AsyncMutex::new(10);

        let result = async_mutex
            .with_lock(|value| -> Result<i32, &str> {
                if *value > 0 {
                    Ok(*value * 2)
                } else {
                    Err("value must be positive")
                }
            })
            .await;

        assert_eq!(result, Ok(20));
    }
}


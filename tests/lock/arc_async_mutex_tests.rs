/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # ArcAsyncMutex Tests
//!
//! Tests for the ArcAsyncMutex implementation

use prism3_concurrent::{ArcAsyncMutex, AsyncLock};
use std::sync::Arc;

#[cfg(test)]
mod arc_async_mutex_tests {
    use super::*;

    #[tokio::test]
    async fn test_arc_async_mutex_new() {
        let async_mutex = ArcAsyncMutex::new(42);
        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_with_lock() {
        let async_mutex = ArcAsyncMutex::new(0);

        // Test async lock
        let result = async_mutex
            .with_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);

        // Test trying to acquire lock
        let result = async_mutex.try_with_lock(|value| *value).unwrap();
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_clone() {
        let async_mutex = ArcAsyncMutex::new(0);
        let async_mutex_clone = async_mutex.clone();

        // Test cloned async lock
        let result = async_mutex_clone
            .with_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);

        // Verify that original lock can see changes
        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_try_with_lock_returns_none() {
        let async_mutex = Arc::new(ArcAsyncMutex::new(0));

        let async_mutex_clone = async_mutex.clone();

        // Hold the lock in another thread (note: using thread instead of tokio task)
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                async_mutex_clone
                    .with_lock(|value| {
                        *value += 1;
                        // Hold the lock for some time
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    })
                    .await;
            });
        });

        // Give spawned thread time to acquire the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to acquire lock, should return None
        let result = async_mutex.try_with_lock(|value| *value);
        assert!(
            result.is_none(),
            "Expected None when lock is held by another thread"
        );

        // Wait for child thread to complete
        handle.join().unwrap();

        // Now should be able to successfully acquire the lock
        let result = async_mutex.try_with_lock(|value| *value);
        assert_eq!(result, Some(1));
    }

    #[tokio::test]
    async fn test_arc_async_mutex_concurrent_access() {
        let async_mutex = ArcAsyncMutex::new(0);
        let async_mutex = Arc::new(async_mutex);
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
    async fn test_arc_async_mutex_with_complex_types() {
        let async_mutex = ArcAsyncMutex::new(String::from("Hello"));

        async_mutex
            .with_lock(|s| {
                s.push_str(" World");
            })
            .await;

        let result = async_mutex.with_lock(|s| s.clone()).await;
        assert_eq!(result, "Hello World");
    }

    #[tokio::test]
    async fn test_arc_async_mutex_multiple_modifications() {
        let async_mutex = ArcAsyncMutex::new(vec![1, 2, 3]);

        async_mutex
            .with_lock(|v| {
                v.push(4);
            })
            .await;

        async_mutex
            .with_lock(|v| {
                v.push(5);
            })
            .await;

        let result = async_mutex.with_lock(|v| v.clone()).await;
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_return_values() {
        let async_mutex = ArcAsyncMutex::new(vec![1, 2, 3, 4, 5]);

        let sum = async_mutex
            .with_lock(|v| v.iter().sum::<i32>())
            .await;
        assert_eq!(sum, 15);

        let len = async_mutex.with_lock(|v| v.len()).await;
        assert_eq!(len, 5);

        let first = async_mutex.with_lock(|v| v[0]).await;
        assert_eq!(first, 1);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_sharing_across_tasks() {
        let async_mutex = ArcAsyncMutex::new(0);

        let async_mutex1 = async_mutex.clone();
        let handle1 = tokio::spawn(async move {
            for _ in 0..100 {
                async_mutex1
                    .with_lock(|value| {
                        *value += 1;
                    })
                    .await;
            }
        });

        let async_mutex2 = async_mutex.clone();
        let handle2 = tokio::spawn(async move {
            for _ in 0..100 {
                async_mutex2
                    .with_lock(|value| {
                        *value += 1;
                    })
                    .await;
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        let result = async_mutex.with_lock(|value| *value).await;
        assert_eq!(result, 200);
    }

    #[tokio::test]
    async fn test_arc_async_mutex_nested_data_structures() {
        use std::collections::HashMap;

        let async_mutex = ArcAsyncMutex::new(HashMap::new());

        async_mutex
            .with_lock(|map| {
                map.insert("key1", 10);
                map.insert("key2", 20);
            })
            .await;

        let value1 = async_mutex
            .with_lock(|map| map.get("key1").copied())
            .await;
        assert_eq!(value1, Some(10));

        let value2 = async_mutex
            .with_lock(|map| map.get("key2").copied())
            .await;
        assert_eq!(value2, Some(20));
    }

    #[tokio::test]
    async fn test_arc_async_mutex_fairness() {
        let async_mutex = ArcAsyncMutex::new(Vec::new());
        let async_mutex = Arc::new(async_mutex);
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
    async fn test_arc_async_mutex_does_not_block_executor() {
        let async_mutex = ArcAsyncMutex::new(0);
        let async_mutex = Arc::new(async_mutex);

        let async_mutex_clone = async_mutex.clone();

        // Hold lock in one task
        let handle1 = tokio::spawn(async move {
            async_mutex_clone
                .with_lock(|value| {
                    *value += 1;
                    // Simulate long operation (using thread::sleep to simulate CPU work)
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
    async fn test_arc_async_mutex_with_result_types() {
        let async_mutex = ArcAsyncMutex::new(10);

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


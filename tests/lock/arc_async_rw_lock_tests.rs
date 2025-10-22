/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # ArcAsyncRwLock Tests
//!
//! Tests for the ArcAsyncRwLock implementation

use prism3_concurrent::{ArcAsyncRwLock, AsyncReadWriteLock};
use std::sync::Arc;

#[cfg(test)]
mod arc_async_rw_lock_tests {
    use super::*;

    #[tokio::test]
    async fn test_arc_async_rw_lock_new() {
        let async_rw_lock = ArcAsyncRwLock::new(42);
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_with_read_lock() {
        let async_rw_lock = ArcAsyncRwLock::new(0);

        // Test read lock
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_with_write_lock() {
        let async_rw_lock = ArcAsyncRwLock::new(0);

        // Test write lock
        let result = async_rw_lock
            .with_write_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_clone() {
        let async_rw_lock = ArcAsyncRwLock::new(0);
        let async_rw_lock_clone = async_rw_lock.clone();

        // Test cloned async read-write lock
        let result = async_rw_lock_clone
            .with_write_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);

        // Verify that original lock can see changes
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_concurrent_readers() {
        let async_rw_lock = ArcAsyncRwLock::new(vec![1, 2, 3, 4, 5]);
        let async_rw_lock = Arc::new(async_rw_lock);
        let mut handles = vec![];

        // Create multiple reader tasks
        for _ in 0..10 {
            let async_rw_lock = Arc::clone(&async_rw_lock);
            let handle = tokio::spawn(async move {
                async_rw_lock
                    .with_read_lock(|data| {
                        // Simulate some read operation
                        data.iter().sum::<i32>()
                    })
                    .await
            });
            handles.push(handle);
        }

        // All readers should get the same result
        for handle in handles {
            let sum = handle.await.unwrap();
            assert_eq!(sum, 15);
        }
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_write_lock_is_exclusive() {
        let async_rw_lock = ArcAsyncRwLock::new(0);
        let async_rw_lock = Arc::new(async_rw_lock);
        let mut handles = vec![];

        // Create multiple writer tasks
        for _ in 0..10 {
            let async_rw_lock = Arc::clone(&async_rw_lock);
            let handle = tokio::spawn(async move {
                async_rw_lock
                    .with_write_lock(|value| {
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

        // Verify final value (should be 10 if writes are exclusive)
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 10);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_read_after_write() {
        let async_rw_lock = ArcAsyncRwLock::new(String::from("Hello"));

        // Write operation
        async_rw_lock
            .with_write_lock(|s| {
                s.push_str(" World");
            })
            .await;

        // Read operation should see the change
        let result = async_rw_lock.with_read_lock(|s| s.clone()).await;
        assert_eq!(result, "Hello World");
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_with_complex_types() {
        let async_rw_lock = ArcAsyncRwLock::new(vec![1, 2, 3]);

        // Multiple readers can access concurrently
        let len = async_rw_lock.with_read_lock(|v| v.len()).await;
        assert_eq!(len, 3);

        // Writer modifies the data
        async_rw_lock
            .with_write_lock(|v| {
                v.push(4);
                v.push(5);
            })
            .await;

        // Reader sees the updated data
        let sum = async_rw_lock
            .with_read_lock(|v| v.iter().sum::<i32>())
            .await;
        assert_eq!(sum, 15);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_read_lock_returns_closure_result() {
        let async_rw_lock = ArcAsyncRwLock::new(vec![10, 20, 30]);

        let result = async_rw_lock
            .with_read_lock(|v| v.iter().map(|&x| x * 2).collect::<Vec<_>>())
            .await;

        assert_eq!(result, vec![20, 40, 60]);

        // Original should be unchanged
        let original = async_rw_lock.with_read_lock(|v| v.clone()).await;
        assert_eq!(original, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_write_lock_returns_closure_result() {
        let async_rw_lock = ArcAsyncRwLock::new(5);

        let result = async_rw_lock
            .with_write_lock(|value| {
                *value *= 2;
                *value
            })
            .await;

        assert_eq!(result, 10);

        // Verify the value was actually modified
        let current = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(current, 10);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_mixed_read_write_operations() {
        let async_rw_lock = ArcAsyncRwLock::new(0);
        let async_rw_lock = Arc::new(async_rw_lock);
        let mut handles = vec![];

        // Create some readers
        for _ in 0..5 {
            let async_rw_lock = Arc::clone(&async_rw_lock);
            let handle = tokio::spawn(async move {
                for _ in 0..10 {
                    async_rw_lock
                        .with_read_lock(|value| {
                            let _ = *value;
                        })
                        .await;
                }
            });
            handles.push(handle);
        }

        // Create some writers
        for _ in 0..5 {
            let async_rw_lock = Arc::clone(&async_rw_lock);
            let handle = tokio::spawn(async move {
                for _ in 0..10 {
                    async_rw_lock
                        .with_write_lock(|value| {
                            *value += 1;
                        })
                        .await;
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify final value
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 50); // 5 writers × 10 increments each
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_readers_do_not_block_each_other() {
        let async_rw_lock = ArcAsyncRwLock::new(vec![1, 2, 3, 4, 5]);
        let async_rw_lock = Arc::new(async_rw_lock);
        let mut handles = vec![];

        // Create multiple readers that all access the lock simultaneously
        for i in 0..5 {
            let async_rw_lock = Arc::clone(&async_rw_lock);
            let handle = tokio::spawn(async move {
                // All readers should be able to access concurrently
                async_rw_lock
                    .with_read_lock(|data| {
                        data.iter().sum::<i32>() + i
                    })
                    .await
            });
            handles.push(handle);
        }

        // All readers should successfully complete and return results
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // Verify all readers got the correct sum (15) plus their index
        assert_eq!(results.len(), 5);
        for (i, &result) in results.iter().enumerate() {
            assert_eq!(result, 15 + i as i32);
        }
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_writer_blocks_readers() {
        let async_rw_lock = ArcAsyncRwLock::new(0);
        let async_rw_lock = Arc::new(async_rw_lock);

        // Hold write lock in one task
        let async_rw_lock_clone = async_rw_lock.clone();
        let write_handle = tokio::spawn(async move {
            async_rw_lock_clone
                .with_write_lock(|value| {
                    *value += 1;
                    // Hold the write lock for some time
                    std::thread::sleep(std::time::Duration::from_millis(50));
                })
                .await;
        });

        // Give the write task time to acquire the lock
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Try to read (should wait for write to complete)
        let read_result = async_rw_lock.with_read_lock(|value| *value).await;

        // Wait for write task to complete
        write_handle.await.unwrap();

        // Should see the updated value
        assert_eq!(read_result, 1);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_sharing_across_tasks() {
        let async_rw_lock = ArcAsyncRwLock::new(0);

        let async_rw_lock1 = async_rw_lock.clone();
        let handle1 = tokio::spawn(async move {
            for _ in 0..50 {
                async_rw_lock1
                    .with_write_lock(|value| {
                        *value += 1;
                    })
                    .await;
            }
        });

        let async_rw_lock2 = async_rw_lock.clone();
        let handle2 = tokio::spawn(async move {
            for _ in 0..50 {
                async_rw_lock2
                    .with_write_lock(|value| {
                        *value += 1;
                    })
                    .await;
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_nested_data_structures() {
        use std::collections::HashMap;

        let async_rw_lock = ArcAsyncRwLock::new(HashMap::new());

        async_rw_lock
            .with_write_lock(|map| {
                map.insert("key1", 10);
                map.insert("key2", 20);
            })
            .await;

        let value1 = async_rw_lock
            .with_read_lock(|map| map.get("key1").copied())
            .await;
        assert_eq!(value1, Some(10));

        let value2 = async_rw_lock
            .with_read_lock(|map| map.get("key2").copied())
            .await;
        assert_eq!(value2, Some(20));
    }

    #[tokio::test]
    async fn test_arc_async_rw_lock_with_result_types() {
        let async_rw_lock = ArcAsyncRwLock::new(10);

        let result = async_rw_lock
            .with_read_lock(|value| -> Result<i32, &str> {
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


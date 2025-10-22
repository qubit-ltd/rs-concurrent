/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # AsyncReadWriteLock Trait Tests
//!
//! Tests for the AsyncReadWriteLock trait and its implementation for tokio::sync::RwLock

use tokio::sync::RwLock as AsyncRwLock;
use prism3_concurrent::AsyncReadWriteLock;

#[cfg(test)]
mod async_read_write_lock_trait_tests {
    use super::*;

    #[tokio::test]
    async fn test_async_rwlock_with_read_lock_basic() {
        let async_rw_lock = AsyncRwLock::new(42);

        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_rwlock_with_write_lock_basic() {
        let async_rw_lock = AsyncRwLock::new(0);

        let result = async_rw_lock
            .with_write_lock(|value| {
                *value += 1;
                *value
            })
            .await;
        assert_eq!(result, 1);

        // Verify the value was persisted
        let result = async_rw_lock.with_read_lock(|value| *value).await;
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_async_rwlock_concurrent_readers() {
        use std::sync::Arc;

        let async_rw_lock = Arc::new(AsyncRwLock::new(vec![1, 2, 3, 4, 5]));
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
    async fn test_async_rwlock_write_lock_is_exclusive() {
        use std::sync::Arc;

        let async_rw_lock = Arc::new(AsyncRwLock::new(0));
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
    async fn test_async_rwlock_read_after_write() {
        let async_rw_lock = AsyncRwLock::new(String::from("Hello"));

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
    async fn test_async_rwlock_with_complex_types() {
        let async_rw_lock = AsyncRwLock::new(vec![1, 2, 3]);

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
    async fn test_async_rwlock_read_lock_returns_closure_result() {
        let async_rw_lock = AsyncRwLock::new(vec![10, 20, 30]);

        let result = async_rw_lock
            .with_read_lock(|v| v.iter().map(|&x| x * 2).collect::<Vec<_>>())
            .await;

        assert_eq!(result, vec![20, 40, 60]);

        // Original should be unchanged
        let original = async_rw_lock.with_read_lock(|v| v.clone()).await;
        assert_eq!(original, vec![10, 20, 30]);
    }

    #[tokio::test]
    async fn test_async_rwlock_write_lock_returns_closure_result() {
        let async_rw_lock = AsyncRwLock::new(5);

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
    async fn test_async_rwlock_mixed_read_write_operations() {
        use std::sync::Arc;

        let async_rw_lock = Arc::new(AsyncRwLock::new(0));
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
    async fn test_async_rwlock_readers_do_not_block_each_other() {
        use std::sync::Arc;

        let async_rw_lock = Arc::new(AsyncRwLock::new(vec![1, 2, 3, 4, 5]));
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
    async fn test_async_rwlock_writer_blocks_readers() {
        use std::sync::Arc;

        let async_rw_lock = Arc::new(AsyncRwLock::new(0));

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
    async fn test_async_rwlock_with_result_types() {
        let async_rw_lock = AsyncRwLock::new(10);

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


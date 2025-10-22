/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # ReadWriteLock Trait Tests
//!
//! Tests for the ReadWriteLock trait and its implementation for std::sync::RwLock

use std::sync::{Arc, RwLock};
use std::thread;
use prism3_concurrent::ReadWriteLock;

#[cfg(test)]
mod read_write_lock_trait_tests {
    use super::*;

    #[test]
    fn test_rwlock_with_read_lock_basic() {
        let rw_lock = RwLock::new(42);

        let result = rw_lock.with_read_lock(|value| *value);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_rwlock_with_write_lock_basic() {
        let rw_lock = RwLock::new(0);

        let result = rw_lock.with_write_lock(|value| {
            *value += 1;
            *value
        });
        assert_eq!(result, 1);

        // Verify the value was persisted
        let result = rw_lock.with_read_lock(|value| *value);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_rwlock_concurrent_readers() {
        let rw_lock = Arc::new(RwLock::new(vec![1, 2, 3, 4, 5]));
        let mut handles = vec![];

        // Create multiple reader threads
        for _ in 0..10 {
            let rw_lock = Arc::clone(&rw_lock);
            let handle = thread::spawn(move || {
                rw_lock.with_read_lock(|data| {
                    // Simulate some read operation
                    thread::sleep(std::time::Duration::from_millis(10));
                    data.iter().sum::<i32>()
                })
            });
            handles.push(handle);
        }

        // All readers should get the same result
        for handle in handles {
            let sum = handle.join().unwrap();
            assert_eq!(sum, 15);
        }
    }

    #[test]
    fn test_rwlock_write_lock_is_exclusive() {
        let rw_lock = Arc::new(RwLock::new(0));
        let mut handles = vec![];

        // Create multiple writer threads
        for _ in 0..10 {
            let rw_lock = Arc::clone(&rw_lock);
            let handle = thread::spawn(move || {
                rw_lock.with_write_lock(|value| {
                    *value += 1;
                });
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final value (should be 10 if writes are exclusive)
        let result = rw_lock.with_read_lock(|value| *value);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_rwlock_read_after_write() {
        let rw_lock = RwLock::new(String::from("Hello"));

        // Write operation
        rw_lock.with_write_lock(|s| {
            s.push_str(" World");
        });

        // Read operation should see the change
        let result = rw_lock.with_read_lock(|s| s.clone());
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_rwlock_with_complex_types() {
        let rw_lock = RwLock::new(vec![1, 2, 3]);

        // Multiple readers can access concurrently
        let len = rw_lock.with_read_lock(|v| v.len());
        assert_eq!(len, 3);

        // Writer modifies the data
        rw_lock.with_write_lock(|v| {
            v.push(4);
            v.push(5);
        });

        // Reader sees the updated data
        let sum = rw_lock.with_read_lock(|v| v.iter().sum::<i32>());
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_rwlock_read_lock_returns_closure_result() {
        let rw_lock = RwLock::new(vec![10, 20, 30]);

        let result = rw_lock.with_read_lock(|v| {
            v.iter().map(|&x| x * 2).collect::<Vec<_>>()
        });

        assert_eq!(result, vec![20, 40, 60]);

        // Original should be unchanged
        let original = rw_lock.with_read_lock(|v| v.clone());
        assert_eq!(original, vec![10, 20, 30]);
    }

    #[test]
    fn test_rwlock_write_lock_returns_closure_result() {
        let rw_lock = RwLock::new(5);

        let result = rw_lock.with_write_lock(|value| {
            *value *= 2;
            *value
        });

        assert_eq!(result, 10);

        // Verify the value was actually modified
        let current = rw_lock.with_read_lock(|value| *value);
        assert_eq!(current, 10);
    }

    #[test]
    #[should_panic(expected = "PoisonError")]
    fn test_rwlock_read_panics_on_poisoned() {
        let rw_lock = Arc::new(RwLock::new(0));

        let rw_lock_clone = rw_lock.clone();

        // Poison the lock by panicking while holding write lock
        let handle = thread::spawn(move || {
            rw_lock_clone.with_write_lock(|value| {
                *value += 1;
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for thread to panic
        let _ = handle.join();

        // Try to acquire read lock on poisoned lock, should panic
        rw_lock.with_read_lock(|_| {});
    }

    #[test]
    #[should_panic(expected = "PoisonError")]
    fn test_rwlock_write_panics_on_poisoned() {
        let rw_lock = Arc::new(RwLock::new(0));

        let rw_lock_clone = rw_lock.clone();

        // Poison the lock by panicking while holding write lock
        let handle = thread::spawn(move || {
            rw_lock_clone.with_write_lock(|value| {
                *value += 1;
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for thread to panic
        let _ = handle.join();

        // Try to acquire write lock on poisoned lock, should panic
        rw_lock.with_write_lock(|_| {});
    }

    #[test]
    fn test_rwlock_mixed_read_write_operations() {
        let rw_lock = Arc::new(RwLock::new(0));
        let mut handles = vec![];

        // Create some readers
        for _ in 0..5 {
            let rw_lock = Arc::clone(&rw_lock);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    rw_lock.with_read_lock(|value| {
                        let _ = *value;
                    });
                }
            });
            handles.push(handle);
        }

        // Create some writers
        for _ in 0..5 {
            let rw_lock = Arc::clone(&rw_lock);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    rw_lock.with_write_lock(|value| {
                        *value += 1;
                    });
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final value
        let result = rw_lock.with_read_lock(|value| *value);
        assert_eq!(result, 50); // 5 writers × 10 increments each
    }
}


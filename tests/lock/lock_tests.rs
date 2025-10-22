/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Lock Trait Tests
//!
//! Tests for the Lock trait and its implementation for std::sync::Mutex

use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use prism3_concurrent::Lock;

#[cfg(test)]
mod lock_trait_tests {
    use super::*;

    #[test]
    fn test_mutex_with_lock_basic_operations() {
        let mutex = Mutex::new(0);

        // Test basic lock and modify
        let result = mutex.with_lock(|value| {
            *value += 1;
            *value
        });
        assert_eq!(result, 1);

        // Verify the value was persisted
        let result = mutex.with_lock(|value| *value);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_mutex_with_lock_returns_closure_result() {
        let mutex = Mutex::new(vec![1, 2, 3]);

        let length = mutex.with_lock(|v| v.len());
        assert_eq!(length, 3);

        let sum = mutex.with_lock(|v| v.iter().sum::<i32>());
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_mutex_try_with_lock_success() {
        let mutex = Mutex::new(42);

        // Should successfully acquire the lock
        let result = mutex.try_with_lock(|value| *value);
        assert_eq!(result, Some(42));

        // Should be able to modify
        let result = mutex.try_with_lock(|value| {
            *value += 1;
            *value
        });
        assert_eq!(result, Some(43));
    }

    #[test]
    fn test_mutex_try_with_lock_returns_none_when_locked() {
        let mutex = Arc::new(Mutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Hold the lock in another thread
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                // Notify main thread
                barrier_clone.wait();
                // Hold the lock for some time
                thread::sleep(std::time::Duration::from_millis(100));
            });
        });

        // Wait for child thread to acquire the lock
        barrier.wait();

        // Try to acquire lock, should return None
        let result = mutex.try_with_lock(|value| *value);
        assert!(
            result.is_none(),
            "Expected None when lock is held by another thread"
        );

        // Wait for child thread to complete
        handle.join().unwrap();

        // Now should be able to successfully acquire the lock
        let result = mutex.try_with_lock(|value| *value);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_mutex_concurrent_access() {
        let mutex = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        // Create multiple threads accessing the lock concurrently
        for _ in 0..10 {
            let mutex = Arc::clone(&mutex);
            let handle = thread::spawn(move || {
                mutex.with_lock(|value| {
                    *value += 1;
                });
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final value
        let result = mutex.with_lock(|value| *value);
        assert_eq!(result, 10);
    }

    #[test]
    #[should_panic(expected = "PoisonError")]
    fn test_mutex_with_lock_panics_on_poisoned() {
        let mutex = Arc::new(Mutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Poison the lock by panicking while holding it
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                barrier_clone.wait();
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for child thread to acquire the lock
        barrier.wait();

        // Wait for child thread to panic
        let _ = handle.join();

        // Try to acquire poisoned lock, should panic
        mutex.with_lock(|_| {});
    }

    #[test]
    fn test_mutex_try_with_lock_returns_none_on_poisoned() {
        let mutex = Arc::new(Mutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Poison the lock by panicking while holding it
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                barrier_clone.wait();
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for child thread to acquire the lock
        barrier.wait();

        // Wait for child thread to panic
        let _ = handle.join();

        // Try to acquire poisoned lock, should return None
        let result = mutex.try_with_lock(|value| *value);
        assert!(
            result.is_none(),
            "Expected None for poisoned lock"
        );
    }

    #[test]
    fn test_mutex_with_lock_complex_types() {
        let mutex = Mutex::new(String::from("Hello"));

        mutex.with_lock(|s| {
            s.push_str(" World");
        });

        let result = mutex.with_lock(|s| s.clone());
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_mutex_nested_operations() {
        let mutex = Mutex::new(vec![1, 2, 3]);

        let result = mutex.with_lock(|v| {
            v.push(4);
            v.push(5);
            v.iter().map(|&x| x * 2).collect::<Vec<_>>()
        });

        assert_eq!(result, vec![2, 4, 6, 8, 10]);

        // Verify original was modified
        let original = mutex.with_lock(|v| v.clone());
        assert_eq!(original, vec![1, 2, 3, 4, 5]);
    }
}


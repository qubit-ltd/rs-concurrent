/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # ArcMutex Tests
//!
//! Tests for the ArcMutex implementation

use prism3_concurrent::{ArcMutex, Lock};
use std::sync::{Arc, Barrier};
use std::thread;

#[cfg(test)]
mod arc_mutex_tests {
    use super::*;

    #[test]
    fn test_arc_mutex_new() {
        let mutex = ArcMutex::new(42);
        let result = mutex.with_lock(|value| *value);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_arc_mutex_with_lock_basic_operations() {
        let mutex = ArcMutex::new(0);

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
    fn test_arc_mutex_try_with_lock_success() {
        let mutex = ArcMutex::new(42);

        let result = mutex.try_with_lock(|value| *value).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_arc_mutex_clone() {
        let mutex = ArcMutex::new(0);
        let mutex_clone = mutex.clone();

        // Test that cloned lock shares data
        let result = mutex_clone.with_lock(|value| {
            *value += 1;
            *value
        });
        assert_eq!(result, 1);

        // Verify that original lock can see changes
        let result = mutex.with_lock(|value| *value);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_arc_mutex_try_with_lock_returns_none() {
        let mutex = Arc::new(ArcMutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Hold the lock in another thread
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                // Notify main thread that it can try to acquire the lock
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
    fn test_arc_mutex_try_with_lock_poisoned() {
        let mutex = Arc::new(ArcMutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Hold the lock and panic in another thread
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                // Notify main thread that lock has been acquired
                barrier_clone.wait();
                // Panic while holding the lock, causing the lock to be poisoned
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for child thread to acquire the lock
        barrier.wait();

        // Wait for child thread to complete panicking (will poison the lock)
        let _ = handle.join();

        // Try to acquire poisoned lock, should return None
        let result = mutex.try_with_lock(|value| *value);
        assert!(
            result.is_none(),
            "Expected None for poisoned lock, got {:?}",
            result
        );
    }

    #[test]
    #[should_panic(expected = "PoisonError")]
    fn test_arc_mutex_with_lock_poisoned() {
        let mutex = Arc::new(ArcMutex::new(0));
        let barrier = Arc::new(Barrier::new(2));

        let mutex_clone = mutex.clone();
        let barrier_clone = barrier.clone();

        // Hold the lock and panic in another thread
        let handle = thread::spawn(move || {
            mutex_clone.with_lock(|value| {
                *value += 1;
                // Notify main thread that lock has been acquired
                barrier_clone.wait();
                // Panic while holding the lock, causing the lock to be poisoned
                panic!("intentional panic to poison the lock");
            });
        });

        // Wait for child thread to acquire the lock
        barrier.wait();

        // Wait for child thread to complete panicking (will poison the lock)
        let _ = handle.join();

        // Try to acquire poisoned lock with with_lock (not try_with_lock)
        // This should panic because with_lock uses unwrap()
        mutex.with_lock(|_| {});
    }

    #[test]
    fn test_arc_mutex_concurrent_access() {
        let mutex = ArcMutex::new(0);
        let mutex = Arc::new(mutex);

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
    fn test_arc_mutex_with_complex_types() {
        let mutex = ArcMutex::new(String::from("Hello"));

        mutex.with_lock(|s| {
            s.push_str(" World");
        });

        let result = mutex.with_lock(|s| s.clone());
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_arc_mutex_multiple_modifications() {
        let mutex = ArcMutex::new(vec![1, 2, 3]);

        mutex.with_lock(|v| {
            v.push(4);
        });

        mutex.with_lock(|v| {
            v.push(5);
        });

        let result = mutex.with_lock(|v| v.clone());
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_arc_mutex_return_values() {
        let mutex = ArcMutex::new(vec![1, 2, 3, 4, 5]);

        let sum = mutex.with_lock(|v| v.iter().sum::<i32>());
        assert_eq!(sum, 15);

        let len = mutex.with_lock(|v| v.len());
        assert_eq!(len, 5);

        let first = mutex.with_lock(|v| v[0]);
        assert_eq!(first, 1);
    }

    #[test]
    fn test_arc_mutex_sharing_across_threads() {
        let mutex = ArcMutex::new(0);

        let mutex1 = mutex.clone();
        let handle1 = thread::spawn(move || {
            for _ in 0..100 {
                mutex1.with_lock(|value| {
                    *value += 1;
                });
            }
        });

        let mutex2 = mutex.clone();
        let handle2 = thread::spawn(move || {
            for _ in 0..100 {
                mutex2.with_lock(|value| {
                    *value += 1;
                });
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        let result = mutex.with_lock(|value| *value);
        assert_eq!(result, 200);
    }

    #[test]
    fn test_arc_mutex_nested_data_structures() {
        use std::collections::HashMap;

        let mutex = ArcMutex::new(HashMap::new());

        mutex.with_lock(|map| {
            map.insert("key1", 10);
            map.insert("key2", 20);
        });

        let value1 = mutex.with_lock(|map| map.get("key1").copied());
        assert_eq!(value1, Some(10));

        let value2 = mutex.with_lock(|map| map.get("key2").copied());
        assert_eq!(value2, Some(20));
    }
}


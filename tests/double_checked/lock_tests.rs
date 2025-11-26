/*******************************************************************************
 *
 *    Copyright (c) 2025.
 *    3-Prism Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
#[cfg(test)]
mod tests {
    use std::io;

    use prism3_concurrent::{
        double_checked::DoubleCheckedLock,
        lock::{
            ArcRwLock,
            ArcStdMutex,
            Lock,
        },
    };

    mod test_double_checked_lock_on {
        use super::*;

        #[test]
        fn test_double_checked_lock_on_with_arc_mutex() {
            let data = ArcStdMutex::new(42);

            let builder = DoubleCheckedLock::on(&data);

            // Verify the builder type
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                builder;
        }

        #[test]
        fn test_double_checked_lock_on_with_arc_rw_lock() {
            let data = ArcRwLock::new(String::from("test"));

            let builder = DoubleCheckedLock::on(&data);

            // Verify the builder type
            let _: prism3_concurrent::double_checked::ExecutionBuilder<
                ArcRwLock<String>,
                String,
                _,
            > = builder;
        }

        #[test]
        fn test_double_checked_lock_on_with_different_types() {
            // Test with various data types
            let int_data = ArcStdMutex::new(100);
            let string_data = ArcStdMutex::new("hello".to_string());
            let vec_data = ArcStdMutex::new(vec![1, 2, 3]);
            let option_data = ArcStdMutex::new(Some(42));

            let _ = DoubleCheckedLock::on(&int_data);
            let _ = DoubleCheckedLock::on(&string_data);
            let _ = DoubleCheckedLock::on(&vec_data);
            let _ = DoubleCheckedLock::on(&option_data);
        }
    }

    mod test_double_checked_lock_integration {
        use super::*;

        #[test]
        fn test_double_checked_lock_simple_read() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), 42);
        }

        #[test]
        fn test_double_checked_lock_simple_write() {
            let data = ArcStdMutex::new(0);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call_mut(|value: &mut i32| {
                    *value = 100;
                    Ok::<(), io::Error>(())
                })
                .get_result();

            assert!(result.is_success());
            assert_eq!(data.read(|v| *v), 100);
        }

        #[test]
        fn test_double_checked_lock_with_rw_lock_read() {
            let data = ArcRwLock::new(vec![1, 2, 3]);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|vec: &Vec<i32>| Ok::<Vec<i32>, io::Error>(vec.clone()))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), vec![1, 2, 3]);
        }

        #[test]
        fn test_double_checked_lock_with_rw_lock_write() {
            let data = ArcRwLock::new(vec![1, 2, 3]);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call_mut(|vec: &mut Vec<i32>| {
                    vec.push(4);
                    Ok::<(), io::Error>(())
                })
                .get_result();

            assert!(result.is_success());
            assert_eq!(data.read(|v| v.clone()), vec![1, 2, 3, 4]);
        }

        #[test]
        fn test_double_checked_lock_condition_not_met() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert_eq!(data.read(|v| *v), 42);
        }

        #[test]
        fn test_double_checked_lock_with_logger() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .logger(log::Level::Info, "Test execution")
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), 42);
        }

        #[test]
        fn test_double_checked_lock_with_prepare() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .prepare(|| Ok::<(), io::Error>(()))
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), 42);
        }

        #[test]
        fn test_double_checked_lock_task_failure() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call_mut(|value: &mut i32| {
                    *value = 100;
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Task failed"))
                })
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_failed());
            // Note: value was modified before failure
            assert_eq!(data.read(|v| *v), 100);
        }

        #[test]
        fn test_double_checked_lock_with_rollback() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call_mut(|value: &mut i32| {
                    *value = 100;
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Task failed"))
                })
                .rollback(|| Ok::<(), io::Error>(()))
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_failed());
        }
    }
}

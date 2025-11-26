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
    use std::{
        io,
        sync::{
            atomic::{
                AtomicBool,
                Ordering,
            },
            Arc,
        },
    };

    use prism3_concurrent::{
        double_checked::DoubleCheckedLock,
        lock::{
            ArcStdMutex,
            Lock,
        },
    };

    mod test_execution_builder_initial_state {
        use super::*;

        #[test]
        fn test_execution_builder_logger() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data);

            let configured = builder.logger(log::Level::Info, "Test message");

            // Verify it transitions to Configuring state
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                configured;
        }

        #[test]
        fn test_execution_builder_when() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data);

            let conditioned = builder.when(|| true);

            // Verify it transitions to Conditioned state
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                conditioned;
        }
    }

    mod test_execution_builder_configuring_state {
        use super::*;

        #[test]
        fn test_execution_builder_configuring_logger_override() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data);

            let configuring = builder.logger(log::Level::Info, "First message");
            let configuring = configuring.logger(log::Level::Warn, "Second message");

            // Should stay in Configuring state
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                configuring;
        }

        #[test]
        fn test_execution_builder_configuring_when() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data);

            let configuring = builder.logger(log::Level::Info, "Test message");
            let conditioned = configuring.when(|| true);

            // Should transition to Conditioned state
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                conditioned;
        }
    }

    mod test_execution_builder_conditioned_state {
        use super::*;

        #[test]
        fn test_execution_builder_prepare() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data).when(|| true);

            let prepared = builder.prepare(|| Ok::<(), io::Error>(()));

            // Should stay in Conditioned state
            let _: prism3_concurrent::double_checked::ExecutionBuilder<ArcStdMutex<i32>, i32, _> =
                prepared;
        }

        #[test]
        fn test_execution_builder_call() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data).when(|| true);

            let context = builder.call(|value: &i32| Ok::<i32, io::Error>(*value));

            // Should return ExecutionContext
            assert!(context.is_success());
            assert_eq!(context.get_result().unwrap(), 42);
        }

        #[test]
        fn test_execution_builder_call_mut() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data).when(|| true);

            let context = builder.call_mut(|value: &mut i32| {
                *value = 100;
                Ok::<i32, io::Error>(*value)
            });

            assert!(context.is_success());
            assert_eq!(context.get_result().unwrap(), 100);
            assert_eq!(data.read(|v| *v), 100);
        }

        #[test]
        fn test_execution_builder_execute() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data).when(|| true);

            let context = builder.execute(|_value: &i32| Ok::<(), io::Error>(()));

            assert!(context.is_success());
            assert_eq!(data.read(|v| *v), 42);
        }

        #[test]
        fn test_execution_builder_execute_mut() {
            let data = ArcStdMutex::new(42);
            let builder = DoubleCheckedLock::on(&data).when(|| true);

            let context = builder.execute_mut(|value: &mut i32| {
                *value = 200;
                Ok::<(), io::Error>(())
            });

            assert!(context.is_success());
            assert_eq!(data.read(|v| *v), 200);
        }
    }

    mod test_execution_builder_full_flow {
        use super::*;

        #[test]
        fn test_execution_builder_full_flow_success() {
            let data = ArcStdMutex::new(10);
            let condition = Arc::new(AtomicBool::new(true));

            let result = DoubleCheckedLock::on(&data)
                .logger(log::Level::Info, "Test execution")
                .when({
                    let condition = condition.clone();
                    move || condition.load(Ordering::Acquire)
                })
                .prepare(|| {
                    // Simulate some preparation
                    Ok::<(), io::Error>(())
                })
                .call_mut(|value: &mut i32| {
                    *value += 5;
                    Ok::<i32, io::Error>(*value)
                })
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), 15);
            assert_eq!(data.read(|v| *v), 15);
        }

        #[test]
        fn test_execution_builder_condition_not_met() {
            let data = ArcStdMutex::new(10);
            let condition = Arc::new(AtomicBool::new(false));

            let result = DoubleCheckedLock::on(&data)
                .when({
                    let condition = condition.clone();
                    move || condition.load(Ordering::Acquire)
                })
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert_eq!(data.read(|v| *v), 10);
        }

        #[test]
        fn test_execution_builder_prepare_fails() {
            let data = ArcStdMutex::new(10);
            let condition = Arc::new(AtomicBool::new(true));

            let result = DoubleCheckedLock::on(&data)
                .when({
                    let condition = condition.clone();
                    move || condition.load(Ordering::Acquire)
                })
                .prepare(|| Err(io::Error::new(io::ErrorKind::Other, "Prepare failed")))
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_failed());
            assert_eq!(data.read(|v| *v), 10);
        }

        #[test]
        fn test_execution_builder_task_fails() {
            let data = ArcStdMutex::new(10);
            let condition = Arc::new(AtomicBool::new(true));

            let result = DoubleCheckedLock::on(&data)
                .when({
                    let condition = condition.clone();
                    move || condition.load(Ordering::Acquire)
                })
                .call_mut(|value: &mut i32| {
                    *value = 20;
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Task failed"))
                })
                .get_result();

            assert!(!result.is_success());
            assert!(result.is_failed());
            // Note: value was still modified before failure
            assert_eq!(data.read(|v| *v), 20);
        }

        #[test]
        fn test_execution_builder_with_rollback() {
            let data = ArcStdMutex::new(10);
            let condition = Arc::new(AtomicBool::new(true));
            let rollback_called = Arc::new(AtomicBool::new(false));

            let mut context = DoubleCheckedLock::on(&data)
                .when({
                    let condition = condition.clone();
                    move || condition.load(Ordering::Acquire)
                })
                .call_mut(|value: &mut i32| {
                    *value = 30;
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Task failed"))
                });

            let rollback_called_clone = rollback_called.clone();
            context = context.rollback(move || {
                rollback_called_clone.store(true, Ordering::Release);
                Ok::<(), io::Error>(())
            });

            let result = context.get_result();

            assert!(!result.is_success());
            assert!(result.is_failed());
            assert!(rollback_called.load(Ordering::Acquire));
            assert_eq!(data.read(|v| *v), 30);
        }
    }

    mod test_execution_builder_edge_cases {
        use super::*;

        #[test]
        fn test_execution_builder_with_complex_data_types() {
            let data = ArcStdMutex::new(vec![1, 2, 3]);

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call_mut(|vec: &mut Vec<i32>| {
                    vec.push(4);
                    Ok::<Vec<i32>, io::Error>(vec.clone())
                })
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), vec![1, 2, 3, 4]);
            assert_eq!(data.read(|v| v.clone()), vec![1, 2, 3, 4]);
        }

        #[test]
        fn test_execution_builder_with_option_data() {
            let data = ArcStdMutex::new(Some(42));

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|opt: &Option<i32>| Ok::<Option<i32>, io::Error>(opt.as_ref().copied()))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), Some(42));
        }

        #[test]
        fn test_execution_builder_multiple_loggers() {
            let data = ArcStdMutex::new(42);

            let result = DoubleCheckedLock::on(&data)
                .logger(log::Level::Debug, "First log")
                .logger(log::Level::Info, "Second log")
                .logger(log::Level::Warn, "Third log")
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(result.is_success());
            assert_eq!(result.unwrap(), 42);
        }

        #[test]
        fn test_execution_builder_prepare_called_multiple_times() {
            let data = ArcStdMutex::new(42);
            let prepare_count = Arc::new(AtomicBool::new(false));

            let result = DoubleCheckedLock::on(&data)
                .when(|| true)
                .prepare({
                    let prepare_count = prepare_count.clone();
                    move || {
                        prepare_count.store(true, Ordering::Release);
                        Ok::<(), io::Error>(())
                    }
                })
                .prepare({
                    // Second prepare should override the first one
                    let prepare_count = prepare_count.clone();
                    move || {
                        prepare_count.store(false, Ordering::Release); // Reset to false to show override
                        prepare_count.store(true, Ordering::Release); // Then set to true
                        Ok::<(), io::Error>(())
                    }
                })
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            assert!(result.is_success());
            assert!(prepare_count.load(Ordering::Acquire)); // Should be true from second prepare
        }

        #[test]
        fn test_execution_builder_condition_changes_between_checks() {
            use std::sync::atomic::{
                AtomicI32,
                Ordering,
            };

            let data = ArcStdMutex::new(10);
            let call_count = Arc::new(AtomicI32::new(0));

            let result = DoubleCheckedLock::on(&data)
                .when({
                    let call_count = call_count.clone();
                    move || {
                        let count = call_count.fetch_add(1, Ordering::AcqRel) + 1;
                        count == 1 // Only first check passes
                    }
                })
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            // Should fail because second check (inside lock) fails
            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert_eq!(data.read(|v| *v), 10); // Data unchanged
            assert_eq!(call_count.load(Ordering::Acquire), 2); // Called twice: once outside, once inside lock
        }

        #[test]
        fn test_execution_builder_prepare_logging_on_condition_not_met() {
            let data = ArcStdMutex::new(10);

            let result = DoubleCheckedLock::on(&data)
                .logger(log::Level::Info, "Condition not met, skipping execution")
                .when(|| false) // Condition never met
                .prepare(|| Ok::<(), io::Error>(()))
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .get_result();

            // Should fail due to condition not met
            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert_eq!(data.read(|v| *v), 10); // Data unchanged
        }

        #[test]
        fn test_execution_builder_write_lock_condition_changes_between_checks() {
            use std::sync::atomic::{
                AtomicI32,
                Ordering,
            };

            let data = ArcStdMutex::new(10);
            let call_count = Arc::new(AtomicI32::new(0));

            let result = DoubleCheckedLock::on(&data)
                .when({
                    let call_count = call_count.clone();
                    move || {
                        let count = call_count.fetch_add(1, Ordering::AcqRel) + 1;
                        count == 1 // Only first check passes
                    }
                })
                .call_mut(|value: &mut i32| {
                    *value = 20;
                    Ok::<(), io::Error>(())
                })
                .get_result();

            // Should fail because second check (inside lock) fails
            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert_eq!(data.read(|v| *v), 10); // Data unchanged (should not be modified)
            assert_eq!(call_count.load(Ordering::Acquire), 2); // Called twice: once outside, once inside lock
        }
    }
}

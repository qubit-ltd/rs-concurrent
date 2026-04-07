/*******************************************************************************
 *
 *    Copyright (c) 2025 - 2026.
 *    Haixing Hu, Qubit Co. Ltd.
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

    use qubit_concurrent::{
        double_checked::DoubleCheckedLock,
        lock::ArcStdMutex,
    };

    mod test_execution_context {
        use super::*;

        #[test]
        fn test_execution_context_creation_success() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            assert!(context.is_success());
            assert!(matches!(
                context.peek_result(),
                qubit_concurrent::double_checked::ExecutionResult::Success(42)
            ));
        }

        #[test]
        fn test_execution_context_creation_condition_not_met() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            assert!(!context.is_success());
        }

        #[test]
        fn test_execution_context_creation_failed() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Test error"))
                });

            assert!(!context.is_success());
        }

        #[test]
        fn test_execution_context_get_result_success() {
            let data = ArcStdMutex::new(100);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let final_result = context.get_result();
            assert!(matches!(
                final_result,
                qubit_concurrent::double_checked::ExecutionResult::Success(100)
            ));
        }

        #[test]
        fn test_execution_context_get_result_condition_not_met() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let final_result = context.get_result();
            assert!(matches!(
                final_result,
                qubit_concurrent::double_checked::ExecutionResult::ConditionNotMet
            ));
        }

        #[test]
        fn test_execution_context_get_result_failed_no_rollback() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Original error"))
                });

            let final_result = context.get_result();
            if let qubit_concurrent::double_checked::ExecutionResult::Failed(
                qubit_concurrent::double_checked::ExecutorError::TaskFailed(e),
            ) = final_result
            {
                assert!(e.to_string().contains("Original error"));
            } else {
                panic!("Expected TaskFailed error");
            }
        }

        #[test]
        fn test_execution_context_rollback_on_failure() {
            let rollback_called = Arc::new(AtomicBool::new(false));
            let data = ArcStdMutex::new(42);

            let mut context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Original error"))
                });

            let rollback_called_clone = rollback_called.clone();
            context = context.rollback(move || {
                rollback_called_clone.store(true, Ordering::Release);
                Ok::<(), io::Error>(())
            });

            let final_result = context.get_result();

            // Check that rollback was called
            assert!(rollback_called.load(Ordering::Acquire));

            // Check that the result is still the original error since rollback succeeded
            if let qubit_concurrent::double_checked::ExecutionResult::Failed(
                qubit_concurrent::double_checked::ExecutorError::TaskFailed(e),
            ) = final_result
            {
                assert!(e.to_string().contains("Original error"));
            } else {
                panic!("Expected TaskFailed error");
            }
        }

        #[test]
        fn test_execution_context_rollback_fails() {
            let data = ArcStdMutex::new(42);
            let mut context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "Original error"))
                });

            context =
                context.rollback(|| Err(io::Error::new(io::ErrorKind::Other, "Rollback failed")));

            let final_result = context.get_result();

            // Check that the result is now RollbackFailed
            if let qubit_concurrent::double_checked::ExecutionResult::Failed(
                qubit_concurrent::double_checked::ExecutorError::RollbackFailed {
                    original,
                    rollback,
                },
            ) = final_result
            {
                assert!(original.contains("Original error"));
                assert!(rollback.contains("Rollback failed"));
            } else {
                panic!("Expected RollbackFailed error");
            }
        }

        #[test]
        fn test_execution_context_rollback_not_called_on_success() {
            let rollback_called = Arc::new(AtomicBool::new(false));
            let data = ArcStdMutex::new(42);

            let mut context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let rollback_called_clone = rollback_called.clone();
            context = context.rollback(move || {
                rollback_called_clone.store(true, Ordering::Release);
                Ok::<(), io::Error>(())
            });

            let _ = context.get_result();

            // Check that rollback was NOT called
            assert!(!rollback_called.load(Ordering::Acquire));
        }

        #[test]
        fn test_execution_context_rollback_not_called_on_condition_not_met() {
            let rollback_called = Arc::new(AtomicBool::new(false));
            let data = ArcStdMutex::new(42);

            let mut context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let rollback_called_clone = rollback_called.clone();
            context = context.rollback(move || {
                rollback_called_clone.store(true, Ordering::Release);
                Ok::<(), io::Error>(())
            });

            let _ = context.get_result();

            // Check that rollback was NOT called
            assert!(!rollback_called.load(Ordering::Acquire));
        }

        #[test]
        fn test_execution_context_peek_result() {
            let data = ArcStdMutex::new(123);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let peeked = context.peek_result();
            assert!(matches!(
                peeked,
                qubit_concurrent::double_checked::ExecutionResult::Success(123)
            ));
        }

        #[test]
        fn test_execution_context_is_success() {
            let data1 = ArcStdMutex::new(1);
            let data2 = ArcStdMutex::new(42);
            let data3 = ArcStdMutex::new(42);

            let success_context = DoubleCheckedLock::on(&data1)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let unmet_context = DoubleCheckedLock::on(&data2)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            let failed_context = DoubleCheckedLock::on(&data3)
                .when(|| true)
                .call(|_value: &i32| Err::<i32, _>(io::Error::new(io::ErrorKind::Other, "error")));

            assert!(success_context.is_success());
            assert!(!unmet_context.is_success());
            assert!(!failed_context.is_success());
        }
    }

    mod test_execution_context_unit_type {
        use super::*;

        #[test]
        fn test_execution_context_finish_success() {
            let data = ArcStdMutex::new(());
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .execute(|_: &()| Ok::<(), io::Error>(()));

            assert!(context.finish());
        }

        #[test]
        fn test_execution_context_finish_condition_not_met() {
            let data = ArcStdMutex::new(());
            let context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .execute(|_: &()| Ok::<(), io::Error>(()));

            assert!(!context.finish());
        }

        #[test]
        fn test_execution_context_finish_failed() {
            let data = ArcStdMutex::new(());
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .execute(|_: &()| Err::<(), _>(io::Error::new(io::ErrorKind::Other, "error")));

            assert!(!context.finish());
        }

        #[test]
        fn test_execution_context_finish_with_rollback() {
            let rollback_called = Arc::new(AtomicBool::new(false));
            let data = ArcStdMutex::new(());

            let mut context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .execute(|_: &()| Err::<(), _>(io::Error::new(io::ErrorKind::Other, "error")));

            let rollback_called_clone = rollback_called.clone();
            context = context.rollback(move || {
                rollback_called_clone.store(true, Ordering::Release);
                Ok::<(), io::Error>(())
            });

            assert!(!context.finish());
            assert!(rollback_called.load(Ordering::Acquire));
        }
    }
}

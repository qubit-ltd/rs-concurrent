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
    use qubit_concurrent::double_checked::{
        ExecutionResult,
        ExecutorError,
    };
    use qubit_concurrent::lock::ArcStdMutex;
    use qubit_concurrent::DoubleCheckedLock;

    mod test_execution_result {
        use super::*;

        #[test]
        fn test_execution_result_success() {
            let result = ExecutionResult::<i32, String>::Success(42);

            assert!(result.is_success());
            assert!(!result.is_unmet());
            assert!(!result.is_failed());
        }

        #[test]
        fn test_execution_result_condition_not_met() {
            let result = ExecutionResult::<i32, String>::ConditionNotMet;

            assert!(!result.is_success());
            assert!(result.is_unmet());
            assert!(!result.is_failed());
        }

        #[test]
        fn test_execution_result_failed() {
            let error = ExecutorError::<String>::TaskFailed("Test error".to_string());
            let result = ExecutionResult::<i32, String>::Failed(error);

            assert!(!result.is_success());
            assert!(!result.is_unmet());
            assert!(result.is_failed());
        }

        #[test]
        fn test_execution_result_unwrap_success() {
            let result = ExecutionResult::<i32, String>::Success(100);
            assert_eq!(result.unwrap(), 100);
        }

        #[test]
        #[should_panic(expected = "Called unwrap on ExecutionResult::ConditionNotMet")]
        fn test_execution_result_unwrap_condition_not_met_panics() {
            let result = ExecutionResult::<i32, String>::ConditionNotMet;
            result.unwrap();
        }

        #[test]
        #[should_panic(expected = "Called unwrap on ExecutionResult::Failed")]
        fn test_execution_result_unwrap_failed_panics() {
            let error = ExecutorError::<String>::TaskFailed("Test error".to_string());
            let result = ExecutionResult::<i32, String>::Failed(error);
            result.unwrap();
        }

        #[test]
        fn test_execution_result_into_result_success() {
            let result = ExecutionResult::<i32, String>::Success(42);
            let converted = result.into_result();

            assert!(converted.is_ok());
            assert_eq!(converted.unwrap(), Some(42));
        }

        #[test]
        fn test_execution_result_into_result_condition_not_met() {
            let result = ExecutionResult::<i32, String>::ConditionNotMet;
            let converted = result.into_result();

            assert!(converted.is_ok());
            assert_eq!(converted.unwrap(), None);
        }

        #[test]
        fn test_execution_result_into_result_failed() {
            let error = ExecutorError::<String>::TaskFailed("Test error".to_string());
            let result = ExecutionResult::<i32, String>::Failed(error);
            let converted = result.into_result();

            assert!(converted.is_err());
            if let Err(ExecutorError::TaskFailed(msg)) = converted {
                assert_eq!(msg, "Test error");
            } else {
                panic!("Expected TaskFailed error");
            }
        }

        #[test]
        fn test_execution_result_debug_success() {
            let result = ExecutionResult::<i32, String>::Success(42);
            let debug_str = format!("{:?}", result);
            assert!(debug_str.contains("Success"));
            assert!(debug_str.contains("42"));
        }

        #[test]
        fn test_execution_result_debug_condition_not_met() {
            let result = ExecutionResult::<i32, String>::ConditionNotMet;
            let debug_str = format!("{:?}", result);
            assert!(debug_str.contains("ConditionNotMet"));
        }

        #[test]
        fn test_execution_result_debug_failed() {
            let error = ExecutorError::<String>::TaskFailed("Test error".to_string());
            let result = ExecutionResult::<i32, String>::Failed(error);
            let debug_str = format!("{:?}", result);
            assert!(debug_str.contains("Failed"));
            assert!(debug_str.contains("TaskFailed"));
        }

        #[test]
        fn test_execution_result_with_complex_types() {
            // Test with Vec
            let result = ExecutionResult::<Vec<i32>, String>::Success(vec![1, 2, 3]);
            assert!(result.is_success());
            assert_eq!(result.unwrap(), vec![1, 2, 3]);

            // Test with Option
            let result =
                ExecutionResult::<Option<String>, String>::Success(Some("test".to_string()));
            assert!(result.is_success());
            assert_eq!(result.unwrap(), Some("test".to_string()));
        }

        #[test]
        fn test_execution_result_with_unit_type() {
            let result = ExecutionResult::<(), String>::Success(());
            assert!(result.is_success());
            assert_eq!(result.unwrap(), ());

            let result = ExecutionResult::<(), String>::ConditionNotMet;
            assert!(result.is_unmet());
        }
    }

    mod test_execution_context {
        use super::*;
        use std::io;

        #[test]
        fn test_execution_context_success() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            assert!(context.is_success());
            assert!(matches!(
                context.peek_result(),
                ExecutionResult::Success(42)
            ));

            let final_result = context.get_result();
            assert!(matches!(final_result, ExecutionResult::Success(42)));
        }

        #[test]
        fn test_execution_context_condition_not_met() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value));

            assert!(!context.is_success());
            assert!(matches!(
                context.peek_result(),
                ExecutionResult::ConditionNotMet
            ));

            let final_result = context.get_result();
            assert!(matches!(final_result, ExecutionResult::ConditionNotMet));
        }

        #[test]
        fn test_execution_context_failed() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::other("Task failed"))
                });

            assert!(!context.is_success());
            assert!(matches!(context.peek_result(), ExecutionResult::Failed(_)));

            let final_result = context.get_result();
            assert!(matches!(final_result, ExecutionResult::Failed(_)));
        }

        #[test]
        fn test_execution_context_rollback_on_failure() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::other("Task failed"))
                })
                .rollback(|| Ok::<(), io::Error>(()));

            let final_result = context.get_result();
            // Should still be failed since rollback succeeded
            assert!(matches!(final_result, ExecutionResult::Failed(_)));
        }

        #[test]
        fn test_execution_context_rollback_fails() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|_value: &i32| {
                    Err::<i32, _>(io::Error::other("Task failed"))
                })
                .rollback(|| Err::<(), _>(io::Error::other("Rollback failed")));

            let final_result = context.get_result();
            // Should be RollbackFailed
            assert!(matches!(
                final_result,
                ExecutionResult::Failed(ExecutorError::RollbackFailed { .. })
            ));
        }

        #[test]
        fn test_execution_context_no_rollback_on_success() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .rollback(|| {
                    Err::<(), _>(io::Error::other("Should not execute"))
                });

            let final_result = context.get_result();
            // Should still be success
            assert!(matches!(final_result, ExecutionResult::Success(42)));
        }

        #[test]
        fn test_execution_context_no_rollback_on_condition_not_met() {
            let data = ArcStdMutex::new(42);
            let context = DoubleCheckedLock::on(&data)
                .when(|| false)
                .call(|value: &i32| Ok::<i32, io::Error>(*value))
                .rollback(|| {
                    Err::<(), _>(io::Error::other("Should not execute"))
                });

            let final_result = context.get_result();
            // Should still be condition not met
            assert!(matches!(final_result, ExecutionResult::ConditionNotMet));
        }

        #[test]
        fn test_execution_context_unit_type_finish_success() {
            let data = ArcStdMutex::new(());
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .execute(|_value: &()| Ok::<(), io::Error>(()));

            assert!(context.finish());
        }

        #[test]
        fn test_execution_context_unit_type_finish_failure() {
            let data = ArcStdMutex::new(());
            let context = DoubleCheckedLock::on(&data)
                .when(|| true)
                .execute(|_value: &()| {
                    Err::<(), _>(io::Error::other("Task failed"))
                });

            assert!(!context.finish());
        }
    }
}

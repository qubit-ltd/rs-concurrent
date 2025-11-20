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
        error::Error,
        fmt,
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
            ArcMutex,
            ArcRwLock,
            Lock,
        },
    };

    #[derive(Debug)]
    struct TestError(&'static str);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn test_call_mut_simple_success() {
        let data = ArcMutex::new(10);
        let condition = Arc::new(AtomicBool::new(true));

        let result = DoubleCheckedLock::on(&data)
            .when({
                let condition = condition.clone();
                move || condition.load(Ordering::Acquire)
            })
            .call_mut(|value: &mut i32| {
                *value += 1;
                Ok::<i32, TestError>(*value)
            })
            .get_result();

        assert!(result.is_success());
        assert_eq!(result.into_result().unwrap(), Some(11));
        assert_eq!(data.read(|d| *d), 11);
    }

    #[test]
    fn test_call_simple_success() {
        let data = ArcRwLock::new(10);
        let condition = Arc::new(AtomicBool::new(true));

        let result = DoubleCheckedLock::on(&data)
            .when({
                let condition = condition.clone();
                move || condition.load(Ordering::Acquire)
            })
            .call(|value: &i32| Ok::<i32, TestError>(*value))
            .get_result();

        assert!(result.is_success());
        assert_eq!(result.into_result().unwrap(), Some(10));
        assert_eq!(data.read(|d| *d), 10);
    }

    #[test]
    fn test_when_condition_is_false() {
        let data = ArcMutex::new(10);
        let condition = Arc::new(AtomicBool::new(false));

        let result = DoubleCheckedLock::on(&data)
            .when({
                let condition = condition.clone();
                move || condition.load(Ordering::Acquire)
            })
            .call_mut(|value: &mut i32| {
                *value += 1;
                Ok::<i32, TestError>(*value)
            })
            .get_result();

        assert!(!result.is_success());
        assert!(result.is_unmet());
        assert_eq!(data.read(|d| *d), 10);
    }

    #[test]
    fn test_task_fails_and_rollback_is_called() {
        let data = ArcMutex::new(10);
        let condition = Arc::new(AtomicBool::new(true));
        let rollback_called = Arc::new(AtomicBool::new(false));

        let result = DoubleCheckedLock::on(&data)
            .when({
                let condition = condition.clone();
                move || condition.load(Ordering::Acquire)
            })
            .call_mut(|value: &mut i32| {
                *value += 1;
                Err::<i32, _>(TestError("task failed"))
            })
            .rollback({
                let rollback_called = rollback_called.clone();
                move || {
                    rollback_called.store(true, Ordering::Release);
                    Ok::<(), TestError>(())
                }
            })
            .get_result();

        assert!(!result.is_success());
        assert!(result.is_failed());
        assert_eq!(data.read(|d| *d), 11); // value was still changed before failure
        assert!(rollback_called.load(Ordering::Acquire));
    }

    #[test]
    fn test_prepare_fails() {
        let data = ArcMutex::new(10);
        let condition = Arc::new(AtomicBool::new(true));

        let result = DoubleCheckedLock::on(&data)
            .when({
                let condition = condition.clone();
                move || condition.load(Ordering::Acquire)
            })
            .prepare(|| Err(TestError("prepare failed")))
            .call_mut(|value: &mut i32| {
                *value += 1;
                Ok::<i32, TestError>(*value)
            })
            .get_result();

        assert!(!result.is_success());
        assert!(result.is_failed());
        assert_eq!(data.read(|d| *d), 10); // task should not have run
    }
}

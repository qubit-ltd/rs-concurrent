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
        ExecutorConfig,
        LogConfig,
    };

    mod test_log_config {
        use super::*;

        #[test]
        fn test_log_config_creation() {
            let config = LogConfig {
                level: log::Level::Info,
                message: "Test message".to_string(),
            };

            assert_eq!(config.level, log::Level::Info);
            assert_eq!(config.message, "Test message");
        }

        #[test]
        fn test_log_config_debug() {
            let config = LogConfig {
                level: log::Level::Warn,
                message: "Warning message".to_string(),
            };

            let debug_str = format!("{:?}", config);
            assert!(debug_str.contains("LogConfig"));
            assert!(debug_str.contains("Warn"));
            assert!(debug_str.contains("Warning message"));
        }

        #[test]
        fn test_log_config_clone() {
            let config = LogConfig {
                level: log::Level::Error,
                message: "Error occurred".to_string(),
            };

            let cloned = config.clone();
            assert_eq!(cloned.level, config.level);
            assert_eq!(cloned.message, config.message);
        }

        #[test]
        fn test_log_config_with_empty_message() {
            let config = LogConfig {
                level: log::Level::Debug,
                message: String::new(),
            };

            assert_eq!(config.level, log::Level::Debug);
            assert!(config.message.is_empty());
        }

        #[test]
        fn test_log_config_with_unicode_message() {
            let config = LogConfig {
                level: log::Level::Info,
                message: "测试消息 🚀".to_string(),
            };

            assert_eq!(config.message, "测试消息 🚀");
        }
    }

    mod test_executor_config {
        use super::*;

        #[test]
        fn test_executor_config_default() {
            let config = ExecutorConfig::default();

            assert_eq!(config.enable_metrics, false);
            assert_eq!(config.disable_backtrace, false);
        }

        #[test]
        fn test_executor_config_creation() {
            let config = ExecutorConfig {
                enable_metrics: true,
                disable_backtrace: true,
            };

            assert_eq!(config.enable_metrics, true);
            assert_eq!(config.disable_backtrace, true);
        }

        #[test]
        fn test_executor_config_debug() {
            let config = ExecutorConfig {
                enable_metrics: true,
                disable_backtrace: false,
            };

            let debug_str = format!("{:?}", config);
            assert!(debug_str.contains("ExecutorConfig"));
            assert!(debug_str.contains("enable_metrics: true"));
            assert!(debug_str.contains("disable_backtrace: false"));
        }

        #[test]
        fn test_executor_config_clone() {
            let config = ExecutorConfig {
                enable_metrics: false,
                disable_backtrace: true,
            };

            let cloned = config.clone();
            assert_eq!(cloned.enable_metrics, config.enable_metrics);
            assert_eq!(cloned.disable_backtrace, config.disable_backtrace);
        }
    }
}

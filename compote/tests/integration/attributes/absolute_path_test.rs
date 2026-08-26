//! Tests for absolute_path validation attribute.

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Test absolute_path validation failure with default
#[test]
fn test_absolute_path_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct AbsolutePathConfig {
        #[compote(absolute_path, default = "/default/path")]
        config_path: String,
    }

    // "relative/path" is not absolute
    let config_str = r#"{"config_path": "relative/path"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: AbsolutePathConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.config_path, "/default/path", "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("config_path") || msg.contains("absolute") || msg.contains("path")
        }),
        "Expected absolute path validation error, got: {:?}",
        errors
    );
}

/// Test absolute_path validation on required field - fails deserialization
#[test]
fn test_absolute_path_required_field_fails_on_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredAbsoluteConfig {
        /// Required field - must be absolute path, no default
        #[compote(absolute_path)]
        log_path: String,
    }

    // "relative/path" is not absolute
    let config_str = r#"{"log_path": "relative/log.txt"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredAbsoluteConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field fails absolute_path validation"
    );
}

/// Test absolute_path validation success with Unix path
#[test]
fn test_absolute_path_unix_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct AbsolutePathConfig {
        #[compote(absolute_path, default = "/default/path")]
        config_path: String,
    }

    // "/etc/config.yaml" is absolute
    let config_str = r#"{"config_path": "/etc/config.yaml"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: AbsolutePathConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.config_path, "/etc/config.yaml",
        "should be the value"
    );
    assert!(
        !config.errors().has_errors(),
        "Should not have errors for valid absolute path"
    );
}

/// Test absolute_path validation success with Windows-style path
#[test]
#[cfg(target_os = "windows")]
fn test_absolute_path_windows_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct AbsolutePathConfig {
        #[compote(absolute_path, default = "C:\\default\\path")]
        config_path: String,
    }

    // "C:\Program Files\app\config.yaml" is absolute on Windows
    let config_str = r#"{"config_path": "C:\\Program Files\\app\\config.yaml"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: AbsolutePathConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.config_path, "C:\\Program Files\\app\\config.yaml");
    assert!(!config.errors().has_errors());
}

/// Test various relative path formats are rejected
#[test]
fn test_absolute_path_rejects_various_relative_formats() {
    #[derive(DeriveConfig, Debug)]
    struct AbsolutePathConfig {
        #[compote(absolute_path, default = "/default")]
        path: String,
    }

    let relative_paths = vec![
        "relative/path",
        "./current/path",
        "../parent/path",
        "just_filename.txt",
        "path/to/file",
    ];

    for path in relative_paths {
        let config_str = format!(r#"{{"path": "{}"}}"#, path);

        let mut config = Config::default();
        config.load_json(&config_str, Context::new(Source::Programmatic, Level::User));

        let result: AbsolutePathConfig = config.deserialize().expect("Should succeed with default");

        assert_eq!(
            result.path, "/default",
            "Path '{}' should be rejected as relative",
            path
        );
        assert!(
            config.errors().has_errors(),
            "Path '{}' should have recorded an error",
            path
        );
    }
}

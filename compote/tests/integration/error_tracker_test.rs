//! Core tests for error tracking during deserialization.
//!
//! These tests verify the fundamental error tracking behavior:
//! - Deserialization errors are recorded in the tracker
//! - Errors are accessible after deserialization via errors()
//! - Fields with defaults use default values on error and continue
//! - Required fields (no default) still fail deserialization
//!
//! For specific error scenarios, see:
//! - validators/ - Range, length, regex, custom validation tests
//! - transforms/ - Duration, coerce, relative_path tests
//! - deserialization/ - Primitives, type mismatch, collections tests
//! - env_test.rs - Environment variable tests

use compote::{Config, Context, ErrorTracker, Format, Level, Source};
use compote_macros::Config as DeriveConfig;

#[test]
fn test_child_tracker_inherits_path_and_commits_diagnostics() {
    let mut tracker = ErrorTracker::new();
    tracker.push_field("config");
    tracker.record_warning("parent warning");

    let mut child = tracker.child();
    assert_eq!(child.current_path(), "config");
    assert!(!child.has_errors());
    assert!(!child.has_warnings());

    child.push_field("value");
    child.record_invalid_value("invalid child value");
    child.record_warning("child warning");

    tracker.commit_child(child);

    assert_eq!(tracker.current_path(), "config");
    assert_eq!(tracker.errors().len(), 1);
    assert_eq!(tracker.errors()[0].location(), "config.value");
    assert_eq!(tracker.warnings().len(), 2);
    assert_eq!(tracker.warnings()[1].path, "config.value");
}

#[test]
fn test_record_warning_at_preserves_explicit_path() {
    let mut tracker = ErrorTracker::new();
    tracker.push_field("current");

    tracker.record_warning_at("copied.nested.path", "copied warning");

    assert_eq!(tracker.current_path(), "current");
    assert_eq!(tracker.warnings()[0].path, "copied.nested.path");
    assert_eq!(tracker.warnings()[0].message, "copied warning");
}

/// Struct for testing deserialization error recording
#[derive(DeriveConfig, Debug, PartialEq)]
struct ErrorTestConfig {
    /// Field with default - if deserialization fails, use default and record error
    #[compote(default = "fallback")]
    name: String,

    /// Numeric field with default - type mismatch should record error
    #[compote(default = "42")]
    count: i32,

    /// Field with validation that might fail
    #[compote(default = "valid")]
    status: String,
}

/// Test that deserialization errors are accessible via loader.errors()
///
/// Deserialization succeeds with default values when fields fail to parse.
/// Errors are recorded in the tracker for the caller to check and decide
/// whether to fail.
#[test]
fn test_deserialization_errors_accessible_via_loader() {
    // This config has a type mismatch: count expects int but gets an object
    let config_with_error = r#"
name: "test"
count:
  nested: "this is wrong type for count"
"#;

    let mut loader = compote::loader()
        .load_str(config_with_error, Format::Yaml, Level::User)
        .expect("Failed to load config");

    // Deserialization SUCCEEDS with defaults for fields that fail
    let config: ErrorTestConfig = loader
        .deserialize()
        .expect("Deserialization should succeed with defaults");

    // Name should be from config
    assert_eq!(config.name, "test");

    // Count should fall back to default due to type mismatch
    assert_eq!(
        config.count, 42,
        "count should use default value due to type error"
    );

    // Status should be default (not in config)
    assert_eq!(config.status, "valid");

    // Errors should be recorded for the caller to inspect
    let errors = loader.errors().errors();
    println!("=== Recorded Errors ({}) ===", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}: {}", i + 1, error);
    }

    // Should have the type mismatch error for 'count'
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string();
            msg.contains("count")
                || msg.contains("type")
                || msg.contains("mismatch")
                || msg.contains("integer")
        }),
        "Expected a type-related error for 'count' field, got: {:?}",
        errors
    );

    // User can decide to fail based on errors
    if loader.errors().has_errors() {
        println!(
            "Caller could choose to fail here due to {} error(s)",
            errors.len()
        );
    }
}

/// Test error access when using Config directly (not via loader)
///
/// This test verifies that when using Config::deserialize() directly,
/// errors recorded during deserialization are accessible via config.errors().
/// Deserialization succeeds with defaults, and errors are recorded.
#[test]
fn test_deserialization_errors_accessible_via_config() {
    let json_with_error = r#"{"name": "test", "count": "not_a_number"}"#;

    let mut config = Config::default();
    config.load_json(
        json_with_error,
        Context::new(Source::Programmatic, Level::User),
    );

    // Deserialization SUCCEEDS with defaults for fields that fail
    let result: ErrorTestConfig = config
        .deserialize()
        .expect("Deserialization should succeed with defaults");

    assert_eq!(result.name, "test");
    assert_eq!(
        result.count, 42,
        "count should use default due to parse error"
    );

    // Errors should be accessible via config.errors()
    let errors = config.errors().errors();
    println!("=== Direct Config Recorded Errors ({}) ===", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}: {}", i + 1, error);
    }

    // Should have an error about parsing 'count'
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string();
            msg.contains("count")
                || msg.contains("parse")
                || msg.contains("i32")
                || msg.contains("i64")
        }),
        "Expected parse error for 'count', got: {:?}",
        errors
    );

    // User can check for errors and decide to fail
    assert!(
        config.errors().has_errors(),
        "Should have errors recorded that caller can check"
    );
}

/// Test that required fields (no default) still fail deserialization
#[test]
fn test_required_field_fails_deserialization() {
    /// Struct with a required field (no default)
    #[derive(DeriveConfig, Debug)]
    struct RequiredFieldConfig {
        /// This field is REQUIRED - no default
        required_name: String,

        /// This field has a default
        #[compote(default = "optional")]
        optional_field: String,
    }

    // Config missing the required field
    let json_missing_required = r#"{"optional_field": "test"}"#;

    let mut config = Config::default();
    config.load_json(
        json_missing_required,
        Context::new(Source::Programmatic, Level::User),
    );

    // Deserialization should FAIL because required field is missing
    let result: Result<RequiredFieldConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Deserialization should fail when required field is missing"
    );

    let err = result.unwrap_err();
    println!("=== Required Field Error ===\n{}", err);

    // Error should mention the missing field
    assert!(
        err.to_string().contains("required_name"),
        "Error should mention the missing field name"
    );
}

/// Test that required field with wrong type also fails
#[test]
fn test_required_field_wrong_type_fails() {
    /// Struct with a required field (no default)
    #[derive(DeriveConfig, Debug)]
    struct RequiredIntConfig {
        /// This field is REQUIRED and must be an integer
        required_count: i32,
    }

    // Config with wrong type for required field
    let json_wrong_type = r#"{"required_count": "not_a_number"}"#;

    let mut config = Config::default();
    config.load_json(
        json_wrong_type,
        Context::new(Source::Programmatic, Level::User),
    );

    // Deserialization should FAIL because required field has wrong type
    let result: Result<RequiredIntConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Deserialization should fail when required field has wrong type"
    );

    let err = result.unwrap_err();
    println!("=== Wrong Type Error ===\n{}", err);
}

/// Test that required field with value that fails validation also fails
///
/// When a required field (no default) has a value that fails validation,
/// deserialization must fail because there's no default to fall back to.
#[test]
fn test_required_field_invalid_value_fails() {
    /// Struct with a required field that has validation but NO default
    #[derive(DeriveConfig, Debug)]
    struct RequiredValidatedConfig {
        /// Required field with range validation - no default!
        #[compote(range(min = 0, max = 100))]
        percentage: i32,
    }

    // Value 150 is present but fails range validation (0-100)
    let json_invalid_value = r#"{"percentage": 150}"#;

    let mut config = Config::default();
    config.load_json(
        json_invalid_value,
        Context::new(Source::Programmatic, Level::User),
    );

    // Deserialization should FAIL because:
    // 1. The value is present (so not a "missing field" error)
    // 2. The value fails validation (150 > 100)
    // 3. There's NO default to fall back to
    let result: Result<RequiredValidatedConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Deserialization should fail when required field fails validation with no default"
    );

    let err = result.unwrap_err();
    println!("=== Required Field Invalid Value Error ===\n{}", err);

    // Error should mention the validation failure
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("percentage")
            || err_str.contains("range")
            || err_str.contains("100")
            || err_str.contains("maximum"),
        "Error should mention the validation failure, got: {}",
        err
    );
}

/// Test that required field with length validation failure also fails
#[test]
fn test_required_field_length_validation_fails() {
    /// Struct with required field that has length validation but NO default
    #[derive(DeriveConfig, Debug)]
    struct RequiredLengthConfig {
        /// Required field - minimum 5 characters, no default
        #[compote(length(min = 5))]
        username: String,
    }

    // "ab" is too short (min 5)
    let json_short_value = r#"{"username": "ab"}"#;

    let mut config = Config::default();
    config.load_json(
        json_short_value,
        Context::new(Source::Programmatic, Level::User),
    );

    // Should fail - value present but fails length validation, no default
    let result: Result<RequiredLengthConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Deserialization should fail when required field fails length validation"
    );

    let err = result.unwrap_err();
    println!("=== Required Field Length Validation Error ===\n{}", err);

    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("username") || err_str.contains("length") || err_str.contains("minimum"),
        "Error should mention length validation, got: {}",
        err
    );
}

/// Test that required field with custom validation failure also fails
#[test]
fn test_required_field_custom_validation_fails() {
    fn validate_positive(value: &i32) -> Result<(), String> {
        if *value > 0 {
            Ok(())
        } else {
            Err("value must be positive".to_string())
        }
    }

    /// Struct with required field that has custom validation but NO default
    #[derive(DeriveConfig, Debug)]
    struct RequiredCustomConfig {
        /// Required field - must be positive, no default
        #[compote(validate = "validate_positive")]
        amount: i32,
    }

    // -5 fails custom validation
    let json_negative = r#"{"amount": -5}"#;

    let mut config = Config::default();
    config.load_json(
        json_negative,
        Context::new(Source::Programmatic, Level::User),
    );

    // Should fail - value present but fails custom validation, no default
    let result: Result<RequiredCustomConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Deserialization should fail when required field fails custom validation"
    );

    let err = result.unwrap_err();
    println!("=== Required Field Custom Validation Error ===\n{}", err);

    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("amount")
            || err_str.contains("validation")
            || err_str.contains("positive"),
        "Error should mention custom validation, got: {}",
        err
    );
}

/// Test multiple errors are accumulated
#[test]
fn test_multiple_errors_accumulated() {
    // Config with multiple type errors
    let config_with_multiple_errors = r#"
name:
  wrong: "type"
count:
  also: "wrong"
status:
  and: "again"
"#;

    let mut loader = compote::loader()
        .load_str(config_with_multiple_errors, Format::Yaml, Level::User)
        .expect("Failed to load config");

    // Deserialization should succeed with all defaults
    let config: ErrorTestConfig = loader
        .deserialize()
        .expect("Deserialization should succeed with defaults");

    // All fields should use defaults
    assert_eq!(config.name, "fallback");
    assert_eq!(config.count, 42);
    assert_eq!(config.status, "valid");

    // Multiple errors should be recorded
    let errors = loader.errors().errors();
    println!("=== Multiple Errors ({}) ===", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}: {}", i + 1, error);
    }

    // Should have errors for multiple fields
    assert!(
        errors.len() >= 2,
        "Expected multiple errors to be recorded, got {}",
        errors.len()
    );
}

/// Test that successful deserialization has no errors
#[test]
fn test_successful_deserialization_no_errors() {
    let valid_config = r#"
name: "valid_name"
count: 100
status: "active"
"#;

    let mut loader = compote::loader()
        .load_str(valid_config, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let config: ErrorTestConfig = loader
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(config.name, "valid_name");
    assert_eq!(config.count, 100);
    assert_eq!(config.status, "active");

    // No errors should be recorded
    assert!(
        !loader.errors().has_errors(),
        "No errors should be recorded for valid config"
    );
    assert!(
        loader.errors().errors().is_empty(),
        "Errors list should be empty"
    );
}

// ============================================================================
// Parse error tracking tests (from integration_test.rs)
// ============================================================================

/// Test that load errors are tracked
#[test]
fn test_error_tracking() {
    let mut config = Config::default();

    // Try to load invalid JSON
    config.load_json(
        "invalid json",
        Context::new(Source::Programmatic, Level::User),
    );

    assert!(config.has_errors());
    let errors = config.get_errors();
    assert!(!errors.is_empty());
}

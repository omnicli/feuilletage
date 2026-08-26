//! Tests for length validation attribute.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

/// Test length min validation failure with default
#[test]
fn test_length_min_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct LengthMinConfig {
        #[feuilletage(length(min = 5), default = "default_value")]
        username: String,
    }

    // "ab" is too short (min 5)
    let config_str = r#"{"username": "ab"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: LengthMinConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.username, "default_value", "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("username") && (msg.contains("length") || msg.contains("minimum"))
        }),
        "Expected length validation error, got: {:?}",
        errors
    );
}

/// Test length max validation failure with default
#[test]
fn test_length_max_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct LengthMaxConfig {
        #[feuilletage(length(max = 5), default = "short")]
        short_text: String,
    }

    // "this is too long" exceeds max 5
    let config_str = r#"{"short_text": "this is too long"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: LengthMaxConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.short_text, "short", "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("short_text") && (msg.contains("length") || msg.contains("maximum"))
        }),
        "Expected length validation error, got: {:?}",
        errors
    );
}

/// Test length validation on required field - fails deserialization
#[test]
fn test_length_required_field_fails_on_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredLengthConfig {
        /// Required field - minimum 5 characters, no default
        #[feuilletage(length(min = 5))]
        username: String,
    }

    // "ab" is too short (min 5)
    let config_str = r#"{"username": "ab"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    // Should fail - value present but fails length validation, no default
    let result: Result<RequiredLengthConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field fails length validation"
    );

    let err = result.unwrap_err();
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("username") || err_str.contains("length") || err_str.contains("minimum"),
        "Error should mention length validation, got: {}",
        err
    );
}

/// Test Vec length validation failure
#[test]
fn test_vec_length_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct VecLengthConfig {
        #[feuilletage(length(min = 2, max = 5), default)]
        items: Vec<String>,
    }

    // Only 1 item when min is 2
    let config_str = r#"{"items": ["only_one"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: VecLengthConfig = config.deserialize().expect("Should succeed with default");

    // Should use default due to length validation failure
    assert!(result.items.is_empty(), "should use default (empty vec)");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("items") && (msg.contains("length") || msg.contains("minimum"))
        }),
        "Expected vec length validation error, got: {:?}",
        errors
    );
}

/// Test Vec too many elements
#[test]
fn test_vec_length_max_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct VecMaxConfig {
        #[feuilletage(length(max = 3), default)]
        items: Vec<String>,
    }

    // 5 items when max is 3
    let config_str = r#"{"items": ["a", "b", "c", "d", "e"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: VecMaxConfig = config.deserialize().expect("Should succeed with default");

    assert!(result.items.is_empty(), "should use default (empty vec)");
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for max violation"
    );
}

/// Test length validation success
#[test]
fn test_length_valid_value_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct LengthConfig {
        #[feuilletage(length(min = 3, max = 10), default = "default")]
        name: String,
    }

    // "hello" is within range (3-10)
    let config_str = r#"{"name": "hello"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: LengthConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, "hello", "name should be hello");
    assert!(
        !config.errors().has_errors(),
        "Should not have errors for valid value"
    );
}

/// Test length at exact boundaries
#[test]
fn test_length_at_boundaries() {
    #[derive(DeriveConfig, Debug)]
    struct BoundaryConfig {
        #[feuilletage(length(min = 3, max = 5), default = "default")]
        value: String,
    }

    // Test at minimum boundary
    let config_str = r#"{"value": "abc"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: BoundaryConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "abc", "length 3 should be valid");
    assert!(!config.errors().has_errors());

    // Test at maximum boundary
    let config_str = r#"{"value": "abcde"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: BoundaryConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "abcde", "length 5 should be valid");
    assert!(!config.errors().has_errors());
}

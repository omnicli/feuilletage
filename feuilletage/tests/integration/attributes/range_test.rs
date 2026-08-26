//! Tests for range validation attribute.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

/// Test range validation failure with default - uses default and records error
#[test]
fn test_range_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct RangeConfig {
        #[feuilletage(range(min = 0, max = 100), default = "50")]
        percentage: i32,
    }

    // 150 is out of range (0-100)
    let config_str = r#"{"percentage": 150}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: RangeConfig = config.deserialize().expect("Should succeed with default");

    // Should use default due to range validation failure
    assert_eq!(result.percentage, 50, "percentage should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("percentage") && (msg.contains("100") || msg.contains("maximum"))
        }),
        "Expected range validation error, got: {:?}",
        errors
    );
}

/// Test range validation failure on required field - fails deserialization
#[test]
fn test_range_required_field_fails_on_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredRangeConfig {
        /// Required field with range validation - no default!
        #[feuilletage(range(min = 0, max = 100))]
        percentage: i32,
    }

    // Value 150 is present but fails range validation (0-100)
    let config_str = r#"{"percentage": 150}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    // Should fail because validation fails and there's no default
    let result: Result<RequiredRangeConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field fails validation with no default"
    );

    let err = result.unwrap_err();
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("percentage") || err_str.contains("range") || err_str.contains("100"),
        "Error should mention the validation failure, got: {}",
        err
    );
}

/// Test range min boundary
#[test]
fn test_range_min_boundary() {
    #[derive(DeriveConfig, Debug)]
    struct MinRangeConfig {
        #[feuilletage(range(min = 10), default = "50")]
        value: i32,
    }

    // 5 is below minimum of 10
    let config_str = r#"{"value": 5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MinRangeConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 50, "value should use default");
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for min violation"
    );
}

/// Test range max boundary
#[test]
fn test_range_max_boundary() {
    #[derive(DeriveConfig, Debug)]
    struct MaxRangeConfig {
        #[feuilletage(range(max = 100), default = "50")]
        value: i32,
    }

    // 150 is above maximum of 100
    let config_str = r#"{"value": 150}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MaxRangeConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 50, "value should use default");
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for max violation"
    );
}

/// Test range validation success - value within range
#[test]
fn test_range_valid_value_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct RangeConfig {
        #[feuilletage(range(min = 0, max = 100), default = "50")]
        percentage: i32,
    }

    // 75 is within range
    let config_str = r#"{"percentage": 75}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: RangeConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.percentage, 75, "percentage should be 75");
    assert!(
        !config.errors().has_errors(),
        "Should not have errors for valid value"
    );
}

/// Test range validation with float
#[test]
fn test_range_with_float() {
    #[derive(DeriveConfig, Debug)]
    struct FloatRangeConfig {
        #[feuilletage(range(min = 0.0, max = 1.0), default = "0.5")]
        ratio: f64,
    }

    // 1.5 is above maximum of 1.0
    let config_str = r#"{"ratio": 1.5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: FloatRangeConfig = config.deserialize().expect("Should succeed with default");

    assert!(
        (result.ratio - 0.5).abs() < f64::EPSILON,
        "ratio should use default 0.5"
    );
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for max violation"
    );
}

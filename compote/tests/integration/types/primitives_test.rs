//! Tests for primitive type deserialization.
//!
//! Tests error handling for bool, int, float, and string deserialization.

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Test bool parsing failure with default
#[test]
fn test_bool_parsing_error_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct BoolConfig {
        #[compote(default = "false")]
        enabled: bool,

        #[compote(default = "test")]
        name: String,
    }

    // "invalid" is not a valid bool string
    let config_str = r#"{"enabled": "invalid_bool_value", "name": "ok"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BoolConfig = config.deserialize().expect("Should succeed with default");

    assert!(!result.enabled, "enabled should use default (false)");
    assert_eq!(result.name, "ok");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("enabled") || msg.contains("bool") || msg.contains("parse")
        }),
        "Expected bool parse error, got: {:?}",
        errors
    );
}

/// Test integer overflow with default
#[test]
fn test_integer_overflow_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct SmallIntConfig {
        #[compote(default = "0")]
        small_num: i8, // i8 range: -128 to 127
    }

    // 999 is out of range for i8
    let config_str = r#"{"small_num": 999}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: SmallIntConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.small_num, 0, "small_num should use default (0)");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("small_num") || msg.contains("range") || msg.contains("i8")
        }),
        "Expected overflow error, got: {:?}",
        errors
    );
}

/// Test unsigned integer with negative value
#[test]
fn test_unsigned_negative_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct UnsignedConfig {
        #[compote(default = "0")]
        positive_only: u32,
    }

    // -5 is invalid for u32
    let config_str = r#"{"positive_only": -5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: UnsignedConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.positive_only, 0, "positive_only should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("positive_only")
                || msg.contains("negative")
                || msg.contains("u32")
                || msg.contains("u64")
        }),
        "Expected unsigned error, got: {:?}",
        errors
    );
}

/// Test float parsing failure with default
#[test]
fn test_float_parsing_error_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct FloatConfig {
        #[compote(default = "0.0")]
        value: f64,
    }

    // "not_a_float" cannot be parsed as f64
    let config_str = r#"{"value": "not_a_float"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: FloatConfig = config.deserialize().expect("Should succeed with default");

    assert!(
        (result.value - 0.0).abs() < f64::EPSILON,
        "value should use default"
    );

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("value")
                || msg.contains("f64")
                || msg.contains("parse")
                || msg.contains("float")
        }),
        "Expected float parse error, got: {:?}",
        errors
    );
}

/// Test valid bool parsing
#[test]
fn test_bool_valid_values() {
    #[derive(DeriveConfig, Debug)]
    struct BoolConfig {
        #[compote(default = "false")]
        flag1: bool,

        #[compote(default = "false")]
        flag2: bool,
    }

    // Test with actual boolean values
    let config_str = r#"{"flag1": true, "flag2": false}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BoolConfig = config.deserialize().expect("Should succeed");

    assert!(result.flag1);
    assert!(!result.flag2);
    assert!(!config.errors().has_errors());
}

/// Test valid integer parsing
#[test]
fn test_integer_valid_values() {
    #[derive(DeriveConfig, Debug)]
    struct IntConfig {
        #[compote(default = "0")]
        i32_val: i32,

        #[compote(default = "0")]
        i64_val: i64,

        #[compote(default = "0")]
        u32_val: u32,

        #[compote(default = "0")]
        u64_val: u64,
    }

    let config_str = r#"{
        "i32_val": -42,
        "i64_val": 9223372036854775807,
        "u32_val": 4294967295,
        "u64_val": 18446744073709551615
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: IntConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.i32_val, -42);
    assert_eq!(result.i64_val, i64::MAX);
    assert_eq!(result.u32_val, u32::MAX);
    assert_eq!(result.u64_val, u64::MAX);
    assert!(!config.errors().has_errors());
}

/// Test valid float parsing
#[test]
fn test_float_valid_values() {
    #[derive(DeriveConfig, Debug)]
    struct FloatConfig {
        #[compote(default = "0.0")]
        f32_val: f32,

        #[compote(default = "0.0")]
        f64_val: f64,
    }

    let config_str = r#"{"f32_val": 3.14, "f64_val": 2.718281828}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: FloatConfig = config.deserialize().expect("Should succeed");

    assert!((result.f32_val - 3.14).abs() < 0.001);
    assert!((result.f64_val - 2.718281828).abs() < 0.0001);
    assert!(!config.errors().has_errors());
}

/// Test required primitive field fails when missing
#[test]
fn test_required_primitive_fails_when_missing() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredPrimitiveConfig {
        required_int: i32,
    }

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredPrimitiveConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field is missing"
    );
}

/// Test required primitive field fails when wrong type
#[test]
fn test_required_primitive_fails_when_wrong_type() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredPrimitiveConfig {
        required_int: i32,
    }

    let config_str = r#"{"required_int": "not_a_number"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredPrimitiveConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field has wrong type"
    );
}

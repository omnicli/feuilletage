//! Tests for coerce attribute.
//!
//! The coerce attribute enables liberal type coercion, allowing string values
//! to be parsed into their target types.
//!
//! Includes both explicit coerce tests and implicit type coercion tests.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Explicit coerce attribute tests
// ============================================================================

/// Test successful coercion from string to int
#[test]
fn test_coerce_string_to_int_success() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceConfig {
        #[feuilletage(coerce, default = "0")]
        count: i32,
    }

    // String "123" with coerce should convert to int
    let config_str = r#"{"count": "123"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.count, 123, "coerce should convert string to int");
    assert!(
        config.errors().errors().is_empty(),
        "No errors expected for valid coercion"
    );
}

/// Test coercion failure when string can't be parsed
#[test]
fn test_coerce_unparseable_string_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceFailConfig {
        #[feuilletage(coerce, default = "0")]
        count: i32,
    }

    // "abc" can't be coerced to int
    let config_str = r#"{"count": "abc"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceFailConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.count, 0, "should use default on coerce failure");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("count") || msg.contains("type") || msg.contains("mismatch")
        }),
        "Expected coerce failure error, got: {:?}",
        errors
    );
}

/// Test coerce string to bool
#[test]
fn test_coerce_string_to_bool() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceBoolConfig {
        #[feuilletage(coerce, default = "false")]
        enabled: bool,
    }

    // "true" as string should coerce to bool
    let config_str = r#"{"enabled": "true"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceBoolConfig = config.deserialize().expect("Should succeed");

    assert!(
        result.enabled,
        "coerce should convert 'true' string to bool"
    );
    assert!(!config.errors().has_errors());
}

/// Test coerce string to float
#[test]
fn test_coerce_string_to_float() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceFloatConfig {
        #[feuilletage(coerce, default = "0.0")]
        ratio: f64,
    }

    // "3.14" as string should coerce to float
    let config_str = r#"{"ratio": "3.14"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceFloatConfig = config.deserialize().expect("Should succeed");

    assert!(
        (result.ratio - 3.14).abs() < 0.001,
        "coerce should convert string to float"
    );
    assert!(!config.errors().has_errors());
}

/// Test coerce int to string (reverse coercion)
#[test]
fn test_coerce_int_to_string() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceToStringConfig {
        #[feuilletage(coerce, default = "default")]
        value: String,
    }

    // 123 as int should coerce to string
    let config_str = r#"{"value": 123}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceToStringConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.value, "123", "coerce should convert int to string");
    assert!(!config.errors().has_errors());
}

/// Test coerce bool to string
#[test]
fn test_coerce_bool_to_string() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceBoolToStringConfig {
        #[feuilletage(coerce, default = "default")]
        flag: String,
    }

    // true as bool should coerce to string
    let config_str = r#"{"flag": true}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceBoolToStringConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.flag, "true", "coerce should convert bool to string");
    assert!(!config.errors().has_errors());
}

/// Test coerce on required field fails when coercion impossible
#[test]
fn test_coerce_required_field_fails_on_impossible_coercion() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredCoerceConfig {
        #[feuilletage(coerce)]
        count: i32,
    }

    // Object can't be coerced to int
    let config_str = r#"{"count": {"nested": "value"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredCoerceConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field can't be coerced"
    );
}

/// Test coerce with negative numbers
#[test]
fn test_coerce_negative_string_to_int() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceNegativeConfig {
        #[feuilletage(coerce, default = "0")]
        value: i32,
    }

    // "-42" as string should coerce to negative int
    let config_str = r#"{"value": "-42"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceNegativeConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.value, -42, "coerce should handle negative numbers");
    assert!(!config.errors().has_errors());
}

/// Test coerce "yes"/"no" strings to bool
#[test]
fn test_coerce_bool_from_string_yes() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceBoolYesConfig {
        #[feuilletage(coerce)]
        enabled: bool,
    }

    let config_str = r#"{"enabled": "yes"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceBoolYesConfig = config.deserialize().expect("Should succeed");

    assert!(result.enabled, "coerce should convert 'yes' to true");
}

#[test]
fn test_coerce_bool_from_string_no() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceBoolNoConfig {
        #[feuilletage(coerce)]
        enabled: bool,
    }

    let config_str = r#"{"enabled": "no"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceBoolNoConfig = config.deserialize().expect("Should succeed");

    assert!(!result.enabled, "coerce should convert 'no' to false");
}

/// Test coerce int to bool (1 = true, 0 = false)
#[test]
fn test_coerce_bool_from_int() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceBoolFromIntConfig {
        #[feuilletage(coerce)]
        enabled: bool,
    }

    let config_str = r#"{"enabled": 1}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceBoolFromIntConfig = config.deserialize().expect("Should succeed");

    assert!(result.enabled, "coerce should convert 1 to true");
}

/// Test coerce string to u64
#[test]
fn test_coerce_u64_from_string() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceU64Config {
        #[feuilletage(coerce)]
        value: u64,
    }

    let config_str = r#"{"value": "999"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceU64Config = config.deserialize().expect("Should succeed");

    assert_eq!(result.value, 999, "coerce should convert string to u64");
}

/// Test coerce with multiple fields (integration style)
#[test]
fn test_coerce_multiple_fields() {
    #[derive(DeriveConfig, Debug)]
    struct CoerceMultiConfig {
        #[feuilletage(coerce)]
        string_field: String,

        #[feuilletage(coerce)]
        bool_field: bool,

        #[feuilletage(coerce)]
        int_field: i64,

        #[feuilletage(coerce)]
        float_field: f64,
    }

    // Test coercing all at once: bool->string, int->bool, string->int, int->float
    let config_str =
        r#"{"string_field": true, "bool_field": 1, "int_field": "42", "float_field": 100}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CoerceMultiConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.string_field, "true");
    assert!(result.bool_field);
    assert_eq!(result.int_field, 42);
    assert_eq!(result.float_field, 100.0);
}

// ============================================================================
// Implicit type coercion tests (from integration_test.rs)
// ============================================================================

/// Test that type coercion happens automatically for certain type conversions
#[test]
fn test_type_coercion() {
    let json = r#"{
        "string_from_int": 42,
        "bool_from_string": "true",
        "int_from_string": "123",
        "float_from_int": 42
    }"#;

    #[derive(Debug, DeriveConfig)]
    struct CoercionTest {
        string_from_int: String,
        bool_from_string: bool,
        int_from_string: i64,
        float_from_int: f64,
    }

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<CoercionTest>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.string_from_int, "42");
    assert!(cfg.bool_from_string);
    assert_eq!(cfg.int_from_string, 123);
    assert_eq!(cfg.float_from_int, 42.0);
}

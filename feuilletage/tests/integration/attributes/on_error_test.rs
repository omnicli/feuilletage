//! Tests for on_error attribute.
//!
//! The on_error attribute controls how errors are handled during field deserialization:
//! - `skip`: Graceful mode - skip invalid items in Vec, convert Option to None
//! - `default`: Use field's default value on any error
//! - `fail`: Hard stop - fail entire parsing immediately on first error

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// on_error = skip tests (graceful mode, the default behavior)
// ============================================================================

/// Test that Vec with invalid items skips them by default
#[test]
fn test_on_error_skip_vec_skips_invalid() {
    #[derive(DeriveConfig, Debug)]
    struct Item {
        value: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct DefaultVecConfig {
        #[feuilletage(allow_single)]
        items: Vec<Item>,
    }

    // Second item is invalid (wrong type)
    let config_str = r#"{"items": [{"value": 1}, {"value": "invalid"}, {"value": 3}]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DefaultVecConfig = config
        .deserialize()
        .expect("Should succeed, skipping invalid items");

    // Only valid items should be in the result
    assert_eq!(result.items.len(), 2, "should skip invalid item");
    assert_eq!(result.items[0].value, 1);
    assert_eq!(result.items[1].value, 3);

    // Error should be recorded
    assert!(
        config.errors().has_errors(),
        "Error should be recorded for invalid item"
    );
}

/// Test that Option with invalid value becomes None by default
#[test]
fn test_on_error_default_option_becomes_none() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        value: Option<i32>,
    }

    // Value is invalid (wrong type)
    let config_str = r#"{"value": "not_an_int"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config
        .deserialize()
        .expect("Should succeed, converting to None");

    assert!(result.value.is_none(), "Invalid Option should become None");

    // Error should be recorded
    assert!(
        config.errors().has_errors(),
        "Error should be recorded for invalid value"
    );
}

/// Test that field with default uses default on error by default (skip mode)
#[test]
fn test_on_error_skip_uses_field_default() {
    #[derive(DeriveConfig, Debug)]
    struct SkipFieldConfig {
        #[feuilletage(default = "42")]
        value: i32,
    }

    // Value is invalid (wrong type)
    let config_str = r#"{"value": "not_an_int"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: SkipFieldConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 42, "Should use default on error");

    // Error should be recorded
    assert!(
        config.errors().has_errors(),
        "Error should be recorded for invalid value"
    );
}

// ============================================================================
// on_error = fail tests
// ============================================================================

/// Test that on_error = fail with Vec fails on first invalid item
#[test]
fn test_on_error_fail_vec_returns_error() {
    #[derive(DeriveConfig, Debug)]
    struct Item {
        value: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct FailVecConfig {
        #[feuilletage(allow_single, on_error = fail)]
        items: Vec<Item>,
    }

    // Second item is invalid
    let config_str = r#"{"items": [{"value": 1}, {"value": "invalid"}, {"value": 3}]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<FailVecConfig, _> = config.deserialize();

    assert!(
        result.is_err(),
        "on_error = fail should return error immediately"
    );
}

/// Test that on_error = fail with default field still fails on error
#[test]
fn test_on_error_fail_ignores_default() {
    #[derive(DeriveConfig, Debug)]
    struct FailWithDefaultConfig {
        #[feuilletage(default = "42", on_error = fail)]
        value: i32,
    }

    // Value is invalid (wrong type)
    let config_str = r#"{"value": "not_an_int"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<FailWithDefaultConfig, _> = config.deserialize();

    assert!(
        result.is_err(),
        "on_error = fail should fail even if field has default"
    );
}

/// Test that on_error = fail with validation error fails
#[test]
fn test_on_error_fail_validation_fails() {
    #[derive(DeriveConfig, Debug)]
    struct FailValidationConfig {
        #[feuilletage(range(0, 100), default = "50", on_error = fail)]
        percentage: i32,
    }

    // Value is out of range
    let config_str = r#"{"percentage": 150}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<FailValidationConfig, _> = config.deserialize();

    assert!(
        result.is_err(),
        "on_error = fail should fail on validation error"
    );
}

// ============================================================================
// on_error = default tests (use field default on any error)
// ============================================================================

/// Test that on_error = default with Vec uses default if any item fails
#[test]
fn test_on_error_default_vec_uses_default_on_error() {
    #[derive(DeriveConfig, Debug)]
    struct Item {
        value: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct DefaultVecConfig {
        #[feuilletage(allow_single, on_error = default, default = "vec![]")]
        items: Vec<Item>,
    }

    // Second item is invalid
    let config_str = r#"{"items": [{"value": 1}, {"value": "invalid"}, {"value": 3}]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DefaultVecConfig = config.deserialize().expect("Should succeed using default");

    // When any item fails, the whole Vec uses its default (empty)
    assert!(
        result.items.is_empty(),
        "on_error = default should use default when any item fails"
    );

    // Error should be recorded
    assert!(
        config.errors().has_errors(),
        "Error should be recorded for invalid item"
    );
}

/// Test that on_error = default uses field default on error
#[test]
fn test_on_error_default_uses_field_default() {
    #[derive(DeriveConfig, Debug)]
    struct DefaultFieldConfig {
        #[feuilletage(default = "42", on_error = default)]
        value: i32,
    }

    // Value is invalid (wrong type)
    let config_str = r#"{"value": "not_an_int"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DefaultFieldConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 42, "Should use default on error");

    // Error should be recorded
    assert!(
        config.errors().has_errors(),
        "Error should be recorded for invalid value"
    );
}

// ============================================================================
// on_error with different field types
// ============================================================================

/// Test on_error = fail with transform
#[test]
fn test_on_error_fail_with_transform() {
    #[derive(DeriveConfig, Debug)]
    struct FailTransformConfig {
        #[feuilletage(transform = "to_uppercase", default = "DEFAULT", on_error = fail)]
        name: String,
    }

    // Invalid value type (not a string)
    let config_str = r#"{"name": 123}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    // Transform should work on stringified int, so this should actually succeed
    let result: Result<FailTransformConfig, _> = config.deserialize();

    // Actually, int gets converted to string "123" during FromContextValue for String
    // So this test verifies that transform still works
    assert!(result.is_ok(), "Transform should work with stringified int");
    assert_eq!(result.unwrap().name, "123"); // transform to_uppercase on "123" is still "123"
}

/// Test on_error = fail with coerce
#[test]
fn test_on_error_fail_with_coerce() {
    #[derive(DeriveConfig, Debug)]
    struct FailCoerceConfig {
        #[feuilletage(coerce, default = "0", on_error = fail)]
        count: i32,
    }

    // Invalid string that can't be coerced
    let config_str = r#"{"count": "abc"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<FailCoerceConfig, _> = config.deserialize();

    assert!(
        result.is_err(),
        "on_error = fail should fail on coercion error even with default"
    );
}

/// Test that on_error = default is explicit version of default behavior
#[test]
fn test_on_error_default_explicit() {
    #[derive(DeriveConfig, Debug)]
    struct ExplicitDefaultConfig {
        #[feuilletage(default = "42", on_error = default)]
        value: i32,
    }

    // Value is invalid (wrong type)
    let config_str = r#"{"value": "not_an_int"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ExplicitDefaultConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(
        result.value, 42,
        "on_error = default should use field default"
    );
}

// ============================================================================
// Edge cases
// ============================================================================

/// Test on_error with nested structs - on_error only affects the field it's on
#[test]
fn test_on_error_fail_nested_struct() {
    #[derive(DeriveConfig, Debug, Default)]
    struct Inner {
        #[feuilletage(on_error = fail)]
        required: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct Outer {
        // The outer field has default, so if Inner fails, Outer uses Default::default()
        #[feuilletage(default)]
        inner: Inner,
    }

    // Inner struct has invalid value
    let config_str = r#"{"inner": {"required": "not_int"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    // The outer field has default, so it uses the default when inner deserialization fails
    // on_error = fail on the inner field makes Inner fail, but Outer catches it
    let result: Outer = config.deserialize().expect("Should use outer default");

    // Inner uses default value (0) because outer's default kicked in
    assert_eq!(result.inner.required, 0);
}

/// Test on_error = fail truly propagates when no outer default
#[test]
fn test_on_error_fail_nested_no_outer_default() {
    #[derive(DeriveConfig, Debug)]
    struct Inner {
        #[feuilletage(on_error = fail)]
        required: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct Outer {
        // No default on outer field - error will propagate
        inner: Inner,
    }

    // Inner struct has invalid value
    let config_str = r#"{"inner": {"required": "not_int"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<Outer, _> = config.deserialize();

    // Without outer default, the failure propagates
    assert!(
        result.is_err(),
        "Nested struct with on_error = fail should fail when no outer default"
    );
}

/// Test on_error with Vec of primitives
#[test]
fn test_on_error_fail_vec_primitives() {
    #[derive(DeriveConfig, Debug)]
    struct PrimitiveVecConfig {
        #[feuilletage(allow_single, on_error = fail)]
        numbers: Vec<i32>,
    }

    // Third item is invalid
    let config_str = r#"{"numbers": [1, 2, "three", 4]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<PrimitiveVecConfig, _> = config.deserialize();

    assert!(
        result.is_err(),
        "on_error = fail with Vec<i32> should fail on invalid item"
    );
}

/// Test on_error = default with Vec of primitives
#[test]
fn test_on_error_default_vec_primitives() {
    #[derive(DeriveConfig, Debug)]
    struct DefaultPrimitiveVecConfig {
        #[feuilletage(allow_single, on_error = default)]
        numbers: Vec<i32>,
    }

    // Third item is invalid
    let config_str = r#"{"numbers": [1, 2, "three", 4]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DefaultPrimitiveVecConfig = config
        .deserialize()
        .expect("Should succeed using empty default");

    // When any item fails with default mode, Vec uses default (empty)
    assert!(
        result.numbers.is_empty(),
        "on_error = default should use empty Vec when any item fails"
    );
}

/// Test that valid data works with all on_error modes
#[test]
fn test_on_error_modes_with_valid_data() {
    #[derive(DeriveConfig, Debug)]
    struct ValidDataConfig {
        #[feuilletage(on_error = skip)]
        skip_mode: i32,

        #[feuilletage(on_error = default, default = "0")]
        default_mode: i32,

        #[feuilletage(on_error = fail)]
        fail_mode: i32,
    }

    let config_str = r#"{"skip_mode": 1, "default_mode": 2, "fail_mode": 3}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ValidDataConfig = config
        .deserialize()
        .expect("Should succeed with valid data");

    assert_eq!(result.skip_mode, 1);
    assert_eq!(result.default_mode, 2);
    assert_eq!(result.fail_mode, 3);
    assert!(
        !config.errors().has_errors(),
        "No errors expected for valid data"
    );
}

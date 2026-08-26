//! Tests for type mismatch error handling.
//!
//! Tests when an object or array is provided instead of a primitive type.

use compote::{Config, Context, Format, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Test object provided when primitive expected
#[test]
fn test_object_instead_of_primitive_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct PrimitiveConfig {
        #[compote(default = "default_string")]
        text: String,
    }

    // Providing object instead of string
    let config_str = r#"
text:
  nested: "object"
  another: "field"
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: PrimitiveConfig = loader.deserialize().expect("Should succeed with default");

    assert_eq!(result.text, "default_string", "text should use default");

    let errors = loader.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("text")
                || msg.contains("type")
                || msg.contains("mismatch")
                || msg.contains("string")
        }),
        "Expected type mismatch error, got: {:?}",
        errors
    );
}

/// Test array provided when primitive expected
#[test]
fn test_array_instead_of_primitive_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct PrimitiveConfig {
        #[compote(default = "100")]
        count: i32,
    }

    // Providing array instead of int
    let config_str = r#"{"count": [1, 2, 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: PrimitiveConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.count, 100, "count should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("count")
                || msg.contains("type")
                || msg.contains("mismatch")
                || msg.contains("array")
        }),
        "Expected type mismatch error, got: {:?}",
        errors
    );
}

/// Test object instead of integer
#[test]
fn test_object_instead_of_int_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct IntConfig {
        #[compote(default = "42")]
        value: i32,
    }

    let config_str = r#"{"value": {"nested": "object"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: IntConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 42, "value should use default");
    assert!(config.errors().has_errors());
}

/// Test array instead of string
#[test]
fn test_array_instead_of_string_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct StringConfig {
        #[compote(default = "default")]
        name: String,
    }

    let config_str = r#"{"name": ["array", "of", "strings"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: StringConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.name, "default", "name should use default");
    assert!(config.errors().has_errors());
}

/// Test object instead of bool
#[test]
fn test_object_instead_of_bool_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct BoolConfig {
        #[compote(default = "false")]
        enabled: bool,
    }

    let config_str = r#"{"enabled": {"key": "value"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BoolConfig = config.deserialize().expect("Should succeed with default");

    assert!(!result.enabled, "enabled should use default (false)");
    assert!(config.errors().has_errors());
}

/// Test array instead of bool
#[test]
fn test_array_instead_of_bool_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct BoolConfig {
        #[compote(default = "true")]
        enabled: bool,
    }

    let config_str = r#"{"enabled": [true, false]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BoolConfig = config.deserialize().expect("Should succeed with default");

    assert!(result.enabled, "enabled should use default (true)");
    assert!(config.errors().has_errors());
}

/// Test required field fails when type mismatch
#[test]
fn test_required_field_fails_on_type_mismatch() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredConfig {
        // No default - required field
        value: i32,
    }

    // Providing object instead of int
    let config_str = r#"{"value": {"nested": "object"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field has type mismatch"
    );
}

/// Test deeply nested wrong type
#[test]
fn test_deeply_nested_type_mismatch() {
    #[derive(DeriveConfig, Debug)]
    struct DeepConfig {
        #[compote(default = "default")]
        level1: String,
    }

    // Providing deeply nested structure instead of string
    let config_str = r#"
level1:
  nested1:
    nested2:
      nested3: "deep value"
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: DeepConfig = loader.deserialize().expect("Should succeed with default");

    assert_eq!(result.level1, "default", "level1 should use default");
    assert!(loader.errors().has_errors());
}

//! Tests for the transform_after and transform_each_after attributes.
//!
//! These attributes allow post-deserialization transformation:
//! - `transform_after`: Transform the deserialized value (operates on Rust types)
//! - `transform_each_after`: Transform each element in collections after deserialization

#![cfg(feature = "json")]

use feuilletage::{Context, Error, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Test helper transform functions
// ============================================================================

/// Transform function that uppercases a String
fn uppercase_string(value: &mut String) -> Result<(), Error> {
    *value = value.to_uppercase();
    Ok(())
}

/// Transform function that trims whitespace from a String
fn trim_string(value: &mut String) -> Result<(), Error> {
    *value = value.trim().to_string();
    Ok(())
}

/// Transform function that doubles an integer
fn double_int(value: &mut i32) -> Result<(), Error> {
    *value *= 2;
    Ok(())
}

/// Transform function that negates a bool
fn negate_bool(value: &mut bool) -> Result<(), Error> {
    *value = !*value;
    Ok(())
}

/// Transform function that fails if string is empty
fn reject_empty(value: &mut String) -> Result<(), Error> {
    if value.is_empty() {
        return Err(Error::InvalidValue {
            path: "".to_string(),
            message: "Value cannot be empty".to_string(),
        });
    }
    Ok(())
}

/// Transform function that adds a prefix to a String
fn add_prefix(value: &mut String) -> Result<(), Error> {
    *value = format!("PREFIX_{}", value);
    Ok(())
}

// ============================================================================
// Basic transform_after tests
// ============================================================================

#[test]
fn test_transform_after_basic_string() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_string")]
        name: String,
    }

    let json = r#"{"name": "hello world"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, "HELLO WORLD");
}

#[test]
fn test_transform_after_trim_string() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "trim_string")]
        value: String,
    }

    let json = r#"{"value": "  spaced  "}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "spaced");
}

#[test]
fn test_transform_after_integer() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "double_int")]
        count: i32,
    }

    let json = r#"{"count": 21}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.count, 42);
}

#[test]
fn test_transform_after_bool() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "negate_bool")]
        flag: bool,
    }

    let json = r#"{"flag": true}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert!(!result.flag);
}

// ============================================================================
// transform_after with default values
// ============================================================================

#[test]
fn test_transform_after_with_default_missing_field() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_string", default = "default_value")]
        name: String,
    }

    let json = r#"{}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    // Default is used without transformation (transform only applies to loaded values)
    assert_eq!(result.name, "default_value");
}

#[test]
fn test_transform_after_with_default_provided_field() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_string", default = "default_value")]
        name: String,
    }

    let json = r#"{"name": "provided"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    // Provided value is transformed
    assert_eq!(result.name, "PROVIDED");
}

// ============================================================================
// transform_after error handling
// ============================================================================

#[test]
fn test_transform_after_error_fails_deserialization() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "reject_empty")]
        name: String,
    }

    let json = r#"{"name": ""}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<Config, _> = config.deserialize();
    assert!(result.is_err());
}

#[test]
fn test_transform_after_error_with_default_fallback() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "reject_empty", default = "fallback")]
        name: String,
    }

    let json = r#"{"name": ""}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed with fallback");
    assert_eq!(result.name, "fallback");
}

#[test]
fn test_transform_after_error_on_error_fail() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(
            transform_after = "reject_empty",
            default = "fallback",
            on_error = "fail"
        )]
        name: String,
    }

    let json = r#"{"name": ""}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<Config, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail even with default when on_error = fail"
    );
}

// ============================================================================
// transform_after combined with other attributes
// ============================================================================

#[test]
fn test_transform_after_with_transform() {
    // transform applies to ConfigValue (before deserialization)
    // transform_after applies to the Rust value (after deserialization)
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform = "trim", transform_after = "uppercase_string")]
        name: String,
    }

    let json = r#"{"name": "  hello  "}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    // First trimmed by transform, then uppercased by transform_after
    assert_eq!(result.name, "HELLO");
}

#[test]
fn test_transform_after_with_validation() {
    fn validate_length(value: &String) -> Result<(), String> {
        if value.len() < 3 {
            return Err("Too short".to_string());
        }
        Ok(())
    }

    #[derive(DeriveConfig, Debug)]
    struct Config {
        // transform_after runs before validation
        #[feuilletage(transform_after = "add_prefix", validate = "validate_length")]
        name: String,
    }

    let json = r#"{"name": "hi"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    // "hi" -> "PREFIX_hi" (length > 3, passes validation)
    assert_eq!(result.name, "PREFIX_hi");
}

#[test]
fn test_transform_after_with_coerce() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(coerce, transform_after = "double_int")]
        count: i32,
    }

    // String "10" is coerced to i32 10, then doubled to 20
    let json = r#"{"count": "10"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.count, 20);
}

// ============================================================================
// transform_each_after tests for Vec fields
// ============================================================================

#[test]
fn test_transform_each_after_basic() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "uppercase_string")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["hello", "world"]}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.items, vec!["HELLO", "WORLD"]);
}

#[test]
fn test_transform_each_after_single_value() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "uppercase_string")]
        items: Vec<String>,
    }

    let json = r#"{"items": "single"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.items, vec!["SINGLE"]);
}

#[test]
fn test_transform_each_after_with_transform_each() {
    // transform_each operates on ConfigValue before deserialization
    // transform_each_after operates on deserialized Rust values
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(
            allow_single,
            transform_each = "trim",
            transform_each_after = "uppercase_string"
        )]
        items: Vec<String>,
    }

    let json = r#"{"items": ["  hello  ", "  world  "]}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    // First trimmed, then uppercased
    assert_eq!(result.items, vec!["HELLO", "WORLD"]);
}

#[test]
fn test_transform_each_after_empty_vec() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "uppercase_string")]
        items: Vec<String>,
    }

    let json = r#"{"items": []}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert!(result.items.is_empty());
}

#[test]
fn test_transform_each_after_integers() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "double_int")]
        numbers: Vec<i32>,
    }

    let json = r#"{"numbers": [1, 2, 3, 4, 5]}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.numbers, vec![2, 4, 6, 8, 10]);
}

#[test]
fn test_transform_each_after_error_skips_element() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "reject_empty")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["valid", "", "also_valid"]}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config
        .deserialize()
        .expect("Should succeed with skipped items");
    // Empty string is skipped due to transform_each_after error
    assert_eq!(result.items, vec!["valid", "also_valid"]);
}

#[test]
fn test_transform_each_after_error_on_error_fail() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(allow_single, transform_each_after = "reject_empty", on_error = "fail")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["valid", "", "also_valid"]}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<Config, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail on empty element when on_error = fail"
    );
}

// ============================================================================
// Multiple fields with different transforms
// ============================================================================

#[test]
fn test_multiple_transform_after_fields() {
    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_string")]
        upper: String,

        #[feuilletage(transform_after = "trim_string")]
        trimmed: String,

        #[feuilletage(transform_after = "double_int")]
        doubled: i32,
    }

    let json = r#"{
        "upper": "make me upper",
        "trimmed": "  trim me  ",
        "doubled": 21
    }"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.upper, "MAKE ME UPPER");
    assert_eq!(result.trimmed, "trim me");
    assert_eq!(result.doubled, 42);
}

// ============================================================================
// transform_after with Option<T>
// ============================================================================

#[test]
fn test_transform_after_option_some() {
    fn uppercase_option(value: &mut Option<String>) -> Result<(), Error> {
        if let Some(s) = value {
            *s = s.to_uppercase();
        }
        Ok(())
    }

    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_option")]
        name: Option<String>,
    }

    let json = r#"{"name": "hello"}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, Some("HELLO".to_string()));
}

#[test]
fn test_transform_after_option_none() {
    fn uppercase_option(value: &mut Option<String>) -> Result<(), Error> {
        if let Some(s) = value {
            *s = s.to_uppercase();
        }
        Ok(())
    }

    #[derive(DeriveConfig, Debug)]
    struct Config {
        #[feuilletage(transform_after = "uppercase_option")]
        name: Option<String>,
    }

    let json = r#"{}"#;
    let mut config = feuilletage::Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Config = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, None);
}

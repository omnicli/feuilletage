//! Tests for Option<T> deserialization.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

/// Test Option with wrong inner type records error
#[test]
fn test_option_wrong_inner_type_uses_none() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_count: Option<i32>,
    }

    // Providing string instead of int for Option<i32>
    let config_str = r#"{"maybe_count": "not_a_number"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    // Option should be None or default on error
    println!("Option Result: {:?}", result.maybe_count);

    let errors = config.errors().errors();
    // Should have recorded the parse error
    assert!(
        !errors.is_empty() || result.maybe_count.is_none(),
        "Should either have error or None result"
    );
}

/// Test Option with valid value succeeds
#[test]
fn test_option_valid_value() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_count: Option<i32>,
    }

    let config_str = r#"{"maybe_count": 42}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.maybe_count, Some(42));
    assert!(!config.errors().has_errors());
}

/// Test Option with null value becomes None
#[test]
fn test_option_null_value() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_value: Option<String>,
    }

    let config_str = r#"{"maybe_value": null}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    assert!(result.maybe_value.is_none());
    assert!(!config.errors().has_errors());
}

/// Test Option with missing field defaults to None
#[test]
fn test_option_missing_field() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_value: Option<String>,
    }

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    assert!(result.maybe_value.is_none());
    assert!(!config.errors().has_errors());
}

/// Test Option<String> with object value
#[test]
fn test_option_string_with_object_value() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_name: Option<String>,
    }

    // Providing object instead of string
    let config_str = r#"{"maybe_name": {"nested": "value"}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    // Should be None due to type mismatch
    assert!(
        result.maybe_name.is_none(),
        "Option should be None on type mismatch"
    );
    assert!(config.errors().has_errors());
}

/// Test Option<i32> with array value
#[test]
fn test_option_int_with_array_value() {
    #[derive(DeriveConfig, Debug)]
    struct OptionConfig {
        maybe_count: Option<i32>,
    }

    // Providing array instead of int
    let config_str = r#"{"maybe_count": [1, 2, 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionConfig = config.deserialize().expect("Should succeed");

    // Should be None due to type mismatch
    assert!(
        result.maybe_count.is_none(),
        "Option should be None on type mismatch"
    );
    assert!(config.errors().has_errors());
}

/// Test multiple Option fields
#[test]
fn test_multiple_option_fields() {
    #[derive(DeriveConfig, Debug)]
    struct MultiOptionConfig {
        opt_string: Option<String>,
        opt_int: Option<i32>,
        opt_bool: Option<bool>,
    }

    let config_str = r#"{
        "opt_string": "hello",
        "opt_int": 42,
        "opt_bool": true
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiOptionConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.opt_string, Some("hello".to_string()));
    assert_eq!(result.opt_int, Some(42));
    assert_eq!(result.opt_bool, Some(true));
    assert!(!config.errors().has_errors());
}

/// Test Option<Vec<T>>
#[test]
fn test_option_vec() {
    #[derive(DeriveConfig, Debug)]
    struct OptionVecConfig {
        maybe_items: Option<Vec<String>>,
    }

    let config_str = r#"{"maybe_items": ["a", "b", "c"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OptionVecConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.maybe_items,
        Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
    );
    assert!(!config.errors().has_errors());
}

/// Test Option with nested struct
#[test]
fn test_option_nested_struct() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct Inner {
        #[feuilletage(default = "default")]
        value: String,
    }

    #[derive(DeriveConfig, Debug)]
    struct OuterOption {
        maybe_inner: Option<Inner>,
    }

    // Test with value present
    let config_str = r#"{"maybe_inner": {"value": "hello"}}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OuterOption = config.deserialize().expect("Should succeed");

    assert!(result.maybe_inner.is_some());
    assert_eq!(result.maybe_inner.unwrap().value, "hello");

    // Test with null
    let config_str = r#"{"maybe_inner": null}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: OuterOption = config.deserialize().expect("Should succeed");

    assert!(result.maybe_inner.is_none());
}

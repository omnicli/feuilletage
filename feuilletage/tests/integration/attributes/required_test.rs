//! Required field tests

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
struct RequiredFieldsConfig {
    required_string: String,
    required_int: i32,
}

#[derive(Debug, DeriveConfig)]
struct OptionalFieldsConfig {
    required_string: String,
    optional_field: Option<String>,
    #[feuilletage(default = "0")]
    with_default: i32,
}

#[test]
fn test_required_field_present() {
    let json = r#"{"required_string": "value", "required_int": 42}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RequiredFieldsConfig>();
    assert!(
        result.is_ok(),
        "Should succeed with all required fields present"
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.required_string, "value");
    assert_eq!(cfg.required_int, 42);
}

#[test]
fn test_required_field_missing() {
    let json = r#"{"required_string": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RequiredFieldsConfig>();
    assert!(
        result.is_err(),
        "Should fail when required field is missing"
    );
}

#[test]
fn test_optional_field_missing() {
    let json = r#"{"required_string": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<OptionalFieldsConfig>();
    assert!(
        result.is_ok(),
        "Should succeed when optional field is missing"
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.required_string, "value");
    assert_eq!(cfg.optional_field, None);
    assert_eq!(cfg.with_default, 0);
}

#[test]
fn test_default_field_missing() {
    let json = r#"{"required_string": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<OptionalFieldsConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.with_default, 0);
}

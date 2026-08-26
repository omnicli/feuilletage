//! Tests for custom validation function attribute.

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// Custom validation functions must be defined outside the test functions
// for the macro to find them

fn validate_positive(value: &i32) -> Result<(), String> {
    if *value > 0 {
        Ok(())
    } else {
        Err("value must be positive".to_string())
    }
}

fn validate_even(value: &i32) -> Result<(), String> {
    if *value % 2 == 0 {
        Ok(())
    } else {
        Err("value must be even".to_string())
    }
}

fn validate_non_empty(value: &String) -> Result<(), String> {
    if value.is_empty() {
        Err("value must not be empty".to_string())
    } else {
        Ok(())
    }
}

fn validate_no_spaces(value: &String) -> Result<(), String> {
    if value.contains(' ') {
        Err("value must not contain spaces".to_string())
    } else {
        Ok(())
    }
}

/// Test custom validation failure with default
#[test]
fn test_custom_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct CustomConfig {
        #[compote(validate = "validate_positive", default = "1")]
        positive_num: i32,
    }

    // -5 fails the custom validation
    let config_str = r#"{"positive_num": -5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CustomConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.positive_num, 1, "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("positive_num") || msg.contains("validation") || msg.contains("positive")
        }),
        "Expected custom validation error, got: {:?}",
        errors
    );
}

/// Test custom validation on required field - fails deserialization
#[test]
fn test_custom_required_field_fails_on_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredCustomConfig {
        /// Required field - must be positive, no default
        #[compote(validate = "validate_positive")]
        amount: i32,
    }

    // -5 fails custom validation
    let config_str = r#"{"amount": -5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredCustomConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field fails custom validation"
    );

    let err = result.unwrap_err();
    let err_str = err.to_string().to_lowercase();
    assert!(
        err_str.contains("amount")
            || err_str.contains("validation")
            || err_str.contains("positive"),
        "Error should mention custom validation, got: {}",
        err
    );
}

/// Test custom validation success
#[test]
fn test_custom_valid_value_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct CustomConfig {
        #[compote(validate = "validate_positive", default = "1")]
        positive_num: i32,
    }

    // 42 passes the validation
    let config_str = r#"{"positive_num": 42}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CustomConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.positive_num, 42, "should be 42");
    assert!(
        !config.errors().has_errors(),
        "Should not have errors for valid value"
    );
}

/// Test custom validation with string validator
#[test]
fn test_custom_string_validator() {
    #[derive(DeriveConfig, Debug)]
    struct StringValidatorConfig {
        #[compote(validate = "validate_non_empty", default = "default")]
        name: String,
    }

    // Empty string fails validation
    let config_str = r#"{"name": ""}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: StringValidatorConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(
        result.name, "default",
        "should use default for empty string"
    );
    assert!(config.errors().has_errors());
}

/// Test custom validation with different error message
#[test]
fn test_custom_validator_error_message() {
    #[derive(DeriveConfig, Debug)]
    struct NoSpacesConfig {
        #[compote(validate = "validate_no_spaces", default = "default")]
        identifier: String,
    }

    // "has spaces" fails validation
    let config_str = r#"{"identifier": "has spaces"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: NoSpacesConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.identifier, "default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| { e.to_string().contains("spaces") }),
        "Error message should mention spaces, got: {:?}",
        errors
    );
}

/// Test even number validator
#[test]
fn test_even_number_validator() {
    #[derive(DeriveConfig, Debug)]
    struct EvenConfig {
        #[compote(validate = "validate_even", default = "0")]
        count: i32,
    }

    // 3 is odd, fails validation
    let config_str = r#"{"count": 3}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EvenConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.count, 0, "should use default for odd number");
    assert!(config.errors().has_errors());

    // 4 is even, passes validation
    let config_str = r#"{"count": 4}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EvenConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.count, 4);
    assert!(!config.errors().has_errors());
}

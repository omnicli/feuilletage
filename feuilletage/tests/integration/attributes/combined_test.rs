//! Tests for multiple validators on the same field.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

fn validate_even(value: &i32) -> Result<(), String> {
    if *value % 2 == 0 {
        Ok(())
    } else {
        Err("value must be even".to_string())
    }
}

/// Test multiple validators: range + custom validation
#[test]
#[cfg(feature = "regex")]
fn test_range_and_custom_validation() {
    #[derive(DeriveConfig, Debug)]
    struct MultiValidationConfig {
        // Range 0-100 AND must be even
        #[feuilletage(range(min = 0, max = 100), validate = "validate_even", default = "50")]
        value: i32,
    }

    // 150 fails range (first validation)
    let config_str = r#"{"value": 150}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiValidationConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 50, "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("value") || msg.contains("range") || msg.contains("100")
        }),
        "Expected at least range validation error, got: {:?}",
        errors
    );
}

/// Test multiple validators where second validation fails
#[test]
fn test_range_passes_but_custom_fails() {
    #[derive(DeriveConfig, Debug)]
    struct MultiValidationConfig {
        // Range 0-100 AND must be even
        #[feuilletage(range(min = 0, max = 100), validate = "validate_even", default = "50")]
        value: i32,
    }

    // 51 passes range but fails even check
    let config_str = r#"{"value": 51}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiValidationConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.value, 50, "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("even") || msg.contains("validation")
        }),
        "Expected custom validation error, got: {:?}",
        errors
    );
}

/// Test all validators pass
#[test]
fn test_all_validators_pass() {
    #[derive(DeriveConfig, Debug)]
    struct MultiValidationConfig {
        // Range 0-100 AND must be even
        #[feuilletage(range(min = 0, max = 100), validate = "validate_even", default = "50")]
        value: i32,
    }

    // 42 passes both range and even check
    let config_str = r#"{"value": 42}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiValidationConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.value, 42, "should be 42");
    assert!(
        !config.errors().has_errors(),
        "Should not have errors when all validations pass"
    );
}

/// Test range + length validation on string that can be parsed to number
#[test]
fn test_length_and_custom_on_string() {
    fn validate_lowercase(value: &String) -> Result<(), String> {
        if value
            .chars()
            .all(|c| c.is_lowercase() || !c.is_alphabetic())
        {
            Ok(())
        } else {
            Err("value must be lowercase".to_string())
        }
    }

    #[derive(DeriveConfig, Debug)]
    struct MultiStringConfig {
        // Length 3-10 AND must be lowercase
        #[feuilletage(
            length(min = 3, max = 10),
            validate = "validate_lowercase",
            default = "default"
        )]
        name: String,
    }

    // "ab" fails length (too short)
    let config_str = r#"{"name": "ab"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiStringConfig = config.deserialize().expect("Should succeed with default");
    assert_eq!(result.name, "default");
    assert!(config.errors().has_errors());

    // "Hello" passes length but fails lowercase
    let config_str = r#"{"name": "Hello"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiStringConfig = config.deserialize().expect("Should succeed with default");
    assert_eq!(result.name, "default");
    assert!(config.errors().has_errors());

    // "hello" passes both
    let config_str = r#"{"name": "hello"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiStringConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, "hello");
    assert!(!config.errors().has_errors());
}

//! Tests for regex validation attribute.
//!
//! Note: These tests require the `regex` feature to be enabled.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

/// Test regex validation failure with default
#[test]
fn test_regex_with_default_uses_default_on_failure() {
    #[derive(DeriveConfig, Debug)]
    struct RegexConfig {
        #[feuilletage(regex = r"^[a-z]+$", default = "default")]
        lowercase_only: String,
    }

    // "Hello123" doesn't match ^[a-z]+$
    let config_str = r#"{"lowercase_only": "Hello123"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: RegexConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.lowercase_only, "default", "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("lowercase_only") || msg.contains("pattern") || msg.contains("match")
        }),
        "Expected regex validation error, got: {:?}",
        errors
    );
}

/// Test regex validation success
#[test]
fn test_regex_valid_value_succeeds() {
    #[derive(DeriveConfig, Debug)]
    struct RegexConfig {
        #[feuilletage(regex = r"^[a-z]+$", default = "default")]
        lowercase_only: String,
    }

    // "hello" matches ^[a-z]+$
    let config_str = r#"{"lowercase_only": "hello"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: RegexConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.lowercase_only, "hello", "should be hello");
    assert!(
        !config.errors().has_errors(),
        "Should not have errors for valid value"
    );
}

/// Test regex validation on required field - fails deserialization
#[test]
fn test_regex_required_field_fails_on_validation_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredRegexConfig {
        /// Required field with regex validation - no default!
        #[feuilletage(regex = r"^\d{3}-\d{4}$")]
        phone: String,
    }

    // "invalid" doesn't match phone pattern
    let config_str = r#"{"phone": "invalid"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredRegexConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field fails regex validation"
    );
}

/// Test email-like regex pattern
#[test]
fn test_regex_email_pattern() {
    #[derive(DeriveConfig, Debug)]
    struct EmailConfig {
        #[feuilletage(regex = r"^[^@]+@[^@]+\.[^@]+$", default = "default@example.com")]
        email: String,
    }

    // Invalid email
    let config_str = r#"{"email": "not-an-email"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EmailConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.email, "default@example.com");
    assert!(config.errors().has_errors());

    // Valid email
    let config_str = r#"{"email": "user@domain.com"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EmailConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.email, "user@domain.com");
    assert!(!config.errors().has_errors());
}

/// Test regex with special characters
#[test]
fn test_regex_with_special_chars() {
    #[derive(DeriveConfig, Debug)]
    struct VersionConfig {
        // Semver-like pattern
        #[feuilletage(regex = r"^\d+\.\d+\.\d+$", default = "0.0.0")]
        version: String,
    }

    // Valid semver
    let config_str = r#"{"version": "1.2.3"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: VersionConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.version, "1.2.3");
    assert!(!config.errors().has_errors());

    // Invalid semver
    let config_str = r#"{"version": "v1.2"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: VersionConfig = config.deserialize().expect("Should succeed with default");
    assert_eq!(result.version, "0.0.0");
    assert!(config.errors().has_errors());
}

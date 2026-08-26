//! Tests for secret attribute.
//!
//! The `#[feuilletage(secret)]` attribute marks fields as sensitive, affecting how
//! values are displayed in error messages. Secret values are redacted to prevent
//! accidental exposure in logs or debug output.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Basic secret attribute tests
// ============================================================================

#[test]
fn test_secret_field_deserializes_correctly() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretConfig {
        #[feuilletage(secret)]
        api_key: String,
    }

    let json = r#"{"api_key": "supersecretkey123"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretConfig = config.deserialize().expect("Should succeed");

    // The secret value should still be correctly deserialized
    assert_eq!(result.api_key, "supersecretkey123");
    assert!(!config.errors().has_errors());
}

#[test]
fn test_secret_field_with_default() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretDefaultConfig {
        #[feuilletage(secret, default = "default_secret")]
        password: String,
    }

    // Test with provided value
    let json = r#"{"password": "explicit_password"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretDefaultConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.password, "explicit_password");

    // Test with missing value (uses default)
    let json_missing = r#"{}"#;
    let mut config2 = Config::default();
    config2.load_json(
        json_missing,
        Context::new(Source::Programmatic, Level::User),
    );

    let result2: SecretDefaultConfig = config2.deserialize().expect("Should succeed");
    assert_eq!(result2.password, "default_secret");
}

#[test]
fn test_secret_required_field_missing() {
    #[derive(DeriveConfig, Debug)]
    struct SecretRequiredConfig {
        #[feuilletage(secret)]
        api_key: String,
    }

    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<SecretRequiredConfig, _> = config.deserialize();
    assert!(result.is_err(), "Missing required field should fail");

    // The error message should not expose any secret value (there wasn't one)
    let err_msg = format!("{:?}", result.err());
    assert!(err_msg.contains("api_key") || err_msg.contains("missing"));
}

// ============================================================================
// Secret with validation - error messages should be redacted
// ============================================================================

#[cfg(feature = "regex")]
#[test]
fn test_secret_with_regex_validation_failure() {
    #[derive(DeriveConfig, Debug)]
    struct SecretRegexConfig {
        #[feuilletage(secret, regex = r"^[a-z]+$", default = "default")]
        token: String,
    }

    // Value that doesn't match regex
    let json = r#"{"token": "ABC123SECRET"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let _result: SecretRegexConfig = config.deserialize().expect("Should succeed with default");

    // Check that the error message doesn't contain the actual secret value
    let errors = config.errors().errors();
    assert!(!errors.is_empty(), "Should have validation error");

    for error in errors {
        let error_str = error.to_string();
        // The actual value should NOT appear in the error message
        assert!(
            !error_str.contains("ABC123SECRET"),
            "Error message should not contain the secret value: {}",
            error_str
        );
        // Instead should show redacted marker or just mention the field
        // (Implementation may vary - just check value is not exposed)
    }
}

#[test]
fn test_secret_with_length_validation_failure() {
    #[derive(DeriveConfig, Debug)]
    struct SecretLengthConfig {
        #[feuilletage(secret, length(min = 8), default = "12345678")]
        password: String,
    }

    // Value that doesn't meet length requirement
    let json = r#"{"password": "short"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let _result: SecretLengthConfig = config.deserialize().expect("Should succeed with default");

    // Check that the error doesn't expose the secret
    let errors = config.errors().errors();
    assert!(!errors.is_empty(), "Should have validation error");

    for error in errors {
        let error_str = error.to_string();
        // The actual value should NOT appear
        assert!(
            !error_str.contains("short"),
            "Error message should not contain the secret value: {}",
            error_str
        );
    }
}

// ============================================================================
// Secret with different types
// ============================================================================

#[test]
fn test_secret_int_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretIntConfig {
        #[feuilletage(secret)]
        pin_code: i32,
    }

    let json = r#"{"pin_code": 1234}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretIntConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.pin_code, 1234);
}

#[test]
fn test_secret_option_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretOptionConfig {
        #[feuilletage(secret)]
        optional_key: Option<String>,
    }

    // With value
    let json = r#"{"optional_key": "secret_value"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretOptionConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.optional_key, Some("secret_value".to_string()));

    // Without value
    let json2 = r#"{}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));

    let result2: SecretOptionConfig = config2.deserialize().expect("Should succeed");
    assert_eq!(result2.optional_key, None);
}

// ============================================================================
// Secret combined with other attributes
// ============================================================================

#[test]
fn test_secret_with_rename() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretRenameConfig {
        #[feuilletage(rename = "apiKey", secret)]
        api_key: String,
    }

    let json = r#"{"apiKey": "renamed_secret"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretRenameConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.api_key, "renamed_secret");
}

#[test]
#[serial_test::serial]
fn test_secret_with_env() {
    use std::env;

    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretEnvConfig {
        #[feuilletage(env = "TEST_SECRET_API_KEY_12345", secret, default = "fallback")]
        api_key: String,
    }

    // Set env var
    env::set_var("TEST_SECRET_API_KEY_12345", "from_env_secret");

    let json = r#"{}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretEnvConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.api_key, "from_env_secret");

    // Clean up
    env::remove_var("TEST_SECRET_API_KEY_12345");
}

#[test]
fn test_secret_with_transform() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretTransformConfig {
        #[feuilletage(secret, transform = "trim")]
        token: String,
    }

    let json = r#"{"token": "  secret_with_spaces  "}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretTransformConfig = config.deserialize().expect("Should succeed");

    // Transform should still be applied to secret
    assert_eq!(result.token, "secret_with_spaces");
}

// ============================================================================
// Multiple secret fields
// ============================================================================

#[test]
fn test_multiple_secret_fields() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MultiSecretConfig {
        #[feuilletage(secret)]
        username: String,

        #[feuilletage(secret)]
        password: String,

        #[feuilletage(secret)]
        api_token: String,

        // Non-secret field
        public_name: String,
    }

    let json = r#"{
        "username": "admin",
        "password": "super_secret_password",
        "api_token": "token_12345",
        "public_name": "My App"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiSecretConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.username, "admin");
    assert_eq!(result.password, "super_secret_password");
    assert_eq!(result.api_token, "token_12345");
    assert_eq!(result.public_name, "My App");
}

// ============================================================================
// Secret serialization behavior
// ============================================================================

#[test]
fn test_secret_field_serializes_normally() {
    // Note: DeriveConfig automatically implements serde::Serialize
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretSerializeConfig {
        #[feuilletage(secret)]
        api_key: String,

        public_field: String,
    }

    let json = r#"{"api_key": "secret123", "public_field": "visible"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretSerializeConfig = config.deserialize().expect("Should succeed");

    // Note: The secret attribute doesn't affect serialization behavior by default
    // It only affects error messages. If you need to hide values during serialization,
    // use #[feuilletage(skip)] instead.
    let serialized = feuilletage::to_json_compact(&result).unwrap();
    assert!(serialized.contains("api_key"));
    assert!(serialized.contains("secret123")); // Secret IS serialized (use skip if you don't want this)
    assert!(serialized.contains("public_field"));
}

// ============================================================================
// Secret with skip (common pattern)
// ============================================================================

#[test]
fn test_secret_with_skip_serialization() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SecretSkipConfig {
        #[feuilletage(secret, skip, default = "hidden_default")]
        password: String,

        visible_field: String,
    }

    // Skip fields are not deserialized - they always use the default value
    // This matches serde's behavior: #[serde(skip)] skips both serialization AND deserialization
    let json = r#"{"password": "should_be_hidden", "visible_field": "visible"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SecretSkipConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.password, "hidden_default"); // Uses default (not deserialized)
    assert_eq!(result.visible_field, "visible");

    // With skip, the field should not be in serialized output
    let serialized = feuilletage::to_json_compact(&result).unwrap();
    assert!(
        !serialized.contains("password"),
        "Password field should be skipped"
    );
    assert!(
        !serialized.contains("should_be_hidden"),
        "Password value should not appear"
    );
    assert!(serialized.contains("visible_field"));
}

// ============================================================================
// Type mismatch with secret field
// ============================================================================

#[test]
fn test_secret_type_mismatch_redacted() {
    #[derive(DeriveConfig, Debug)]
    struct SecretTypeMismatchConfig {
        #[feuilletage(secret, default = "0")]
        secret_number: i32,
    }

    // Provide wrong type that can't be coerced
    let json = r#"{"secret_number": {"nested": "object"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let _result: SecretTypeMismatchConfig = config.deserialize().expect("Should use default");

    // Check errors - they should not expose the invalid value details
    let errors = config.errors().errors();
    assert!(!errors.is_empty(), "Should have type mismatch error");

    for error in errors {
        let error_str = error.to_string();
        // Shouldn't expose nested structure details (though type name is ok)
        assert!(
            !error_str.contains("nested"),
            "Error should not expose secret object structure: {}",
            error_str
        );
    }
}

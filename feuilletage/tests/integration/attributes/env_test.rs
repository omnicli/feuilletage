//! Tests for environment variable attribute.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;
use serial_test::serial;

/// Test env variable not found uses default
#[test]
#[serial]
fn test_env_variable_not_found_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct EnvConfig {
        #[feuilletage(
            env = "FEUILLETAGE_TEST_NONEXISTENT_VAR_12345",
            default = "default_value"
        )]
        from_env: String,
    }

    // Make sure the env var doesn't exist
    std::env::remove_var("FEUILLETAGE_TEST_NONEXISTENT_VAR_12345");

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EnvConfig = config.deserialize().expect("Should succeed with default");

    // Should use default since env var doesn't exist
    assert_eq!(result.from_env, "default_value", "should use default");
}

/// Test env variable found is used
#[test]
#[serial]
fn test_env_variable_found_is_used() {
    #[derive(DeriveConfig, Debug)]
    struct EnvConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_VAR_FOR_ENV_TEST", default = "default_value")]
        from_env: String,
    }

    // Set the env var
    std::env::set_var("FEUILLETAGE_TEST_VAR_FOR_ENV_TEST", "env_value");

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EnvConfig = config.deserialize().expect("Should succeed");

    // Should use env var value
    assert_eq!(result.from_env, "env_value", "should use env var value");

    // Cleanup
    std::env::remove_var("FEUILLETAGE_TEST_VAR_FOR_ENV_TEST");
}

/// Test config value takes precedence over env var
#[test]
#[serial]
fn test_config_value_precedence_over_env() {
    #[derive(DeriveConfig, Debug)]
    struct EnvConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_PRECEDENCE_VAR", default = "default_value")]
        value: String,
    }

    // Set the env var
    std::env::set_var("FEUILLETAGE_TEST_PRECEDENCE_VAR", "env_value");

    // Provide value in config
    let config_str = r#"{"value": "config_value"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EnvConfig = config.deserialize().expect("Should succeed");

    // Config value should take precedence over env var
    assert_eq!(
        result.value, "config_value",
        "config value should take precedence"
    );

    // Cleanup
    std::env::remove_var("FEUILLETAGE_TEST_PRECEDENCE_VAR");
}

/// Test env variable with integer value
#[test]
#[serial]
fn test_env_variable_with_integer() {
    #[derive(DeriveConfig, Debug)]
    struct EnvIntConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_INT_VAR", coerce, default = "0")]
        count: i32,
    }

    // Set the env var with numeric string
    std::env::set_var("FEUILLETAGE_TEST_INT_VAR", "42");

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EnvIntConfig = config.deserialize().expect("Should succeed");

    // Should parse the env var as integer
    assert_eq!(result.count, 42, "should parse env var as integer");

    // Cleanup
    std::env::remove_var("FEUILLETAGE_TEST_INT_VAR");
}

/// Test env variable with boolean value
#[test]
#[serial]
fn test_env_variable_with_boolean() {
    #[derive(DeriveConfig, Debug)]
    struct EnvBoolConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_BOOL_VAR", coerce, default = "false")]
        enabled: bool,
    }

    // Set the env var
    std::env::set_var("FEUILLETAGE_TEST_BOOL_VAR", "true");

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: EnvBoolConfig = config.deserialize().expect("Should succeed");

    // Should parse the env var as boolean
    assert!(result.enabled, "should parse env var as boolean");

    // Cleanup
    std::env::remove_var("FEUILLETAGE_TEST_BOOL_VAR");
}

/// Test required field with only env attribute fails when env not set
#[test]
#[serial]
fn test_required_env_fails_when_not_set() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredEnvConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_REQUIRED_ENV_VAR")]
        required_from_env: String,
    }

    // Make sure the env var doesn't exist
    std::env::remove_var("FEUILLETAGE_TEST_REQUIRED_ENV_VAR");

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredEnvConfig, _> = config.deserialize();

    // Should fail because required field is not set
    assert!(
        result.is_err(),
        "Should fail when required env field is not set"
    );
}

/// Test multiple env attributes
#[test]
#[serial]
fn test_multiple_env_attributes() {
    #[derive(DeriveConfig, Debug)]
    struct MultiEnvConfig {
        #[feuilletage(env = "FEUILLETAGE_TEST_HOST", default = "localhost")]
        host: String,

        #[feuilletage(env = "FEUILLETAGE_TEST_PORT", coerce, default = "8080")]
        port: u16,

        #[feuilletage(env = "FEUILLETAGE_TEST_DEBUG", coerce, default = "false")]
        debug: bool,
    }

    // Set some env vars
    std::env::set_var("FEUILLETAGE_TEST_HOST", "example.com");
    std::env::set_var("FEUILLETAGE_TEST_PORT", "3000");
    // Don't set debug, should use default

    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MultiEnvConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.host, "example.com");
    assert_eq!(result.port, 3000);
    assert!(!result.debug, "debug should use default (false)");

    // Cleanup
    std::env::remove_var("FEUILLETAGE_TEST_HOST");
    std::env::remove_var("FEUILLETAGE_TEST_PORT");
}

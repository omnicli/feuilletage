//! Environment Variable Loading Pattern
//!
//! Common use cases:
//! - Secrets (API keys, passwords, tokens)
//! - Machine-specific settings (hostnames, ports)
//! - CI/CD configuration overrides
//! - Cloud provider credentials
//!
//! Feuilletage Solution: `#[feuilletage(env = "VAR_NAME")]`
//!
//! Example:
//! ```yaml
//! # Config file - env vars can override or provide defaults
//! api_key: "default-key"  # Overridden by TEST_API_KEY env var
//! timeout: 30             # Overridden by TEST_TIMEOUT env var
//! ```

use feuilletage::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Config with environment variable loading
#[derive(Debug, feuilletage::Config, PartialEq)]
struct EnvConfig {
    /// Required field from env
    #[feuilletage(env = "TEST_API_KEY")]
    api_key: String,

    /// Optional field from env
    #[feuilletage(env = "TEST_SECRET")]
    secret: Option<String>,

    /// Field with fallback default
    #[feuilletage(env = "TEST_TIMEOUT", default = "30")]
    timeout: i32,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Environment Variable Loading Examples ===\n");

    // Set up test environment
    std::env::set_var("TEST_API_KEY", "api_key_from_env");
    std::env::set_var("TEST_SECRET", "secret_from_env");
    std::env::set_var("TEST_TIMEOUT", "60");

    // Basic env loading
    println!("--- Basic Environment Loading ---");
    let json = r#"{}"#;
    let config: EnvConfig = deserialize_json(json).expect("Should load from env");
    println!("api_key: {}", config.api_key);
    println!("secret: {:?}", config.secret);
    println!("timeout: {}", config.timeout);
    assert_eq!(config.api_key, "api_key_from_env");
    assert_eq!(config.secret, Some("secret_from_env".to_string()));
    assert_eq!(config.timeout, 60);

    // Config value overrides env
    println!("\n--- Config Value Overrides Environment ---");
    let json = r#"{"api_key": "config_api_key", "timeout": 120}"#;
    let config: EnvConfig = deserialize_json(json).expect("Config should override env");
    println!("api_key: {} (from config, not env)", config.api_key);
    println!("timeout: {} (from config, not env)", config.timeout);
    assert_eq!(config.api_key, "config_api_key");
    assert_eq!(config.timeout, 120);

    // Default when env not set
    println!("\n--- Default When Environment Not Set ---");
    std::env::remove_var("TEST_TIMEOUT");
    let json = r#"{}"#;
    let config: EnvConfig = deserialize_json(json).expect("Should use default when env missing");
    println!("timeout: {} (default)", config.timeout);
    assert_eq!(config.timeout, 30);
    std::env::set_var("TEST_TIMEOUT", "60"); // Restore for other tests

    // Missing required env
    println!("\n--- Missing Required Environment ---");
    std::env::remove_var("TEST_API_KEY");
    let json = r#"{}"#;
    let result = deserialize_json::<EnvConfig>(json);
    match &result {
        Ok(_) => println!("Unexpected success (env might be set globally)"),
        Err(_) => println!("Error (expected): missing required field api_key"),
    }
    std::env::set_var("TEST_API_KEY", "api_key_from_env"); // Restore

    // Multiple env configs
    println!("\n--- Multiple Environment Configs ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct MultiEnvConfig {
        #[feuilletage(env = "DB_HOST", default = "localhost")]
        db_host: String,

        #[feuilletage(env = "DB_PORT", coerce, default = "5432")]
        db_port: i32,

        #[feuilletage(env = "DB_USER", default = "postgres")]
        db_user: String,

        #[feuilletage(env = "DB_PASSWORD")]
        db_password: Option<String>,
    }

    std::env::set_var("DB_HOST", "db.example.com");
    std::env::set_var("DB_PORT", "5433");
    std::env::remove_var("DB_USER");
    std::env::set_var("DB_PASSWORD", "secret_password");

    let json = r#"{}"#;
    let config: MultiEnvConfig = deserialize_json(json).expect("Multi env config");
    println!("db_host: {} (from env)", config.db_host);
    println!("db_port: {} (from env)", config.db_port);
    println!("db_user: {} (default)", config.db_user);
    println!("db_password: {:?} (from env)", config.db_password);
    assert_eq!(config.db_host, "db.example.com");
    assert_eq!(config.db_port, 5433);
    assert_eq!(config.db_user, "postgres"); // default
    assert_eq!(config.db_password, Some("secret_password".to_string()));

    // Real-world: Cloud Config
    println!("\n--- Real-World: Cloud Config ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct CloudConfig {
        #[feuilletage(env = "CLOUD_REGION", default = "us-east-1")]
        region: String,

        #[feuilletage(env = "CLOUD_ACCESS_KEY")]
        access_key: Option<String>,

        #[feuilletage(env = "CLOUD_SECRET_KEY", secret)]
        secret_key: Option<String>,

        #[feuilletage(env = "CLOUD_PROFILE", default = "default")]
        profile: String,
    }

    std::env::set_var("CLOUD_REGION", "eu-west-1");
    std::env::set_var("CLOUD_ACCESS_KEY", "AKIAEXAMPLE");
    std::env::set_var(
        "CLOUD_SECRET_KEY",
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    );
    std::env::remove_var("CLOUD_PROFILE");

    let json = r#"{}"#;
    let config: CloudConfig = deserialize_json(json).expect("Cloud config");
    println!("region: {}", config.region);
    println!("access_key: {:?}", config.access_key);
    println!("secret_key: {:?} (marked as secret)", config.secret_key);
    println!("profile: {}", config.profile);
    assert_eq!(config.region, "eu-west-1");
    assert_eq!(config.access_key, Some("AKIAEXAMPLE".to_string()));
    assert!(config.secret_key.is_some());
    assert_eq!(config.profile, "default");

    // Cleanup
    std::env::remove_var("TEST_API_KEY");
    std::env::remove_var("TEST_SECRET");
    std::env::remove_var("TEST_TIMEOUT");
    std::env::remove_var("DB_HOST");
    std::env::remove_var("DB_PORT");
    std::env::remove_var("DB_USER");
    std::env::remove_var("DB_PASSWORD");
    std::env::remove_var("CLOUD_REGION");
    std::env::remove_var("CLOUD_ACCESS_KEY");
    std::env::remove_var("CLOUD_SECRET_KEY");
    std::env::remove_var("CLOUD_PROFILE");

    println!("\n=== All environment loading examples passed! ===");
}

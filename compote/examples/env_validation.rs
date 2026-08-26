//! Environment Variable Loading Pattern: Override config from environment
//!
//! Common use cases:
//! - API keys and secrets
//! - Database connection strings
//! - CI/CD configuration overrides
//! - Machine-specific settings
//!
//! Compote Solution: `#[compote(env = "VAR_NAME")]`
//!
//! Example:
//! ```yaml
//! # Config file (env_field not set)
//! name: "test"
//! ```
//! With `TEST_CONFIG_VALUE=from_env`, the env_field will be "from_env"

use compote::{Config as ConfigContainer, Context, Level, Source};

#[derive(Debug, compote::Config)]
struct EnvConfig {
    #[compote(env = "TEST_CONFIG_VALUE", default = "default")]
    env_field: String,

    name: String,
}

fn main() {
    println!("=== Testing Environment Variable Loading ===");

    // Test without env var set (should use default)
    let json = r#"{
        "name": "test"
    }"#;

    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    match config.deserialize::<EnvConfig>() {
        Ok(cfg) => println!("Config without env var: {:?}", cfg),
        Err(e) => println!("Error: {:?}", e),
    }

    // Set env var
    std::env::set_var("TEST_CONFIG_VALUE", "from_env");

    let mut config2 = ConfigContainer::default();
    config2.load_json(json, Context::new(Source::Programmatic, Level::User));

    match config2.deserialize::<EnvConfig>() {
        Ok(cfg) => println!("Config with env var: {:?}", cfg),
        Err(e) => println!("Error: {:?}", e),
    }

    std::env::remove_var("TEST_CONFIG_VALUE");
}

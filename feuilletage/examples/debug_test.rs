//! Basic Deserialization Test: Core config loading features
//!
//! Common use cases:
//! - Quick testing of config struct definitions
//! - Validating default values work correctly
//! - Testing allow_single for Vec fields
//! - Verifying optional fields handle missing values
//!
//! Feuilletage Solution: Multiple attributes working together
//!
//! Example:
//! ```yaml
//! name: "test"
//! count: 42
//! items: ["a", "b", "c"]
//! # with_default uses default when missing
//! # optional becomes None when missing
//! ```

use feuilletage::{Config, Context, Level, Source};

#[derive(Debug, feuilletage::Config)]
struct TestConfig {
    name: String,
    count: i32,

    #[feuilletage(default = "default_value")]
    with_default: String,

    #[feuilletage(allow_single)]
    items: Vec<String>,

    optional: Option<String>,
}

fn main() {
    let json = r#"{
        "name": "test",
        "count": 42,
        "items": ["a", "b", "c"]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    println!("Config loaded. Errors: {:?}", config.get_errors());

    let result = config.deserialize::<TestConfig>();
    match result {
        Ok(cfg) => println!("Success: {:?}", cfg),
        Err(e) => println!("Error: {:?}", e),
    }
}

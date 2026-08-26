//! Range Validation Pattern: Numeric bounds checking
//!
//! Common use cases:
//! - Percentage values (0-100)
//! - Port numbers (1-65535)
//! - Thread counts (1-N)
//! - Priority levels (1-10)
//!
//! Compote Solution: `#[compote(range(min, max))]`
//!
//! Example:
//! ```yaml
//! percentage: 75  # Valid: within 0-100
//! percentage: 150 # Error: out of range
//! ```

use compote::{Config as ConfigContainer, Context, Level, Source};

#[derive(Debug, compote::Config)]
struct SimpleConfig {
    // Range validation
    #[compote(range(0, 100))]
    percentage: i32,

    name: String,
}

fn main() {
    println!("=== Testing Range Validation ===");

    // Test valid range
    let json_valid = r#"{
        "percentage": 50,
        "name": "test"
    }"#;

    let mut config = ConfigContainer::default();
    config.load_json(json_valid, Context::new(Source::Programmatic, Level::User));

    match config.deserialize::<SimpleConfig>() {
        Ok(cfg) => println!("Valid config: {:?}", cfg),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test invalid range
    let json_invalid = r#"{
        "percentage": 150,
        "name": "test"
    }"#;

    let mut config2 = ConfigContainer::default();
    config2.load_json(
        json_invalid,
        Context::new(Source::Programmatic, Level::User),
    );

    match config2.deserialize::<SimpleConfig>() {
        Ok(cfg) => println!("Config (should have failed): {:?}", cfg),
        Err(e) => println!("Expected error: {:?}", e),
    }
}

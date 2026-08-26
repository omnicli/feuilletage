//! Type Coercion Pattern: Liberal type conversion
//!
//! Common use cases:
//! - User-friendly boolean inputs ("true", "yes", "1", "on")
//! - String-to-number conversion from environment variables
//! - Number-to-string conversion for display values
//! - CLI flag parsing with flexible input types
//!
//! Feuilletage Solution: `#[feuilletage(coerce)]`
//!
//! Example:
//! ```yaml
//! verbose: "true"    # Coerced to bool: true
//! verbose: "yes"     # Coerced to bool: true
//! verbose: 1         # Coerced to bool: true
//! jobs: "4"          # Coerced to int: 4
//! port: 8080         # Coerced to string: "8080"
//! ```

#![allow(clippy::approx_constant)]

use feuilletage::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Config with coercible fields
#[derive(Debug, feuilletage::Config, PartialEq)]
struct CoerceConfig {
    #[feuilletage(coerce)]
    string_field: String,

    #[feuilletage(coerce)]
    bool_field: bool,

    #[feuilletage(coerce)]
    int_field: i64,

    #[feuilletage(coerce)]
    float_field: f64,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Type Coercion Examples ===\n");

    // String Coercion Tests
    println!("--- String Coercion ---");

    let json = r#"{"string_field": true, "bool_field": true, "int_field": 1, "float_field": 1.0}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce bool to string");
    println!("Bool to string: {} (from true)", config.string_field);
    assert_eq!(config.string_field, "true");

    let json = r#"{"string_field": 42, "bool_field": true, "int_field": 1, "float_field": 1.0}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce int to string");
    println!("Int to string: {} (from 42)", config.string_field);
    assert_eq!(config.string_field, "42");

    let json = r#"{"string_field": 3.14, "bool_field": true, "int_field": 1, "float_field": 1.0}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce float to string");
    println!("Float to string: {} (from 3.14)", config.string_field);
    assert!(config.string_field.starts_with("3.14"));

    // Boolean Coercion Tests
    println!("\n--- Boolean Coercion ---");

    for value in ["true", "yes", "y", "on", "1", "TRUE", "Yes", "Y", "ON"] {
        let json = format!(
            r#"{{"string_field": "x", "bool_field": "{value}", "int_field": 1, "float_field": 1.0}}"#
        );
        let config: CoerceConfig =
            deserialize_json(&json).unwrap_or_else(|_| panic!("Should coerce '{}' to true", value));
        println!("'{}' -> {} (expected true)", value, config.bool_field);
        assert!(config.bool_field, "'{}' should be true", value);
    }

    for value in ["false", "no", "n", "off", "0", "FALSE", "No", "N", "OFF"] {
        let json = format!(
            r#"{{"string_field": "x", "bool_field": "{value}", "int_field": 1, "float_field": 1.0}}"#
        );
        let config: CoerceConfig = deserialize_json(&json)
            .unwrap_or_else(|_| panic!("Should coerce '{}' to false", value));
        println!("'{}' -> {} (expected false)", value, config.bool_field);
        assert!(!config.bool_field, "'{}' should be false", value);
    }

    // Integer Coercion Tests
    println!("\n--- Integer Coercion ---");

    let json =
        r#"{"string_field": "x", "bool_field": true, "int_field": "42", "float_field": 1.0}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce string to int");
    println!("String to int: {} (from \"42\")", config.int_field);
    assert_eq!(config.int_field, 42);

    let json =
        r#"{"string_field": "x", "bool_field": true, "int_field": 42.0, "float_field": 1.0}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce float to int");
    println!("Float to int: {} (from 42.0)", config.int_field);
    assert_eq!(config.int_field, 42);

    // Float Coercion Tests
    println!("\n--- Float Coercion ---");

    let json =
        r#"{"string_field": "x", "bool_field": true, "int_field": 1, "float_field": "3.14159"}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce string to float");
    println!("String to float: {} (from \"3.14159\")", config.float_field);
    assert!((config.float_field - 3.14159).abs() < 0.00001);

    let json = r#"{"string_field": "x", "bool_field": true, "int_field": 1, "float_field": 42}"#;
    let config: CoerceConfig = deserialize_json(json).expect("Should coerce int to float");
    println!("Int to float: {} (from 42)", config.float_field);
    assert_eq!(config.float_field, 42.0);

    // Real-World Scenario: CLI flags
    println!("\n--- Real-World: CLI Flag Config ---");

    /// CLI flag config
    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct CliFlagConfig {
        #[feuilletage(coerce, default = "false")]
        verbose: bool,

        #[feuilletage(coerce, default = "false")]
        quiet: bool,

        #[feuilletage(coerce, default = "1")]
        jobs: i32,

        #[feuilletage(coerce, default = "info")]
        log_level: String,
    }

    let json = r#"{
        "verbose": "true",
        "quiet": 0,
        "jobs": "4",
        "log_level": "debug"
    }"#;
    let config: CliFlagConfig = deserialize_json(json).expect("Should parse CLI flags");

    println!("verbose: {} (from \"true\")", config.verbose);
    println!("quiet: {} (from 0)", config.quiet);
    println!("jobs: {} (from \"4\")", config.jobs);
    println!("log_level: {} (from \"debug\")", config.log_level);

    assert!(config.verbose);
    assert!(!config.quiet);
    assert_eq!(config.jobs, 4);
    assert_eq!(config.log_level, "debug");

    println!("\n=== All type coercion examples passed! ===");
}

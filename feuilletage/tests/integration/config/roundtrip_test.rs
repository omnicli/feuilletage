//! Round-trip serialization/deserialization tests.
//!
//! These tests verify that configurations can be:
//! 1. Loaded from format string
//! 2. Deserialized to typed struct
//! 3. Serialized back to format string
//! 4. Loaded again and deserialized
//! 5. Compared for equality
//!
//! Note: Round-trip equality is verified at the struct level, not string level,
//! because transformations like allow_single, allow_map, and coerce may change
//! the serialized representation.

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// =============================================================================
// Test Structs
// =============================================================================

/// Simple struct with primitive types only
#[derive(Debug, DeriveConfig, PartialEq)]
struct SimpleConfig {
    name: String,
    count: i32,
    enabled: bool,
    ratio: f64,
}

/// Struct with Vec fields (no allow_single to ensure clean round-trip)
#[derive(Debug, DeriveConfig, PartialEq)]
struct VecConfig {
    tags: Vec<String>,
    numbers: Vec<i32>,
}

/// Struct with Option fields
#[derive(Debug, DeriveConfig, PartialEq)]
struct OptionConfig {
    required: String,
    optional_string: Option<String>,
    optional_int: Option<i32>,
}

/// Nested struct for testing object nesting
#[derive(Debug, DeriveConfig, PartialEq)]
struct DatabaseConfig {
    host: String,
    port: i32,
}

/// Struct with nested objects
#[derive(Debug, DeriveConfig, PartialEq)]
struct NestedConfig {
    name: String,
    database: DatabaseConfig,
}

/// Helper to create a test context
fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

// =============================================================================
// YAML Round-trip Tests
// =============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_roundtrip_yaml_simple() {
    let yaml = r#"
name: test-app
count: 42
enabled: true
ratio: 3.14
"#;

    // Step 1: Load and deserialize
    let mut config1 = Config::default();
    config1.load_yaml(yaml, test_context());
    assert!(!config1.has_errors(), "Initial load should not have errors");

    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "test-app");
    assert_eq!(struct1.count, 42);
    assert!(struct1.enabled);
    assert!((struct1.ratio - 3.14).abs() < f64::EPSILON);

    // Step 2: Serialize back to YAML
    let serialized = config1.to_yaml().expect("Serialization should succeed");

    // Step 3: Load the serialized string
    let mut config2 = Config::default();
    config2.load_yaml(&serialized, test_context());
    assert!(!config2.has_errors(), "Second load should not have errors");

    // Step 4: Deserialize again and compare
    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "yaml")]
fn test_roundtrip_yaml_with_vec() {
    let yaml = r#"
tags:
  - alpha
  - beta
  - gamma
numbers:
  - 1
  - 2
  - 3
"#;

    // Step 1: Load and deserialize
    let mut config1 = Config::default();
    config1.load_yaml(yaml, test_context());
    assert!(!config1.has_errors());

    let struct1: VecConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.tags, vec!["alpha", "beta", "gamma"]);
    assert_eq!(struct1.numbers, vec![1, 2, 3]);

    // Step 2: Serialize back to YAML
    let serialized = config1.to_yaml().expect("Serialization should succeed");

    // Step 3: Load and deserialize again
    let mut config2 = Config::default();
    config2.load_yaml(&serialized, test_context());
    assert!(!config2.has_errors());

    let struct2: VecConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "yaml")]
fn test_roundtrip_yaml_with_option() {
    // Test with all options present
    let yaml_with_options = r#"
required: must-have
optional_string: present
optional_int: 99
"#;

    let mut config1 = Config::default();
    config1.load_yaml(yaml_with_options, test_context());
    let struct1: OptionConfig = config1
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct1.required, "must-have");
    assert_eq!(struct1.optional_string, Some("present".to_string()));
    assert_eq!(struct1.optional_int, Some(99));

    let serialized = config1.to_yaml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_yaml(&serialized, test_context());
    let struct2: OptionConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);

    // Test with options missing
    let yaml_without_options = r#"
required: only-this
"#;

    let mut config3 = Config::default();
    config3.load_yaml(yaml_without_options, test_context());
    let struct3: OptionConfig = config3
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct3.required, "only-this");
    assert_eq!(struct3.optional_string, None);
    assert_eq!(struct3.optional_int, None);

    let serialized = config3.to_yaml().expect("Serialization should succeed");
    let mut config4 = Config::default();
    config4.load_yaml(&serialized, test_context());
    let struct4: OptionConfig = config4
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct3, struct4);
}

#[test]
#[cfg(feature = "yaml")]
fn test_roundtrip_yaml_nested() {
    let yaml = r#"
name: my-app
database:
  host: localhost
  port: 5432
"#;

    let mut config1 = Config::default();
    config1.load_yaml(yaml, test_context());
    assert!(!config1.has_errors());

    let struct1: NestedConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "my-app");
    assert_eq!(struct1.database.host, "localhost");
    assert_eq!(struct1.database.port, 5432);

    let serialized = config1.to_yaml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_yaml(&serialized, test_context());
    let struct2: NestedConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);
}

// =============================================================================
// JSON Round-trip Tests
// =============================================================================

#[test]
#[cfg(feature = "json")]
fn test_roundtrip_json_simple() {
    let json = r#"{
        "name": "test-app",
        "count": 42,
        "enabled": true,
        "ratio": 3.14
    }"#;

    let mut config1 = Config::default();
    config1.load_json(json, test_context());
    assert!(!config1.has_errors(), "Initial load should not have errors");

    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "test-app");
    assert_eq!(struct1.count, 42);
    assert!(struct1.enabled);
    assert!((struct1.ratio - 3.14).abs() < f64::EPSILON);

    let serialized = config1.to_json().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_json(&serialized, test_context());
    assert!(!config2.has_errors(), "Second load should not have errors");

    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "json")]
fn test_roundtrip_json_with_vec() {
    let json = r#"{
        "tags": ["alpha", "beta", "gamma"],
        "numbers": [1, 2, 3]
    }"#;

    let mut config1 = Config::default();
    config1.load_json(json, test_context());
    assert!(!config1.has_errors());

    let struct1: VecConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.tags, vec!["alpha", "beta", "gamma"]);
    assert_eq!(struct1.numbers, vec![1, 2, 3]);

    let serialized = config1.to_json().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_json(&serialized, test_context());
    assert!(!config2.has_errors());

    let struct2: VecConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "json")]
fn test_roundtrip_json_with_option() {
    // Test with all options present
    let json_with_options = r#"{
        "required": "must-have",
        "optional_string": "present",
        "optional_int": 99
    }"#;

    let mut config1 = Config::default();
    config1.load_json(json_with_options, test_context());
    let struct1: OptionConfig = config1
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct1.required, "must-have");
    assert_eq!(struct1.optional_string, Some("present".to_string()));
    assert_eq!(struct1.optional_int, Some(99));

    let serialized = config1.to_json().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_json(&serialized, test_context());
    let struct2: OptionConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);

    // Test with options missing
    let json_without_options = r#"{
        "required": "only-this"
    }"#;

    let mut config3 = Config::default();
    config3.load_json(json_without_options, test_context());
    let struct3: OptionConfig = config3
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct3.required, "only-this");
    assert_eq!(struct3.optional_string, None);
    assert_eq!(struct3.optional_int, None);

    let serialized = config3.to_json().expect("Serialization should succeed");
    let mut config4 = Config::default();
    config4.load_json(&serialized, test_context());
    let struct4: OptionConfig = config4
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct3, struct4);
}

#[test]
#[cfg(feature = "json")]
fn test_roundtrip_json_nested() {
    let json = r#"{
        "name": "my-app",
        "database": {
            "host": "localhost",
            "port": 5432
        }
    }"#;

    let mut config1 = Config::default();
    config1.load_json(json, test_context());
    assert!(!config1.has_errors());

    let struct1: NestedConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "my-app");
    assert_eq!(struct1.database.host, "localhost");
    assert_eq!(struct1.database.port, 5432);

    let serialized = config1.to_json().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_json(&serialized, test_context());
    let struct2: NestedConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);
}

// =============================================================================
// TOML Round-trip Tests
// =============================================================================

#[test]
#[cfg(feature = "toml")]
fn test_roundtrip_toml_simple() {
    let toml_str = r#"
name = "test-app"
count = 42
enabled = true
ratio = 3.14
"#;

    let mut config1 = Config::default();
    config1.load_toml(toml_str, test_context());
    assert!(
        !config1.has_errors(),
        "Initial load should not have errors: {:?}",
        config1.get_errors()
    );

    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "test-app");
    assert_eq!(struct1.count, 42);
    assert!(struct1.enabled);
    assert!((struct1.ratio - 3.14).abs() < f64::EPSILON);

    let serialized = config1.to_toml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_toml(&serialized, test_context());
    assert!(
        !config2.has_errors(),
        "Second load should not have errors: {:?}",
        config2.get_errors()
    );

    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "toml")]
fn test_roundtrip_toml_with_vec() {
    let toml_str = r#"
tags = ["alpha", "beta", "gamma"]
numbers = [1, 2, 3]
"#;

    let mut config1 = Config::default();
    config1.load_toml(toml_str, test_context());
    assert!(!config1.has_errors());

    let struct1: VecConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.tags, vec!["alpha", "beta", "gamma"]);
    assert_eq!(struct1.numbers, vec![1, 2, 3]);

    let serialized = config1.to_toml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_toml(&serialized, test_context());
    assert!(!config2.has_errors());

    let struct2: VecConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2, "Round-trip should produce equal structs");
}

#[test]
#[cfg(feature = "toml")]
fn test_roundtrip_toml_with_option() {
    // Test with all options present
    let toml_with_options = r#"
required = "must-have"
optional_string = "present"
optional_int = 99
"#;

    let mut config1 = Config::default();
    config1.load_toml(toml_with_options, test_context());
    let struct1: OptionConfig = config1
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct1.required, "must-have");
    assert_eq!(struct1.optional_string, Some("present".to_string()));
    assert_eq!(struct1.optional_int, Some(99));

    let serialized = config1.to_toml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_toml(&serialized, test_context());
    let struct2: OptionConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);

    // Test with options missing
    let toml_without_options = r#"
required = "only-this"
"#;

    let mut config3 = Config::default();
    config3.load_toml(toml_without_options, test_context());
    let struct3: OptionConfig = config3
        .deserialize()
        .expect("Deserialization should succeed");

    assert_eq!(struct3.required, "only-this");
    assert_eq!(struct3.optional_string, None);
    assert_eq!(struct3.optional_int, None);

    let serialized = config3.to_toml().expect("Serialization should succeed");
    let mut config4 = Config::default();
    config4.load_toml(&serialized, test_context());
    let struct4: OptionConfig = config4
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct3, struct4);
}

#[test]
#[cfg(feature = "toml")]
fn test_roundtrip_toml_nested() {
    let toml_str = r#"
name = "my-app"

[database]
host = "localhost"
port = 5432
"#;

    let mut config1 = Config::default();
    config1.load_toml(toml_str, test_context());
    assert!(!config1.has_errors(), "Errors: {:?}", config1.get_errors());

    let struct1: NestedConfig = config1
        .deserialize()
        .expect("First deserialization should succeed");
    assert_eq!(struct1.name, "my-app");
    assert_eq!(struct1.database.host, "localhost");
    assert_eq!(struct1.database.port, 5432);

    let serialized = config1.to_toml().expect("Serialization should succeed");
    let mut config2 = Config::default();
    config2.load_toml(&serialized, test_context());
    let struct2: NestedConfig = config2
        .deserialize()
        .expect("Second deserialization should succeed");
    assert_eq!(struct1, struct2);
}

// =============================================================================
// Cross-format Tests
// =============================================================================

/// Test that data loaded from one format can be serialized to another and back
#[test]
#[cfg(all(feature = "yaml", feature = "json"))]
fn test_crossformat_yaml_to_json() {
    let yaml = r#"
name: cross-format-test
count: 100
enabled: false
ratio: 2.718
"#;

    // Load from YAML
    let mut config1 = Config::default();
    config1.load_yaml(yaml, test_context());
    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("YAML deserialization should succeed");

    // Serialize to JSON
    let json = config1
        .to_json()
        .expect("JSON serialization should succeed");

    // Load from JSON
    let mut config2 = Config::default();
    config2.load_json(&json, test_context());
    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("JSON deserialization should succeed");

    assert_eq!(
        struct1, struct2,
        "Cross-format conversion should preserve data"
    );
}

#[test]
#[cfg(all(feature = "json", feature = "toml"))]
fn test_crossformat_json_to_toml() {
    let json = r#"{
        "name": "cross-format-test",
        "count": 100,
        "enabled": false,
        "ratio": 2.718
    }"#;

    // Load from JSON
    let mut config1 = Config::default();
    config1.load_json(json, test_context());
    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("JSON deserialization should succeed");

    // Serialize to TOML
    let toml_str = config1
        .to_toml()
        .expect("TOML serialization should succeed");

    // Load from TOML
    let mut config2 = Config::default();
    config2.load_toml(&toml_str, test_context());
    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("TOML deserialization should succeed");

    assert_eq!(
        struct1, struct2,
        "Cross-format conversion should preserve data"
    );
}

#[test]
#[cfg(all(feature = "toml", feature = "yaml"))]
fn test_crossformat_toml_to_yaml() {
    let toml_str = r#"
name = "cross-format-test"
count = 100
enabled = false
ratio = 2.718
"#;

    // Load from TOML
    let mut config1 = Config::default();
    config1.load_toml(toml_str, test_context());
    let struct1: SimpleConfig = config1
        .deserialize()
        .expect("TOML deserialization should succeed");

    // Serialize to YAML
    let yaml = config1
        .to_yaml()
        .expect("YAML serialization should succeed");

    // Load from YAML
    let mut config2 = Config::default();
    config2.load_yaml(&yaml, test_context());
    let struct2: SimpleConfig = config2
        .deserialize()
        .expect("YAML deserialization should succeed");

    assert_eq!(
        struct1, struct2,
        "Cross-format conversion should preserve data"
    );
}

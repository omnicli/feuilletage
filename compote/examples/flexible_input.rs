//! Flexible Input Pattern: Multi-format input acceptance
//!
//! This pattern is useful for configs that accept:
//! - A string (treated as one field)
//! - An array (treated as another field)
//! - A full object with all fields
//!
//! Example use case: A config that can be specified as:
//! - `"config.yaml"` - a file path
//! - `["item1", "item2"]` - a list of items
//! - `{file: "config.yaml", items: ["item1"]}` - full specification
//!
//! Compote Solution: `#[compote(scalar_as = "field", array_as = "field")]`

use compote::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// A spec that accepts string, array, or object input
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "file", array_as = "items")]
struct FlexibleSpec {
    file: Option<String>,
    #[compote(default)]
    items: Vec<String>,
}

/// Helper to deserialize JSON to a type
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Flexible Input Examples ===\n");

    // From string input
    println!("--- From String ---");
    let json = r#""config.yaml""#;
    let spec: FlexibleSpec = deserialize_json(json).expect("Should deserialize from string");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, Some("config.yaml".to_string()));
    assert!(spec.items.is_empty());

    // From array input
    println!("\n--- From Array ---");
    let json = r#"["item1", "item2"]"#;
    let spec: FlexibleSpec = deserialize_json(json).expect("Should deserialize from array");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, None);
    assert_eq!(spec.items, vec!["item1".to_string(), "item2".to_string()]);

    // From object with file
    println!("\n--- From Object (file only) ---");
    let json = r#"{"file": "config.yaml"}"#;
    let spec: FlexibleSpec =
        deserialize_json(json).expect("Should deserialize from object with file");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, Some("config.yaml".to_string()));
    assert!(spec.items.is_empty());

    // From object with items
    println!("\n--- From Object (items only) ---");
    let json = r#"{"items": ["item1"]}"#;
    let spec: FlexibleSpec =
        deserialize_json(json).expect("Should deserialize from object with items");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, None);
    assert_eq!(spec.items, vec!["item1".to_string()]);

    // From full object
    println!("\n--- From Full Object ---");
    let json = r#"{"file": "settings.yaml", "items": ["feature1", "feature2"]}"#;
    let spec: FlexibleSpec = deserialize_json(json).expect("Should deserialize from full object");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, Some("settings.yaml".to_string()));
    assert_eq!(
        spec.items,
        vec!["feature1".to_string(), "feature2".to_string()]
    );

    // Empty array
    println!("\n--- From Empty Array ---");
    let json = r#"[]"#;
    let spec: FlexibleSpec = deserialize_json(json).expect("Should deserialize from empty array");
    println!("Input: {}", json);
    println!("Result: file={:?}, items={:?}", spec.file, spec.items);
    assert_eq!(spec.file, None);
    assert!(spec.items.is_empty());

    // Nested in parent struct
    println!("\n--- Nested in Parent Struct ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct ParentConfig {
        #[compote(default)]
        spec: Option<FlexibleSpec>,
    }

    let json = r#"{"spec": "config.yaml"}"#;
    let config: ParentConfig =
        deserialize_json(json).expect("Should deserialize nested FlexibleSpec");
    println!("Input: {}", json);
    println!("Result: spec={:?}", config.spec);
    assert!(config.spec.is_some());
    let spec = config.spec.unwrap();
    assert_eq!(spec.file, Some("config.yaml".to_string()));

    let json = r#"{"spec": ["a", "b", "c"]}"#;
    let config: ParentConfig =
        deserialize_json(json).expect("Should deserialize nested FlexibleSpec with array");
    println!("Input: {}", json);
    println!("Result: spec={:?}", config.spec);
    assert!(config.spec.is_some());
    let spec = config.spec.unwrap();
    assert_eq!(spec.file, None);
    assert_eq!(
        spec.items,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );

    // Round-trip serialization
    println!("\n--- Round-trip Serialization ---");
    let json = r#""default.yaml""#;
    let spec: FlexibleSpec = deserialize_json(json).unwrap();
    let serialized = compote::to_json_compact(&spec).unwrap();
    println!("Input: {}", json);
    println!("Serialized: {}", serialized);
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed["file"], "default.yaml");

    println!("\n=== All flexible input examples passed! ===");
}

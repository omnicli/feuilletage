//! allow_list tests - accepting array input for HashMap/BTreeMap fields
//!
//! Tests for the allow_list attribute which allows HashMap/BTreeMap fields
//! to accept both object and array input formats.
//!
//! Array items:
//! - String items: inserted as key with V::default()
//! - Object items: key-value pairs inserted with parsed values

#![cfg(feature = "json")]

use std::collections::HashMap;

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// Test types
// ============================================================================

/// A simple tag filter enum for testing
#[derive(Debug, Clone, Default, PartialEq, DeriveConfig)]
#[compote(external_tag, rename_all = "snake_case")]
enum TagFilter {
    Contains(String),
    StartsWith(String),
    #[default]
    #[compote(variant = null)]
    Any,
}

#[derive(Debug, DeriveConfig)]
struct WithAllowList {
    #[compote(default, allow_list)]
    pub tags: HashMap<String, TagFilter>,
}

// ============================================================================
// Object input tests (standard map parsing should still work)
// ============================================================================

#[test]
fn test_allow_list_object_input() {
    let json = r#"{"tags": {"tag1": {"contains": "foo"}, "tag2": null}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept object input: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(
        cfg.tags.get("tag1"),
        Some(&TagFilter::Contains("foo".to_string()))
    );
    assert_eq!(cfg.tags.get("tag2"), Some(&TagFilter::Any));
}

// ============================================================================
// Array input tests (new with allow_list)
// ============================================================================

#[test]
fn test_allow_list_array_of_strings() {
    let json = r#"{"tags": ["tag1", "tag2", "tag3"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept array of strings: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 3);
    assert_eq!(cfg.tags.get("tag1"), Some(&TagFilter::Any));
    assert_eq!(cfg.tags.get("tag2"), Some(&TagFilter::Any));
    assert_eq!(cfg.tags.get("tag3"), Some(&TagFilter::Any));
}

#[test]
fn test_allow_list_array_of_objects() {
    let json = r#"{"tags": [{"tag1": {"contains": "foo"}}, {"tag2": {"starts_with": "bar"}}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept array of objects: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(
        cfg.tags.get("tag1"),
        Some(&TagFilter::Contains("foo".to_string()))
    );
    assert_eq!(
        cfg.tags.get("tag2"),
        Some(&TagFilter::StartsWith("bar".to_string()))
    );
}

#[test]
fn test_allow_list_mixed_array() {
    // Mix of strings (default value) and objects (parsed values)
    let json = r#"{"tags": ["tag1", {"tag2": {"contains": "foo"}}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept mixed array: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(cfg.tags.get("tag1"), Some(&TagFilter::Any));
    assert_eq!(
        cfg.tags.get("tag2"),
        Some(&TagFilter::Contains("foo".to_string()))
    );
}

// ============================================================================
// Null/empty input tests
// ============================================================================

#[test]
fn test_allow_list_null_input() {
    let json = r#"{"tags": null}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept null with default: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

#[test]
fn test_allow_list_missing_field() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should use default when missing: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

#[test]
fn test_allow_list_empty_array() {
    let json = r#"{"tags": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept empty array: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

#[test]
fn test_allow_list_empty_object() {
    let json = r#"{"tags": {}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Should accept empty object: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

// ============================================================================
// BTreeMap test
// ============================================================================

#[derive(Debug, DeriveConfig)]
struct WithAllowListBTree {
    #[compote(default, allow_list)]
    pub tags: std::collections::BTreeMap<String, TagFilter>,
}

#[test]
fn test_allow_list_btreemap_array() {
    let json = r#"{"tags": ["alpha", {"beta": {"contains": "test"}}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowListBTree>();
    assert!(
        result.is_ok(),
        "Should work with BTreeMap: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(cfg.tags.get("alpha"), Some(&TagFilter::Any));
    assert_eq!(
        cfg.tags.get("beta"),
        Some(&TagFilter::Contains("test".to_string()))
    );
}

#[test]
fn test_allow_list_btreemap_object() {
    let json = r#"{"tags": {"alpha": null, "beta": {"contains": "test"}}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowListBTree>();
    assert!(
        result.is_ok(),
        "BTreeMap should accept object input: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(cfg.tags.get("alpha"), Some(&TagFilter::Any));
    assert_eq!(
        cfg.tags.get("beta"),
        Some(&TagFilter::Contains("test".to_string()))
    );
}

// ============================================================================
// HashMap with simple value types
// ============================================================================

#[derive(Debug, DeriveConfig)]
struct WithAllowListSimple {
    #[compote(default, allow_list)]
    pub items: HashMap<String, String>,
}

#[test]
fn test_allow_list_simple_string_values() {
    // Array of strings -> each string maps to String::default() (empty string)
    let json = r#"{"items": ["key1", "key2"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowListSimple>();
    assert!(
        result.is_ok(),
        "Should work with String values: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.items.len(), 2);
    assert_eq!(cfg.items.get("key1"), Some(&String::new()));
    assert_eq!(cfg.items.get("key2"), Some(&String::new()));
}

#[test]
fn test_allow_list_simple_object_values() {
    // Object input with string values
    let json = r#"{"items": {"key1": "value1", "key2": "value2"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowListSimple>();
    assert!(
        result.is_ok(),
        "Should accept object with string values: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.items.len(), 2);
    assert_eq!(cfg.items.get("key1"), Some(&"value1".to_string()));
    assert_eq!(cfg.items.get("key2"), Some(&"value2".to_string()));
}

#[test]
fn test_allow_list_simple_mixed() {
    // Mix: string items get default, object items get parsed values
    let json = r#"{"items": ["key1", {"key2": "value2"}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowListSimple>();
    assert!(
        result.is_ok(),
        "Should accept mixed array with simple values: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.items.len(), 2);
    assert_eq!(cfg.items.get("key1"), Some(&String::new()));
    assert_eq!(cfg.items.get("key2"), Some(&"value2".to_string()));
}

// ============================================================================
// Object with multiple entries per array element
// ============================================================================

#[test]
fn test_allow_list_object_with_multiple_keys() {
    // An object item in the array can have multiple key-value pairs
    let json = r#"{"tags": [{"tag1": {"contains": "foo"}, "tag2": {"starts_with": "bar"}}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<WithAllowList>();
    assert!(
        result.is_ok(),
        "Object items can have multiple keys: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2);
    assert_eq!(
        cfg.tags.get("tag1"),
        Some(&TagFilter::Contains("foo".to_string()))
    );
    assert_eq!(
        cfg.tags.get("tag2"),
        Some(&TagFilter::StartsWith("bar".to_string()))
    );
}

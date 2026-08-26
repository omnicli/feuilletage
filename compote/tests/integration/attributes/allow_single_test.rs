//! allow_single tests - accepting scalar values for Vec fields
//!
//! Tests for the allow_single attribute which allows a single value
//! to be accepted where a Vec is expected.
//!
//! Also tests:
//! - serialize_single_as_value attribute
//! - allow_single auto-implies serialize_single_as_value for roundtrip

#![cfg(feature = "json")]

use std::collections::{BTreeSet, HashSet};

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// Basic allow_single deserialization tests
// ============================================================================

#[derive(Debug, DeriveConfig)]
struct AllowSingleConfig {
    #[compote(allow_single)]
    items: Vec<String>,
}

#[test]
fn test_allow_single_with_single_value() {
    let json = r#"{"items": "single"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleConfig>();
    assert!(result.is_ok(), "Should accept single value");

    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["single"]);
}

#[test]
fn test_allow_single_with_array() {
    let json = r#"{"items": ["a", "b", "c"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleConfig>();
    assert!(result.is_ok(), "Should accept array");

    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["a", "b", "c"]);
}

#[test]
fn test_allow_single_with_empty_array() {
    let json = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleConfig>();
    assert!(result.is_ok(), "Should accept empty array");

    let cfg = result.unwrap();
    assert!(cfg.items.is_empty());
}

// ============================================================================
// Combined allow_single with allow_map test (from integration_test.rs)
// ============================================================================

#[derive(Debug, DeriveConfig)]
struct Entry {
    id: String,
    value: i32,
}

#[derive(Debug, DeriveConfig)]
struct TestConfig {
    name: String,
    count: i32,

    #[compote(default = "default_value")]
    with_default: String,

    #[compote(allow_single)]
    items: Vec<String>,

    #[compote(allow_single, allow_map = "id")]
    entries: Vec<Entry>,

    optional: Option<String>,
}

#[test]
fn test_allow_single_feature() {
    let json = r#"{
        "name": "test",
        "count": 1,
        "items": "single"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<TestConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["single"]);
}

// ============================================================================
// serialize_single_as_value tests
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct SerializeSingleConfig {
    #[compote(serialize_single_as_value)]
    items: Vec<String>,
}

#[test]
fn test_serialize_single_as_value_empty_vec() {
    let json = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SerializeSingleConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.items, Vec::<String>::new());

    // Empty Vec should be skipped
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_serialize_single_as_value_single_item() {
    let json = r#"{"items": ["foo"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SerializeSingleConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["foo".to_string()]);

    // Single element should be serialized as just "foo", not ["foo"]
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"items":"foo"}"#);
}

#[test]
fn test_serialize_single_as_value_multiple_items() {
    let json = r#"{"items": ["foo", "bar"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SerializeSingleConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["foo".to_string(), "bar".to_string()]);

    // Multiple elements should be serialized as array
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"items":["foo","bar"]}"#);
}

// ============================================================================
// allow_single auto-implies serialize_single_as_value tests (roundtrip)
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct AllowSingleAutoConfig {
    #[compote(allow_single)]
    tags: Vec<String>,
}

#[test]
fn test_allow_single_roundtrip_single_value() {
    // Deserialize from single value
    let json = r#"{"tags": "only-one"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleAutoConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.tags, vec!["only-one".to_string()]);

    // Should serialize back as single value (not array)
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"tags":"only-one"}"#);
}

#[test]
fn test_allow_single_roundtrip_array() {
    // Deserialize from array
    let json = r#"{"tags": ["a", "b", "c"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleAutoConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(
        cfg.tags,
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );

    // Should serialize back as array
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"tags":["a","b","c"]}"#);
}

#[test]
fn test_allow_single_roundtrip_empty() {
    // Deserialize from empty array
    let json = r#"{"tags": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AllowSingleAutoConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());

    // Empty should be omitted
    let serialized = compote::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

// ============================================================================
// Generic collection support (BTreeSet, HashSet)
// ============================================================================

#[derive(Debug, DeriveConfig)]
struct BTreeSetConfig {
    #[compote(allow_single, default)]
    tags: BTreeSet<String>,
}

#[test]
fn test_allow_single_btreeset_single_value() {
    let json = r#"{"tags": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetConfig>();
    assert!(result.is_ok(), "Should accept single value for BTreeSet");

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 1);
    assert!(cfg.tags.contains("hello"));
}

#[test]
fn test_allow_single_btreeset_array() {
    let json = r#"{"tags": ["a", "b", "c"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetConfig>();
    assert!(result.is_ok(), "Should accept array for BTreeSet");

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 3);
    assert!(cfg.tags.contains("a"));
    assert!(cfg.tags.contains("b"));
    assert!(cfg.tags.contains("c"));
}

#[test]
fn test_allow_single_btreeset_dedup() {
    let json = r#"{"tags": ["a", "b", "a"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 2, "BTreeSet should deduplicate");
}

#[test]
fn test_allow_single_btreeset_missing_field() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetConfig>();
    assert!(result.is_ok(), "Missing field should use default");

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

#[test]
fn test_allow_single_btreeset_null_field() {
    let json = r#"{"tags": null}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetConfig>();
    assert!(result.is_ok(), "Null field should use default");

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

#[derive(Debug, DeriveConfig)]
struct HashSetConfig {
    #[compote(allow_single, default)]
    tags: HashSet<String>,
}

#[test]
fn test_allow_single_hashset_single_value() {
    let json = r#"{"tags": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<HashSetConfig>();
    assert!(result.is_ok(), "Should accept single value for HashSet");

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 1);
    assert!(cfg.tags.contains("hello"));
}

#[test]
fn test_allow_single_hashset_array() {
    let json = r#"{"tags": ["x", "y", "z"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<HashSetConfig>();
    assert!(result.is_ok(), "Should accept array for HashSet");

    let cfg = result.unwrap();
    assert_eq!(cfg.tags.len(), 3);
    assert!(cfg.tags.contains("x"));
    assert!(cfg.tags.contains("y"));
    assert!(cfg.tags.contains("z"));
}

#[test]
fn test_allow_single_hashset_missing_field() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<HashSetConfig>();
    assert!(result.is_ok(), "Missing field should use default");

    let cfg = result.unwrap();
    assert!(cfg.tags.is_empty());
}

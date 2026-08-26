//! Basic config loading and deserialization tests.
//!
//! Tests for:
//! - Basic deserialization
//! - Nested merge behavior
//! - Multi-format loading (JSON, YAML, TOML)
//! - Null vs missing field handling

#![cfg(feature = "json")]

use feuilletage::{Config, Context, ContextValue, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
struct TestConfig {
    name: String,
    count: i32,

    #[feuilletage(default = "default_value")]
    with_default: String,

    #[feuilletage(allow_single)]
    items: Vec<String>,

    #[feuilletage(allow_single, allow_map = "id")]
    entries: Vec<Entry>,

    optional: Option<String>,
}

#[derive(Debug, DeriveConfig)]
struct Entry {
    id: String,
    value: i32,
}

#[test]
fn test_basic_deserialization() {
    let json = r#"{
        "name": "test",
        "count": 42,
        "items": ["a", "b", "c"]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<TestConfig>();
    assert!(result.is_ok(), "Deserialization should succeed");

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    assert_eq!(cfg.count, 42);
    assert_eq!(cfg.with_default, "default_value");
    assert_eq!(cfg.items, vec!["a", "b", "c"]);
}

#[test]
fn test_null_vs_missing() {
    // Test with explicit null
    let json1 = r#"{
        "name": "test",
        "count": 1,
        "optional": null
    }"#;

    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));

    let result1 = config1.deserialize::<TestConfig>();
    assert!(result1.is_ok());
    let cfg1 = result1.unwrap();
    assert_eq!(cfg1.optional, None);

    // Test with missing field
    let json2 = r#"{
        "name": "test",
        "count": 1
    }"#;

    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));

    let result2 = config2.deserialize::<TestConfig>();
    assert!(result2.is_ok());
    let cfg2 = result2.unwrap();
    assert_eq!(cfg2.optional, None);

    // Test with actual value
    let json3 = r#"{
        "name": "test",
        "count": 1,
        "optional": "value"
    }"#;

    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));

    let result3 = config3.deserialize::<TestConfig>();
    assert!(result3.is_ok());
    let cfg3 = result3.unwrap();
    assert_eq!(cfg3.optional, Some("value".to_string()));
}

#[test]
fn test_nested_merge() {
    let mut config = Config::default();

    let json1 = r#"{
        "a": {
            "b": {
                "c": 1,
                "d": 2
            },
            "e": 3
        }
    }"#;
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));

    let json2 = r#"{
        "a": {
            "b": {
                "c": 10,
                "f": 4
            }
        }
    }"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    // Should have: a.b.c=10 (overridden), a.b.d=2 (preserved), a.b.f=4 (added), a.e=3 (preserved)
    if let ContextValue::Object(map, _) = config.root() {
        if let Some(a) = map.get("a") {
            if let ContextValue::Object(a_map, _) = a {
                // Check a.e
                if let Some(e) = a_map.get("e") {
                    if let ContextValue::Int(i, _) = e {
                        assert_eq!(*i, 3, "a.e should be preserved");
                    }
                }

                // Check a.b
                if let Some(b) = a_map.get("b") {
                    if let ContextValue::Object(b_map, _) = b {
                        assert_eq!(b_map.len(), 3, "a.b should have 3 fields");

                        if let Some(c) = b_map.get("c") {
                            if let ContextValue::Int(i, _) = c {
                                assert_eq!(*i, 10, "a.b.c should be overridden");
                            }
                        }
                        if let Some(d) = b_map.get("d") {
                            if let ContextValue::Int(i, _) = d {
                                assert_eq!(*i, 2, "a.b.d should be preserved");
                            }
                        }
                        if let Some(f) = b_map.get("f") {
                            if let ContextValue::Int(i, _) = f {
                                assert_eq!(*i, 4, "a.b.f should be added");
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(all(feature = "yaml", feature = "toml"))]
#[test]
fn test_multi_format_loading() {
    // JSON
    let mut config1 = Config::default();
    config1.load_json(
        r#"{"name": "json", "value": 42}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    assert!(!config1.has_errors());

    // YAML
    let mut config2 = Config::default();
    config2.load_yaml(
        "name: yaml\nvalue: 42",
        Context::new(Source::Programmatic, Level::User),
    );
    assert!(!config2.has_errors());

    // TOML
    let mut config3 = Config::default();
    config3.load_toml(
        "name = \"toml\"\nvalue = 42",
        Context::new(Source::Programmatic, Level::User),
    );
    assert!(!config3.has_errors());
}

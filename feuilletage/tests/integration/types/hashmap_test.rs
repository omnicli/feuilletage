//! HashMap field tests
//!
//! Tests for using HashMap<K, V> fields in Config derive macro.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;
use std::collections::HashMap;

#[derive(Debug, DeriveConfig, PartialEq)]
struct ConfigWithHashMap {
    name: String,
    #[feuilletage(default)]
    env_vars: HashMap<String, String>,
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct ConfigWithRequiredHashMap {
    env_vars: HashMap<String, String>,
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct ConfigWithIntValues {
    #[feuilletage(default)]
    counts: HashMap<String, i64>,
}

#[test]
fn test_hashmap_field_basic() {
    let json = r#"{
        "name": "myapp",
        "env_vars": {
            "PATH": "/usr/bin",
            "HOME": "/home/user"
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithHashMap>();
    assert!(
        result.is_ok(),
        "Should successfully deserialize HashMap field: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "myapp");
    assert_eq!(cfg.env_vars.len(), 2);
    assert_eq!(cfg.env_vars.get("PATH"), Some(&"/usr/bin".to_string()));
    assert_eq!(cfg.env_vars.get("HOME"), Some(&"/home/user".to_string()));
}

#[test]
fn test_hashmap_field_empty() {
    let json = r#"{
        "name": "myapp",
        "env_vars": {}
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithHashMap>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.env_vars.is_empty());
}

#[test]
fn test_hashmap_field_default_when_missing() {
    let json = r#"{"name": "myapp"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithHashMap>();
    assert!(
        result.is_ok(),
        "Should use default empty HashMap when field is missing"
    );

    let cfg = result.unwrap();
    assert!(cfg.env_vars.is_empty());
}

#[test]
fn test_hashmap_required_field() {
    let json = r#"{
        "env_vars": {
            "KEY": "value"
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithRequiredHashMap>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.env_vars.get("KEY"), Some(&"value".to_string()));
}

#[test]
fn test_hashmap_required_field_missing() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithRequiredHashMap>();
    assert!(
        result.is_err(),
        "Should fail when required HashMap field is missing"
    );
}

#[test]
fn test_hashmap_with_integer_values() {
    let json = r#"{
        "counts": {
            "apples": 10,
            "oranges": 5,
            "bananas": 20
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithIntValues>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.counts.get("apples"), Some(&10));
    assert_eq!(cfg.counts.get("oranges"), Some(&5));
    assert_eq!(cfg.counts.get("bananas"), Some(&20));
}

#[test]
fn test_hashmap_merge() {
    // Test that HashMaps can be merged from multiple config sources
    let json1 = r#"{
        "name": "myapp",
        "env_vars": {
            "A": "1",
            "B": "2"
        }
    }"#;

    let json2 = r#"{
        "env_vars": {
            "B": "override",
            "C": "3"
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithHashMap>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.env_vars.get("A"), Some(&"1".to_string()));
    assert_eq!(cfg.env_vars.get("B"), Some(&"override".to_string()));
    assert_eq!(cfg.env_vars.get("C"), Some(&"3".to_string()));
}

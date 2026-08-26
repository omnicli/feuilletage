//! Vec advanced attributes tests
//!
//! Tests for Vec fields with various attribute combinations:
//! allow_single, allow_map, default, mutable_by, transform_each, aliases

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// Helper to create config and deserialize
fn deserialize_json<T: feuilletage::FromContextValue>(json: &str) -> Result<T, feuilletage::Error> {
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

// === DEFAULT TESTS ===

#[test]
fn test_vec_allow_single_with_explicit_default() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, default = "vec![\"default\".to_string()]")]
        items: Vec<String>,
    }

    let json = r#"{}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["default".to_string()]);
}

#[test]
fn test_vec_allow_single_with_explicit_default_overridden() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, default = "vec![\"default\".to_string()]")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["a", "b"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_vec_allow_single_with_null_uses_default() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, default = "vec![\"default\".to_string()]")]
        items: Vec<String>,
    }

    let json = r#"{"items": null}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["default".to_string()]);
}

#[test]
fn test_vec_allow_map_with_default() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct Entry {
        id: String,
        #[feuilletage(default = "0")]
        value: i32,
    }

    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_map = "id", default = "Vec::new()")]
        entries: Vec<Entry>,
    }

    let json = r#"{}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert!(config.entries.is_empty());
}

// === MUTABLE_BY TESTS ===

#[test]
fn test_vec_allow_single_with_mutable_by_allowed() {
    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_single, mutable_by = ["user"])]
        items: Vec<String>,
    }

    let json = r#"{"items": ["a", "b"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TestConfig>();
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.items, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_vec_allow_single_with_mutable_by_denied() {
    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_single, mutable_by = ["system"])]
        items: Vec<String>,
    }

    let json = r#"{"items": ["a", "b"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TestConfig>();
    assert!(result.is_err());
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("can only be set by levels"),
        "Expected mutable_by error, got: {}",
        err_str
    );
}

#[test]
fn test_vec_allow_map_with_mutable_by_allowed() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct Entry {
        id: String,
        #[feuilletage(default = "0")]
        value: i32,
    }

    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_map = "id", mutable_by = ["local"])]
        entries: Vec<Entry>,
    }

    let json = r#"{"entries": {"foo": {"value": 1}}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::Local));
    let result = config.deserialize::<TestConfig>();
    assert!(result.is_ok());
}

#[test]
fn test_vec_allow_map_with_mutable_by_denied() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct Entry {
        id: String,
        #[feuilletage(default = "0")]
        value: i32,
    }

    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_map = "id", mutable_by = ["local"])]
        entries: Vec<Entry>,
    }

    let json = r#"{"entries": {"foo": {"value": 1}}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TestConfig>();
    assert!(result.is_err());
}

// === TRANSFORM_EACH TESTS ===

#[test]
fn test_vec_allow_single_with_transform_each() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "to_uppercase")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["hello", "world"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["HELLO".to_string(), "WORLD".to_string()]);
}

#[test]
fn test_vec_allow_single_single_value_with_transform_each() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "to_uppercase")]
        items: Vec<String>,
    }

    let json = r#"{"items": "hello"}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["HELLO".to_string()]);
}

#[test]
fn test_vec_allow_single_with_transform_each_to_lowercase() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "to_lowercase")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["HELLO", "WORLD"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["hello".to_string(), "world".to_string()]);
}

#[test]
fn test_vec_allow_single_with_transform_each_trim() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "trim")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["  hello  ", "  world  "]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["hello".to_string(), "world".to_string()]);
}

// === COMBINATION TESTS ===

#[test]
fn test_vec_all_attributes_combined() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(
            allow_single,
            transform_each = "to_uppercase",
            mutable_by = ["user"],
            default = "vec![\"DEFAULT\".to_string()]"
        )]
        items: Vec<String>,
    }

    // Test with value
    let json = r#"{"items": "hello"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TestConfig>().unwrap();
    assert_eq!(result.items, vec!["HELLO".to_string()]);

    // Test without value (uses default)
    let json = r#"{}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["DEFAULT".to_string()]);
}

#[test]
fn test_vec_transform_each_with_default_on_missing() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(
            allow_single,
            transform_each = "to_uppercase",
            default = "vec![\"a\".to_string(), \"b\".to_string()]"
        )]
        items: Vec<String>,
    }

    // Missing field uses default (transform_each is not applied to default)
    let json = r#"{}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn test_vec_allow_map_with_entries() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct Entry {
        id: String,
        name: String,
    }

    #[derive(Debug, DeriveConfig)]
    struct TestConfig {
        #[feuilletage(allow_map = "id")]
        entries: Vec<Entry>,
    }

    let json = r#"{"entries": {"foo": {"name": "hello"}, "bar": {"name": "world"}}}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.entries.len(), 2);
    // Check that entries have their id field injected
    let ids: Vec<_> = config.entries.iter().map(|e| e.id.clone()).collect();
    assert!(ids.contains(&"foo".to_string()));
    assert!(ids.contains(&"bar".to_string()));
}

// === ALIAS TESTS WITH VEC ===

#[test]
fn test_vec_allow_single_with_alias() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, aliases = ["things"])]
        items: Vec<String>,
    }

    // Test with primary name
    let json = r#"{"items": ["a", "b"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["a".to_string(), "b".to_string()]);

    // Test with alias
    let json = r#"{"things": ["c", "d"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.items, vec!["c".to_string(), "d".to_string()]);
}

// === DURATION SHORTCUT WITH VEC ===

#[test]
fn test_vec_allow_single_with_duration_transform_each() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "parse_duration")]
        timeouts: Vec<i64>,
    }

    // parse_duration transform on each item converts string to i64
    let json = r#"{"timeouts": ["30s", "1m", "2h"]}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.timeouts, vec![30, 60, 7200]);
}

#[test]
fn test_vec_allow_single_single_duration_with_transform_each() {
    #[derive(Debug, DeriveConfig, PartialEq)]
    struct TestConfig {
        #[feuilletage(allow_single, transform_each = "parse_duration")]
        timeouts: Vec<i64>,
    }

    // Single value should also work with transform_each
    let json = r#"{"timeouts": "5m"}"#;
    let config: TestConfig = deserialize_json(json).unwrap();
    assert_eq!(config.timeouts, vec![300]);
}

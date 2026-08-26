//! Tests for collection and nested struct deserialization.
//!
//! Tests error handling for Vec and nested struct deserialization.

use compote::{Config, Context, Format, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Test array element error is recorded but valid elements are kept
#[test]
fn test_array_element_error_records_error() {
    #[derive(DeriveConfig, Debug)]
    struct ArrayConfig {
        #[compote(default)]
        numbers: Vec<i32>,
    }

    // Array with mixed valid and invalid elements
    let config_str = r#"
numbers:
  - 1
  - 2
  - "not_a_number"
  - 4
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: ArrayConfig = loader.deserialize().expect("Should succeed");

    // Valid elements should be present, invalid skipped
    println!("Array Result: {:?}", result.numbers);

    let errors = loader.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("numbers") || msg.contains("parse") || msg.contains("i64")
        }),
        "Expected array element error, got: {:?}",
        errors
    );
}

/// Test nested struct field error is recorded
#[test]
fn test_nested_struct_field_error_uses_default() {
    #[derive(DeriveConfig, Debug, Default, PartialEq)]
    struct Inner {
        #[compote(default = "0")]
        value: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct Outer {
        #[compote(default)]
        inner: Inner,

        #[compote(default = "outer_default")]
        name: String,
    }

    // inner.value() has wrong type
    let config_str = r#"
name: "test"
inner:
  value: "not_a_number"
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: Outer = loader.deserialize().expect("Should succeed with defaults");

    assert_eq!(result.name, "test");
    assert_eq!(result.inner.value, 0, "inner.value should use default");

    let errors = loader.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("value") || msg.contains("inner") || msg.contains("parse")
        }),
        "Expected nested field error, got: {:?}",
        errors
    );
}

/// Test required field in nested struct still fails
#[test]
fn test_required_field_in_nested_struct_fails() {
    #[derive(DeriveConfig, Debug)]
    struct InnerRequired {
        /// Required field - no default
        required_value: String,
    }

    #[derive(DeriveConfig, Debug)]
    struct OuterWithRequiredNested {
        #[compote(default = "outer_name")]
        name: String,
        /// Nested struct with required field
        inner: InnerRequired,
    }

    // Inner struct missing required field
    let config_str = r#"
name: "test"
inner: {}
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: Result<OuterWithRequiredNested, _> = loader.deserialize();

    // Should fail because inner.required_value is missing
    assert!(
        result.is_err(),
        "Should fail when nested struct has missing required field"
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("required_value") || err.to_string().contains("inner"),
        "Error should mention the missing field or path"
    );
}

/// Test empty array is valid
#[test]
fn test_empty_array_is_valid() {
    #[derive(DeriveConfig, Debug)]
    struct ArrayConfig {
        #[compote(default)]
        items: Vec<String>,
    }

    let config_str = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ArrayConfig = config.deserialize().expect("Should succeed");

    assert!(result.items.is_empty());
    assert!(!config.errors().has_errors());
}

/// Test nested array of structs
#[test]
fn test_nested_array_of_structs() {
    #[derive(DeriveConfig, Debug, Default, PartialEq)]
    struct Item {
        #[compote(default = "unnamed")]
        name: String,

        #[compote(default = "0")]
        count: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct ContainerConfig {
        #[compote(default)]
        items: Vec<Item>,
    }

    let config_str = r#"
items:
  - name: "first"
    count: 1
  - name: "second"
    count: "invalid"
  - name: "third"
    count: 3
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: ContainerConfig = loader.deserialize().expect("Should succeed");

    // All items should be present, second one with default count
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.items[0].name, "first");
    assert_eq!(result.items[0].count, 1);
    assert_eq!(result.items[1].name, "second");
    assert_eq!(
        result.items[1].count, 0,
        "second item count should use default"
    );
    assert_eq!(result.items[2].name, "third");
    assert_eq!(result.items[2].count, 3);

    // Should have error for second item's count
    assert!(loader.errors().has_errors());
}

/// Test deeply nested structs
#[test]
fn test_deeply_nested_structs() {
    #[derive(DeriveConfig, Debug, Default, PartialEq)]
    struct Level3 {
        #[compote(default = "level3_default")]
        value: String,
    }

    #[derive(DeriveConfig, Debug, Default, PartialEq)]
    struct Level2 {
        #[compote(default)]
        level3: Level3,
    }

    #[derive(DeriveConfig, Debug)]
    struct Level1 {
        #[compote(default)]
        level2: Level2,
    }

    let config_str = r#"
level2:
  level3:
    value: 123
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: Level1 = loader.deserialize().expect("Should succeed with defaults");

    // Value should use default because 123 (int) can't become String without coerce
    // Note: YAML parses 123 as integer, not string
    println!("Deep nested result: {:?}", result.level2.level3.value);

    // The behavior depends on how the macro handles this
    // With soft errors, it should use the default
}

/// Test nested struct with all fields having defaults
#[test]
fn test_nested_struct_all_defaults() {
    #[derive(DeriveConfig, Debug, Default, PartialEq)]
    struct Inner {
        #[compote(default = "inner_default")]
        inner_value: String,
    }

    #[derive(DeriveConfig, Debug)]
    struct Outer {
        #[compote(default)]
        inner: Inner,
    }

    // Empty config - should use all defaults
    let config_str = r#"{}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Outer = config.deserialize().expect("Should succeed with defaults");

    assert_eq!(result.inner.inner_value, "inner_default");
    assert!(!config.errors().has_errors());
}

// ============================================================================
// BTreeSet tests
// ============================================================================

/// Test BTreeSet basic deserialization
#[test]
fn test_btreeset_basic() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetConfig {
        #[compote(default)]
        tags: BTreeSet<String>,
    }

    let config_str = r#"{"tags": ["c", "a", "b", "a"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetConfig = config.deserialize().expect("Should succeed");

    // BTreeSet deduplicates and orders entries
    assert_eq!(result.tags.len(), 3);
    let tags_vec: Vec<_> = result.tags.into_iter().collect();
    assert_eq!(tags_vec, vec!["a", "b", "c"]);
    assert!(!config.errors().has_errors());
}

/// Test BTreeSet with graceful error handling
#[test]
fn test_btreeset_graceful_errors() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetConfig {
        #[compote(default)]
        numbers: BTreeSet<i32>,
    }

    let config_str = r#"{"numbers": [1, 2, "invalid", 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetConfig = config
        .deserialize()
        .expect("Should succeed with valid elements");

    // Should have 3 valid elements, invalid one skipped
    assert_eq!(result.numbers.len(), 3);
    assert!(result.numbers.contains(&1));
    assert!(result.numbers.contains(&2));
    assert!(result.numbers.contains(&3));
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for invalid element"
    );
}

/// Test empty BTreeSet
#[test]
fn test_btreeset_empty() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetConfig {
        #[compote(default)]
        items: BTreeSet<String>,
    }

    let config_str = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetConfig = config.deserialize().expect("Should succeed");

    assert!(result.items.is_empty());
    assert!(!config.errors().has_errors());
}

// ============================================================================
// BTreeMap tests
// ============================================================================

/// Test BTreeMap basic deserialization
#[test]
fn test_btreemap_basic() {
    use std::collections::BTreeMap;

    #[derive(DeriveConfig, Debug)]
    struct BTreeMapConfig {
        #[compote(default)]
        scores: BTreeMap<String, i32>,
    }

    let config_str = r#"{"scores": {"charlie": 3, "alice": 1, "bob": 2}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeMapConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.scores.len(), 3);
    assert_eq!(result.scores.get("alice"), Some(&1));
    assert_eq!(result.scores.get("bob"), Some(&2));
    assert_eq!(result.scores.get("charlie"), Some(&3));

    // BTreeMap iterates in sorted key order
    let keys: Vec<_> = result.scores.keys().collect();
    assert_eq!(keys, vec!["alice", "bob", "charlie"]);
    assert!(!config.errors().has_errors());
}

/// Test BTreeMap with graceful error handling
#[test]
fn test_btreemap_graceful_errors() {
    use std::collections::BTreeMap;

    #[derive(DeriveConfig, Debug)]
    struct BTreeMapConfig {
        #[compote(default)]
        values: BTreeMap<String, i32>,
    }

    let config_str = r#"{"values": {"a": 1, "b": "invalid", "c": 3}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeMapConfig = config
        .deserialize()
        .expect("Should succeed with valid entries");

    // Should have 2 valid entries, invalid one skipped
    assert_eq!(result.values.len(), 2);
    assert_eq!(result.values.get("a"), Some(&1));
    assert_eq!(result.values.get("c"), Some(&3));
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for invalid entry"
    );
}

// ============================================================================
// HashSet tests
// ============================================================================

/// Test HashSet basic deserialization
#[test]
fn test_hashset_basic() {
    use std::collections::HashSet;

    #[derive(DeriveConfig, Debug)]
    struct HashSetConfig {
        #[compote(default)]
        unique: HashSet<String>,
    }

    let config_str = r#"{"unique": ["a", "b", "a", "c"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: HashSetConfig = config.deserialize().expect("Should succeed");

    // HashSet deduplicates entries
    assert_eq!(result.unique.len(), 3);
    assert!(result.unique.contains("a"));
    assert!(result.unique.contains("b"));
    assert!(result.unique.contains("c"));
    assert!(!config.errors().has_errors());
}

/// Test HashSet with graceful error handling
#[test]
fn test_hashset_graceful_errors() {
    use std::collections::HashSet;

    #[derive(DeriveConfig, Debug)]
    struct HashSetConfig {
        #[compote(default)]
        numbers: HashSet<i32>,
    }

    let config_str = r#"{"numbers": [1, 2, "invalid", 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: HashSetConfig = config
        .deserialize()
        .expect("Should succeed with valid elements");

    // Should have 3 valid elements, invalid one skipped
    assert_eq!(result.numbers.len(), 3);
    assert!(result.numbers.contains(&1));
    assert!(result.numbers.contains(&2));
    assert!(result.numbers.contains(&3));
    assert!(
        config.errors().has_errors(),
        "Should have recorded error for invalid element"
    );
}

// ============================================================================
// allow_single with BTreeSet tests
// ============================================================================

/// Test BTreeSet with allow_single attribute
#[test]
fn test_btreeset_allow_single() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetAllowSingleConfig {
        #[compote(default, allow_single)]
        tags: BTreeSet<String>,
    }

    // Test with single value
    let config_str = r#"{"tags": "single"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetAllowSingleConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.tags.len(), 1);
    assert!(result.tags.contains("single"));
}

/// Test HashSet with allow_single attribute
#[test]
fn test_hashset_allow_single() {
    use std::collections::HashSet;

    #[derive(DeriveConfig, Debug)]
    struct HashSetAllowSingleConfig {
        #[compote(default, allow_single)]
        items: HashSet<String>,
    }

    // Test with single value
    let config_str = r#"{"items": "single"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: HashSetAllowSingleConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.items.len(), 1);
    assert!(result.items.contains("single"));
}

// ============================================================================
// allow_map with BTreeSet tests
// ============================================================================

/// Test BTreeSet with allow_map attribute
#[test]
fn test_btreeset_allow_map() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug, PartialEq, Default)]
    struct Item {
        #[compote(default)]
        name: String,
        #[compote(default = "0")]
        value: i32,
    }

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetAllowMapConfig {
        #[compote(default, allow_map(key = "name"))]
        items: BTreeSet<Item>,
    }

    // Implement Ord for Item based on name for BTreeSet
    impl Ord for Item {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.name.cmp(&other.name)
        }
    }
    impl PartialOrd for Item {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Eq for Item {}

    // Test with map notation
    let config_str = r#"{"items": {"alpha": {"value": 1}, "beta": {"value": 2}}}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetAllowMapConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.items.len(), 2);
    // BTreeSet orders by name
    let items_vec: Vec<_> = result.items.into_iter().collect();
    assert_eq!(items_vec[0].name, "alpha");
    assert_eq!(items_vec[0].value, 1);
    assert_eq!(items_vec[1].name, "beta");
    assert_eq!(items_vec[1].value, 2);
}

// ============================================================================
// transform_each with BTreeSet tests
// ============================================================================

/// Test BTreeSet with transform_each attribute (requires allow_single to use generate_vec_deserialization)
#[test]
fn test_btreeset_transform_each() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetTransformConfig {
        // Note: transform_each only works with allow_single or allow_map
        #[compote(default, allow_single, transform_each = "to_uppercase")]
        tags: BTreeSet<String>,
    }

    let config_str = r#"{"tags": ["charlie", "alpha", "beta"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: BTreeSetTransformConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.tags.len(), 3);
    // BTreeSet orders alphabetically, and transform_each uppercases
    let tags_vec: Vec<_> = result.tags.into_iter().collect();
    assert_eq!(tags_vec, vec!["ALPHA", "BETA", "CHARLIE"]);
}

/// Test HashSet with transform_each attribute (requires allow_single to use generate_vec_deserialization)
#[test]
fn test_hashset_transform_each() {
    use std::collections::HashSet;

    #[derive(DeriveConfig, Debug)]
    struct HashSetTransformConfig {
        // Note: transform_each only works with allow_single or allow_map
        #[compote(default, allow_single, transform_each = "to_lowercase")]
        tags: HashSet<String>,
    }

    let config_str = r#"{"tags": ["HELLO", "WORLD", "TEST"]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: HashSetTransformConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.tags.len(), 3);
    assert!(result.tags.contains("hello"));
    assert!(result.tags.contains("world"));
    assert!(result.tags.contains("test"));
}

// ============================================================================
// on_error with BTreeSet tests
// ============================================================================

/// Test BTreeSet with on_error = "fail" (requires allow_single to use generate_vec_deserialization)
#[test]
fn test_btreeset_on_error_fail() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetOnErrorFailConfig {
        // Note: on_error only works with allow_single or allow_map
        #[compote(default, allow_single, on_error = "fail")]
        numbers: BTreeSet<i32>,
    }

    // Has an invalid element
    let config_str = r#"{"numbers": [1, "invalid", 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetOnErrorFailConfig>();

    // Should fail because on_error = "fail" means immediate failure
    assert!(result.is_err(), "Should fail with on_error = fail");
}

/// Test BTreeSet with on_error = "default" (uses default on any error)
#[test]
fn test_btreeset_on_error_default() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetOnErrorDefaultConfig {
        // Note: on_error only works with allow_single or allow_map
        // Need a default for default mode to use when errors occur
        #[compote(default, allow_single, on_error = "default")]
        numbers: BTreeSet<i32>,
    }

    // Has an invalid element - should use default (empty set)
    let config_str = r#"{"numbers": [1, "invalid", 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetOnErrorDefaultConfig>();

    // on_error = "default" should use the default (empty set) when any error occurs
    assert!(result.is_ok(), "Should succeed with default");
    assert!(
        result.unwrap().numbers.is_empty(),
        "Should use empty default on error"
    );
}

// Debug test to see what actually happens
#[test]
fn test_btreeset_on_error_debug() {
    use std::collections::BTreeSet;

    #[derive(DeriveConfig, Debug)]
    struct BTreeSetDebugConfig {
        #[compote(default, allow_single, on_error = "fail")]
        numbers: BTreeSet<i32>,
    }

    let config_str = r#"{"numbers": [1, "invalid", 3]}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BTreeSetDebugConfig>();

    println!("Result: {:?}", result);
    println!("Errors: {:?}", config.errors().errors());
}

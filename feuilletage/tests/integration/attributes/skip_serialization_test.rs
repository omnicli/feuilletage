//! Skip serialization tests - skip, skip_if_empty, skip_if_empty_recursive, skip_if_default

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// skip (unconditional) tests
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipUnconditional {
    name: String,

    #[feuilletage(skip, default = "0")]
    internal_state: i32,

    count: i32,
}

#[test]
fn test_skip_unconditional_always_excluded() {
    // Skip fields are not deserialized - they always use the default value
    // This matches serde's behavior: #[serde(skip)] skips both serialization AND deserialization
    let json = r#"{"name": "test", "internal_state": 42, "count": 10}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipUnconditional>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    assert_eq!(cfg.internal_state, 0); // Uses default value (not deserialized from input)
    assert_eq!(cfg.count, 10);

    // Test serialization - internal_state should NEVER be included
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"name":"test","count":10}"#);
    assert!(
        !serialized.contains("internal_state"),
        "skip field should never be serialized"
    );
}

// ============================================================================
// skip_if_empty tests
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfEmptyVec {
    #[feuilletage(skip_if_empty)]
    items: Vec<String>,
}

#[test]
fn test_skip_if_empty_vec_empty() {
    let json = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyVec>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.items.is_empty());

    // Should be skipped because it's empty
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_skip_if_empty_vec_with_values() {
    let json = r#"{"items": ["a", "b"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyVec>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // Should NOT be skipped because it has values
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"items":["a","b"]}"#);
}

// ============================================================================
// skip_if_default tests
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfDefaultBool {
    #[feuilletage(default = "true", skip_if_default)]
    enabled: bool,
}

#[test]
fn test_skip_if_default_bool_matches_default() {
    let json = r#"{"enabled": true}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfDefaultBool>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.enabled);

    // Test serialization - should skip because value matches configured default (true)
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_skip_if_default_bool_differs_from_default() {
    let json = r#"{"enabled": false}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfDefaultBool>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(!cfg.enabled);

    // Test serialization - should NOT skip because value differs from configured default
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"enabled":false}"#);
}

// ============================================================================
// skip_if (custom function) tests
// ============================================================================

// Custom skip function - returns true if the string is empty
fn is_empty_string(s: &String) -> bool {
    s.is_empty()
}

// Custom skip function - returns true if value is zero
fn is_zero(n: &i32) -> bool {
    *n == 0
}

// Custom skip function - returns true if all strings in vec are empty
fn all_empty(v: &Vec<String>) -> bool {
    v.iter().all(|s| s.is_empty())
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfCustomString {
    #[feuilletage(skip_if = "is_empty_string", default = "")]
    optional_name: String,

    required_field: String,
}

#[test]
fn test_skip_if_custom_function_empty_string() {
    let json = r#"{"optional_name": "", "required_field": "required"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomString>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.optional_name, "");
    assert_eq!(cfg.required_field, "required");

    // Empty string should be skipped
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"required_field":"required"}"#);
    assert!(!serialized.contains("optional_name"));
}

#[test]
fn test_skip_if_custom_function_non_empty_string() {
    let json = r#"{"optional_name": "has_value", "required_field": "required"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomString>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // Non-empty string should NOT be skipped
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(
        serialized,
        r#"{"optional_name":"has_value","required_field":"required"}"#
    );
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfCustomInt {
    #[feuilletage(skip_if = "is_zero", default = "0")]
    count: i32,
}

#[test]
fn test_skip_if_custom_function_zero_int() {
    let json = r#"{"count": 0}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomInt>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.count, 0);

    // Zero should be skipped
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_skip_if_custom_function_non_zero_int() {
    let json = r#"{"count": 42}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomInt>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.count, 42);

    // Non-zero should NOT be skipped
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"count":42}"#);
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfCustomVec {
    #[feuilletage(skip_if = "all_empty")]
    items: Vec<String>,
}

#[test]
fn test_skip_if_custom_function_all_empty_vec() {
    let json = r#"{"items": ["", "", ""]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomVec>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // All empty strings should cause skip
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_skip_if_custom_function_some_non_empty_vec() {
    let json = r#"{"items": ["", "value", ""]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfCustomVec>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // Has non-empty string, should NOT be skipped
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert!(serialized.contains("items"));
}

// ============================================================================
// skip_if_empty_recursive tests
// ============================================================================

// Note: skip_if_empty_recursive for nested structs requires the nested type to implement
// the IsEmpty trait. When implemented, empty nested structs are correctly skipped.

#[derive(Debug, DeriveConfig, PartialEq)]
struct InnerWithVec {
    items: Vec<String>,
}

impl feuilletage::IsEmpty for InnerWithVec {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfEmptyRecursiveNested {
    #[feuilletage(skip_if_empty_recursive)]
    nested: InnerWithVec,
}

#[test]
fn test_skip_if_empty_recursive_nested_vec_empty() {
    let json = r#"{"nested": {"items": []}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveNested>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.nested.items.is_empty());

    // With IsEmpty implemented on InnerWithVec, skip_if_empty_recursive correctly
    // skips the nested field when InnerWithVec.is_empty() returns true
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert!(!serialized.contains("nested"));
    assert!(!serialized.contains("items"));
}

#[test]
fn test_skip_if_empty_recursive_nested_vec_with_values() {
    let json = r#"{"nested": {"items": ["a", "b"]}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveNested>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // Should NOT skip because nested.items has values
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert!(serialized.contains("nested"));
    assert!(serialized.contains("items"));
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct InnerWithOption {
    value: Option<String>,
}

impl feuilletage::IsEmpty for InnerWithOption {
    fn is_empty(&self) -> bool {
        self.value.is_none()
    }
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfEmptyRecursiveOption {
    #[feuilletage(skip_if_empty_recursive)]
    nested: InnerWithOption,
}

#[test]
fn test_skip_if_empty_recursive_option_none() {
    let json = r#"{"nested": {}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveOption>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert!(cfg.nested.value.is_none());

    // With IsEmpty implemented on InnerWithOption, skip_if_empty_recursive correctly
    // skips the nested field when InnerWithOption.is_empty() returns true
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert!(!serialized.contains("nested"));
}

#[test]
fn test_skip_if_empty_recursive_option_some() {
    let json = r#"{"nested": {"value": "present"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveOption>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.nested.value, Some("present".to_string()));

    // Should NOT skip because nested.value has a value
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert!(serialized.contains("nested"));
    assert!(serialized.contains("value"));
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct SkipIfEmptyRecursiveString {
    #[feuilletage(skip_if_empty_recursive)]
    text: String,
}

#[test]
fn test_skip_if_empty_recursive_empty_string() {
    let json = r#"{"text": ""}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveString>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.text, "");

    // Should skip because string is empty
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, "{}");
}

#[test]
fn test_skip_if_empty_recursive_non_empty_string() {
    let json = r#"{"text": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipIfEmptyRecursiveString>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    // Should NOT skip because string has content
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"text":"hello"}"#);
}

// ============================================================================
// Combined skip attributes tests
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct CombinedSkipConfig {
    #[feuilletage(skip)]
    always_skip: String,

    #[feuilletage(skip_if_empty)]
    skip_when_empty: Vec<String>,

    #[feuilletage(skip_if_default, default = "default_value")]
    skip_when_default: String,

    #[feuilletage(skip_if = "is_empty_string", default = "")]
    skip_custom: String,

    always_include: String,
}

#[test]
fn test_combined_skip_attributes() {
    let json = r#"{
        "always_skip": "hidden",
        "skip_when_empty": [],
        "skip_when_default": "default_value",
        "skip_custom": "",
        "always_include": "visible"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<CombinedSkipConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    let serialized = feuilletage::to_json_compact(&cfg).unwrap();

    // always_skip: always skipped
    assert!(!serialized.contains("always_skip"));

    // skip_when_empty: skipped because empty
    assert!(!serialized.contains("skip_when_empty"));

    // skip_when_default: skipped because equals default
    assert!(!serialized.contains("skip_when_default"));

    // skip_custom: skipped because is_empty_string returns true
    assert!(!serialized.contains("skip_custom"));

    // always_include: always included
    assert!(serialized.contains("always_include"));
    assert!(serialized.contains("visible"));
}

#[test]
fn test_combined_skip_attributes_non_skipping_case() {
    let json = r#"{
        "always_skip": "still_hidden",
        "skip_when_empty": ["has", "values"],
        "skip_when_default": "not_default",
        "skip_custom": "has_content",
        "always_include": "visible"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<CombinedSkipConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();

    let serialized = feuilletage::to_json_compact(&cfg).unwrap();

    // always_skip: STILL skipped
    assert!(!serialized.contains("always_skip"));

    // skip_when_empty: NOT skipped (has values)
    assert!(serialized.contains("skip_when_empty"));

    // skip_when_default: NOT skipped (differs from default)
    assert!(serialized.contains("skip_when_default"));
    assert!(serialized.contains("not_default"));

    // skip_custom: NOT skipped (not empty)
    assert!(serialized.contains("skip_custom"));
    assert!(serialized.contains("has_content"));

    // always_include: always included
    assert!(serialized.contains("always_include"));
}

// ============================================================================
// skip_serialize (container-level) tests
// ============================================================================

/// Test that skip_serialize at container level skips generating Serialize impl for struct.
/// We verify this by testing that the struct still deserializes but doesn't implement Serialize.
#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(skip_serialize)]
struct SkipSerializeStruct {
    name: String,
    count: i32,
}

#[test]
fn test_skip_serialize_struct_deserializes() {
    let json = r#"{"name": "test", "count": 42}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeStruct>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    assert_eq!(cfg.count, 42);
}

// Note: We can't easily test that Serialize is NOT implemented without compile-time checks.
// The fact that the above compiles and the derive macro doesn't generate Serialize
// is verified by the code structure. If you try to call serde_json::to_string(&cfg)
// on a SkipSerializeStruct, it would fail at compile time with "trait bound not satisfied".

/// Test that skip_serialize works with external_tag enums
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, skip_serialize)]
enum SkipSerializeEnum {
    #[feuilletage(rename = "foo")]
    Foo(String),
    #[feuilletage(rename = "bar")]
    Bar(i32),
}

#[test]
fn test_skip_serialize_enum_deserializes_foo() {
    let json = r#"{"foo": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(result.unwrap(), SkipSerializeEnum::Foo("hello".to_string()));
}

#[test]
fn test_skip_serialize_enum_deserializes_bar() {
    let json = r#"{"bar": 42}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(result.unwrap(), SkipSerializeEnum::Bar(42));
}

/// Test that skip_serialize works with tagged enums
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "type", skip_serialize)]
enum SkipSerializeTaggedEnum {
    #[feuilletage(rename = "alpha")]
    Alpha { value: String },
    #[feuilletage(rename = "beta")]
    Beta { count: i32 },
}

#[test]
fn test_skip_serialize_tagged_enum_deserializes() {
    let json = r#"{"type": "alpha", "value": "test"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeTaggedEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(
        result.unwrap(),
        SkipSerializeTaggedEnum::Alpha {
            value: "test".to_string()
        }
    );
}

/// Test that skip_serialize works with untagged enums
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(untagged, skip_serialize)]
enum SkipSerializeUntaggedEnum {
    Number(i32),
    Text(String),
}

#[test]
fn test_skip_serialize_untagged_enum_deserializes_number() {
    let json = r#"42"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeUntaggedEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(result.unwrap(), SkipSerializeUntaggedEnum::Number(42));
}

#[test]
fn test_skip_serialize_untagged_enum_deserializes_text() {
    let json = r#""hello""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeUntaggedEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(
        result.unwrap(),
        SkipSerializeUntaggedEnum::Text("hello".to_string())
    );
}

/// Test that skip_serialize works with value_matched enums
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(value_matched, skip_serialize)]
enum SkipSerializeValueEnum {
    #[feuilletage(variant = true)]
    Yes,
    #[feuilletage(variant = false)]
    No,
}

#[test]
fn test_skip_serialize_value_matched_enum_deserializes() {
    let json = r#"true"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<SkipSerializeValueEnum>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    assert_eq!(result.unwrap(), SkipSerializeValueEnum::Yes);
}

// ============================================================================
// OnceCell skip tests (Feature 12)
// ============================================================================

use std::cell::OnceCell;

/// Test that skip works with OnceCell fields.
/// The skip attribute should skip both deserialization and serialization for OnceCell.
#[derive(Debug, DeriveConfig)]
struct ConfigWithOnceCell {
    name: String,

    #[feuilletage(skip, default)]
    cached_value: OnceCell<String>,
}

#[test]
fn test_skip_with_oncecell_deserializes() {
    let json = r#"{"name": "test"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ConfigWithOnceCell>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    // OnceCell should be empty (default)
    assert!(cfg.cached_value.get().is_none());
}

#[test]
fn test_skip_with_oncecell_serializes_without_cached_field() {
    let cfg = ConfigWithOnceCell {
        name: "test".to_string(),
        cached_value: OnceCell::new(),
    };

    // Should serialize without the cached_value field
    let serialized = feuilletage::to_json_compact(&cfg).unwrap();
    assert_eq!(serialized, r#"{"name":"test"}"#);
    assert!(!serialized.contains("cached_value"));
}

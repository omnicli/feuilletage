//! Tests for deprecated attribute.
//!
//! The `#[compote(deprecated)]` and `#[compote(deprecated = "message")]` attributes
//! mark fields as deprecated and print warnings when the field is used.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// Note: The deprecated attribute prints warnings to stderr. These tests verify:
// 1. Deprecated fields still function normally (values are deserialized)
// 2. The attribute doesn't prevent normal operation

// ============================================================================
// Basic deprecated attribute tests
// ============================================================================

#[test]
fn test_deprecated_field_with_message() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedConfig {
        #[compote(deprecated = "Use new_field instead")]
        old_field: String,
    }

    let json = r#"{"old_field": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    // The field should still deserialize correctly
    let result: DeprecatedConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_field, "value");
    // Note: Deprecation warning is printed to stderr, not recorded in errors
}

#[test]
fn test_deprecated_field_without_message() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedFlagConfig {
        #[compote(deprecated)]
        old_field: String,
    }

    let json = r#"{"old_field": "test"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedFlagConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_field, "test");
}

// ============================================================================
// Deprecated fields still function normally
// ============================================================================

#[test]
fn test_deprecated_field_still_works() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedWorksConfig {
        #[compote(deprecated = "This field is deprecated but should still work")]
        legacy_field: String,

        new_field: String,
    }

    let json = r#"{"legacy_field": "old", "new_field": "new"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedWorksConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.legacy_field, "old");
    assert_eq!(result.new_field, "new");
}

#[test]
fn test_deprecated_field_required() {
    #[derive(DeriveConfig, Debug)]
    struct DeprecatedRequiredConfig {
        #[compote(deprecated = "Still required even though deprecated")]
        required_old_field: String,
    }

    // Missing required deprecated field should still fail
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<DeprecatedRequiredConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Missing required field should fail even if deprecated"
    );
}

// ============================================================================
// Deprecated with default value
// ============================================================================

#[test]
fn test_deprecated_with_default() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedDefaultConfig {
        #[compote(deprecated = "Use newer_field instead", default = "default_value")]
        older_field: String,
    }

    // When field is missing, default is used (no deprecation warning since field wasn't present)
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedDefaultConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.older_field, "default_value");
}

#[test]
fn test_deprecated_with_default_value_provided() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedDefaultConfig {
        #[compote(deprecated = "Use newer_field instead", default = "default")]
        older_field: String,
    }

    // When deprecated field is provided, it should work (and print warning)
    let json = r#"{"older_field": "explicit_value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedDefaultConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.older_field, "explicit_value");
}

// ============================================================================
// Deprecated with other types
// ============================================================================

#[test]
fn test_deprecated_int_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedIntConfig {
        #[compote(deprecated = "Use new_count instead")]
        old_count: i32,
    }

    let json = r#"{"old_count": 42}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedIntConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_count, 42);
}

#[test]
fn test_deprecated_bool_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedBoolConfig {
        #[compote(deprecated = "Use new_flag instead")]
        old_flag: bool,
    }

    let json = r#"{"old_flag": true}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedBoolConfig = config.deserialize().expect("Should succeed");

    assert!(result.old_flag);
}

#[test]
fn test_deprecated_vec_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedVecConfig {
        #[compote(deprecated = "Use new_tags instead")]
        old_tags: Vec<String>,
    }

    let json = r#"{"old_tags": ["a", "b", "c"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedVecConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_tags, vec!["a", "b", "c"]);
}

#[test]
fn test_deprecated_option_field() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedOptionConfig {
        #[compote(deprecated = "Use new_optional instead")]
        old_optional: Option<String>,
    }

    // With value
    let json = r#"{"old_optional": "present"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedOptionConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.old_optional, Some("present".to_string()));

    // Without value
    let json2 = r#"{}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));

    let result2: DeprecatedOptionConfig = config2.deserialize().expect("Should succeed");
    assert_eq!(result2.old_optional, None);
}

// ============================================================================
// Deprecated combined with other attributes
// ============================================================================

#[test]
fn test_deprecated_with_rename() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedRenameConfig {
        #[compote(rename = "oldName", deprecated = "Use newName instead")]
        old_name: String,
    }

    let json = r#"{"oldName": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedRenameConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_name, "value");
}

#[test]
fn test_deprecated_with_aliases() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedAliasesConfig {
        #[compote(aliases = ["legacy_name", "old_name"], deprecated = "All these names are deprecated")]
        current_name: String,
    }

    // Using an alias
    let json = r#"{"legacy_name": "via_alias"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedAliasesConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.current_name, "via_alias");
}

#[test]
fn test_deprecated_with_transform() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedTransformConfig {
        #[compote(deprecated = "Use new_name instead", transform = "to_uppercase")]
        old_name: String,
    }

    let json = r#"{"old_name": "lowercase"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedTransformConfig = config.deserialize().expect("Should succeed");

    // Transform should still be applied
    assert_eq!(result.old_name, "LOWERCASE");
}

// ============================================================================
// Multiple deprecated fields
// ============================================================================

#[test]
fn test_multiple_deprecated_fields() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MultiDeprecatedConfig {
        #[compote(deprecated = "Use config_a instead")]
        old_a: String,

        #[compote(deprecated = "Use config_b instead")]
        old_b: i32,

        // Non-deprecated field
        current_field: String,
    }

    let json = r#"{"old_a": "a", "old_b": 10, "current_field": "current"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiDeprecatedConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.old_a, "a");
    assert_eq!(result.old_b, 10);
    assert_eq!(result.current_field, "current");
}

// ============================================================================
// Deprecated serialization behavior
// ============================================================================

#[test]
fn test_deprecated_field_serializes() {
    // Note: DeriveConfig automatically implements serde::Serialize
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DeprecatedSerializeConfig {
        #[compote(deprecated = "Deprecated but still serialized")]
        old_field: String,
    }

    let json = r#"{"old_field": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DeprecatedSerializeConfig = config.deserialize().expect("Should succeed");

    // Deprecated fields should still serialize normally
    let serialized = compote::to_json_compact(&result).unwrap();
    assert!(serialized.contains("old_field"));
    assert!(serialized.contains("value"));
}

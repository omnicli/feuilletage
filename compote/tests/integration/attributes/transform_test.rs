//! Tests for the generic transform attribute.
//!
//! This tests the `#[compote(transform = "fn")]` attribute which applies
//! a transform function to field values during deserialization.
//!
//! Note: Shortcut attributes like `relative_path` and `duration` have their own test files.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// Custom transform functions (using built-in transforms from compote::transform)
// ============================================================================

// Note: Custom user-defined transform functions are not directly supported
// because transforms must be in the compote::transform module.
// These tests use the built-in transforms: to_uppercase, to_lowercase, trim

// ============================================================================
// to_uppercase transform tests
// ============================================================================

#[test]
fn test_transform_string_to_uppercase() {
    #[derive(DeriveConfig, Debug)]
    struct UppercaseConfig {
        #[compote(transform = "to_uppercase")]
        name: String,
    }

    let json = r#"{"name": "hello world"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UppercaseConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, "HELLO WORLD");
    assert!(!config.errors().has_errors());
}

#[test]
fn test_transform_uppercase_with_default() {
    #[derive(DeriveConfig, Debug)]
    struct UppercaseDefaultConfig {
        #[compote(transform = "to_uppercase", default = "default_value")]
        name: String,
    }

    // Test with provided value
    let json = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UppercaseDefaultConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, "TEST");

    // Test with missing value (uses default)
    let json_missing = r#"{}"#;
    let mut config2 = Config::default();
    config2.load_json(
        json_missing,
        Context::new(Source::Programmatic, Level::User),
    );

    let result2: UppercaseDefaultConfig = config2.deserialize().expect("Should succeed");
    // Default is NOT transformed (transform only applies to loaded values)
    assert_eq!(result2.name, "default_value");
}

#[test]
fn test_transform_uppercase_required_field() {
    #[derive(DeriveConfig, Debug)]
    struct UppercaseRequiredConfig {
        #[compote(transform = "to_uppercase")]
        name: String,
    }

    let json = r#"{"name": "required value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UppercaseRequiredConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.name, "REQUIRED VALUE");
}

#[test]
fn test_transform_uppercase_missing_required_fails() {
    #[derive(DeriveConfig, Debug)]
    struct UppercaseRequiredConfig {
        #[compote(transform = "to_uppercase")]
        name: String,
    }

    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<UppercaseRequiredConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field is missing"
    );
}

// ============================================================================
// to_lowercase transform tests
// ============================================================================

#[test]
fn test_transform_string_to_lowercase() {
    #[derive(DeriveConfig, Debug)]
    struct LowercaseConfig {
        #[compote(transform = "to_lowercase")]
        name: String,
    }

    let json = r#"{"name": "HELLO WORLD"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: LowercaseConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, "hello world");
    assert!(!config.errors().has_errors());
}

#[test]
fn test_transform_lowercase_with_mixed_case() {
    #[derive(DeriveConfig, Debug)]
    struct LowercaseConfig {
        #[compote(transform = "to_lowercase")]
        value: String,
    }

    let json = r#"{"value": "HeLLo WoRLD"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: LowercaseConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "hello world");
}

// ============================================================================
// trim transform tests
// ============================================================================

#[test]
fn test_transform_string_trim() {
    #[derive(DeriveConfig, Debug)]
    struct TrimConfig {
        #[compote(transform = "trim")]
        value: String,
    }

    let json = r#"{"value": "  hello world  "}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TrimConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.value, "hello world");
    assert!(!config.errors().has_errors());
}

#[test]
fn test_transform_trim_with_newlines() {
    #[derive(DeriveConfig, Debug)]
    struct TrimConfig {
        #[compote(transform = "trim")]
        value: String,
    }

    let json = r#"{"value": "\n\t  content  \n"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TrimConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "content");
}

#[test]
fn test_transform_trim_empty_after_trim() {
    #[derive(DeriveConfig, Debug)]
    struct TrimConfig {
        #[compote(transform = "trim")]
        value: String,
    }

    let json = r#"{"value": "   "}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TrimConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, "");
}

// ============================================================================
// transform_each tests (on Vec with allow_single/allow_map)
// ============================================================================

// Note: transform_each is designed to work in conjunction with allow_single or allow_map
// attributes. It applies the transform to each element in the resulting Vec.

#[test]
fn test_transform_each_with_allow_single() {
    #[derive(DeriveConfig, Debug)]
    struct TransformEachAllowSingleConfig {
        #[compote(allow_single, transform_each = "to_uppercase")]
        tags: Vec<String>,
    }

    // Single value is converted to vec, then transform_each is applied
    let json = r#"{"tags": "single"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformEachAllowSingleConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.tags, vec!["SINGLE"]);
    assert!(!config.errors().has_errors());
}

#[test]
fn test_transform_each_with_allow_single_array() {
    #[derive(DeriveConfig, Debug)]
    struct TransformEachAllowSingleArrayConfig {
        #[compote(allow_single, transform_each = "to_uppercase")]
        tags: Vec<String>,
    }

    // Array input - transform_each should apply to each element
    let json = r#"{"tags": ["hello", "world"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformEachAllowSingleArrayConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.tags, vec!["HELLO", "WORLD"]);
}

#[test]
fn test_transform_each_with_allow_single_trim() {
    #[derive(DeriveConfig, Debug)]
    struct TransformEachTrimConfig {
        #[compote(allow_single, transform_each = "trim")]
        items: Vec<String>,
    }

    let json = r#"{"items": ["  a  ", " b ", "c"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformEachTrimConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.items, vec!["a", "b", "c"]);
}

#[test]
fn test_transform_each_empty_vec() {
    #[derive(DeriveConfig, Debug)]
    struct TransformEachEmptyConfig {
        #[compote(allow_single, transform_each = "to_uppercase")]
        items: Vec<String>,
    }

    let json = r#"{"items": []}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformEachEmptyConfig = config.deserialize().expect("Should succeed");
    assert!(result.items.is_empty());
}

// ============================================================================
// Multiple transforms in same struct
// ============================================================================

#[test]
fn test_multiple_transforms_same_struct() {
    #[derive(DeriveConfig, Debug)]
    struct MultiTransformConfig {
        #[compote(transform = "to_uppercase")]
        upper_field: String,

        #[compote(transform = "to_lowercase")]
        lower_field: String,

        #[compote(transform = "trim")]
        trimmed_field: String,
    }

    let json = r#"{
        "upper_field": "make me upper",
        "lower_field": "MAKE ME LOWER",
        "trimmed_field": "  trim me  "
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiTransformConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.upper_field, "MAKE ME UPPER");
    assert_eq!(result.lower_field, "make me lower");
    assert_eq!(result.trimmed_field, "trim me");
}

// ============================================================================
// Transform with Option<T>
// ============================================================================

#[test]
fn test_transform_with_option_present() {
    #[derive(DeriveConfig, Debug)]
    struct TransformOptionConfig {
        #[compote(transform = "to_uppercase")]
        name: Option<String>,
    }

    let json = r#"{"name": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformOptionConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, Some("HELLO".to_string()));
}

#[test]
fn test_transform_with_option_absent() {
    #[derive(DeriveConfig, Debug)]
    struct TransformOptionConfig {
        #[compote(transform = "to_uppercase")]
        name: Option<String>,
    }

    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformOptionConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, None);
}

// ============================================================================
// Transform combined with other attributes
// ============================================================================

#[test]
fn test_transform_with_rename() {
    #[derive(DeriveConfig, Debug)]
    struct TransformRenameConfig {
        #[compote(rename = "userName", transform = "to_lowercase")]
        user_name: String,
    }

    let json = r#"{"userName": "ADMIN"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformRenameConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.user_name, "admin");
}

#[test]
fn test_transform_with_aliases() {
    #[derive(DeriveConfig, Debug)]
    struct TransformAliasConfig {
        #[compote(aliases = ["user", "username"], transform = "to_uppercase")]
        user_name: String,
    }

    let json = r#"{"user": "alice"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransformAliasConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.user_name, "ALICE");
}

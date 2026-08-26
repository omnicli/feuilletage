//! Tests for allow_single on untagged enum variants
//!
//! When a variant has #[compote(allow_single)], scalar input is wrapped
//! into a single-element array before deserialization of the inner Vec<T>.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// Test 1: Basic allow_single on untagged enum
// ============================================================================

#[derive(Debug, PartialEq, DeriveConfig)]
#[compote(untagged)]
enum FlexInput {
    #[compote(variant = "none")]
    None,
    #[compote(allow_single)]
    Items(Vec<String>),
}

#[test]
fn test_flex_input_exact_match() {
    // "none" should match the exact variant, not allow_single
    let json = r#""none""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FlexInput>();
    assert!(
        result.is_ok(),
        "Should match exact variant: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), FlexInput::None);
}

#[test]
fn test_flex_input_scalar_string() {
    // "hello" should be wrapped into vec!["hello"] via allow_single
    let json = r#""hello""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FlexInput>();
    assert!(
        result.is_ok(),
        "Should wrap scalar via allow_single: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), FlexInput::Items(vec!["hello".to_string()]));
}

#[test]
fn test_flex_input_array() {
    // ["a", "b"] should match Items normally as array
    let json = r#"["a", "b"]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FlexInput>();
    assert!(result.is_ok(), "Should accept array: {:?}", result.err());
    assert_eq!(
        result.unwrap(),
        FlexInput::Items(vec!["a".to_string(), "b".to_string()])
    );
}

// ============================================================================
// Test 2: PipConfig-like pattern
// ============================================================================

#[derive(Debug, PartialEq, DeriveConfig)]
#[compote(untagged)]
enum PipConfig {
    #[compote(variant = false)]
    Disabled,
    #[compote(variant = true | "auto")]
    Auto,
    #[compote(allow_single)]
    Files(Vec<String>),
}

#[test]
fn test_pip_config_disabled() {
    let json = r#"false"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(result.is_ok(), "Should match Disabled: {:?}", result.err());
    assert_eq!(result.unwrap(), PipConfig::Disabled);
}

#[test]
fn test_pip_config_auto_bool() {
    let json = r#"true"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(result.is_ok(), "Should match Auto: {:?}", result.err());
    assert_eq!(result.unwrap(), PipConfig::Auto);
}

#[test]
fn test_pip_config_auto_string() {
    let json = r#""auto""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(result.is_ok(), "Should match Auto: {:?}", result.err());
    assert_eq!(result.unwrap(), PipConfig::Auto);
}

#[test]
fn test_pip_config_single_file() {
    let json = r#""requirements.txt""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(
        result.is_ok(),
        "Should wrap scalar into Files: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap(),
        PipConfig::Files(vec!["requirements.txt".to_string()])
    );
}

#[test]
fn test_pip_config_multiple_files() {
    let json = r#"["a.txt", "b.txt"]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(
        result.is_ok(),
        "Should accept array of files: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap(),
        PipConfig::Files(vec!["a.txt".to_string(), "b.txt".to_string()])
    );
}

// ============================================================================
// Test 3: allow_single with Vec<i64>
// ============================================================================

#[derive(Debug, PartialEq, DeriveConfig)]
#[compote(untagged)]
enum NumOrNums {
    #[compote(allow_single)]
    Numbers(Vec<i64>),
}

#[test]
fn test_num_or_nums_single_int() {
    let json = r#"42"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NumOrNums>();
    assert!(result.is_ok(), "Should wrap single int: {:?}", result.err());
    assert_eq!(result.unwrap(), NumOrNums::Numbers(vec![42]));
}

#[test]
fn test_num_or_nums_array() {
    let json = r#"[1, 2, 3]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NumOrNums>();
    assert!(
        result.is_ok(),
        "Should accept array of ints: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), NumOrNums::Numbers(vec![1, 2, 3]));
}

// ============================================================================
// Test 4: allow_single with empty array
// ============================================================================

#[test]
fn test_pip_config_empty_array() {
    let json = r#"[]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PipConfig>();
    assert!(
        result.is_ok(),
        "Should accept empty array: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), PipConfig::Files(vec![]));
}

// ============================================================================
// Test 5: allow_single used as struct field
// ============================================================================

#[derive(Debug, PartialEq, DeriveConfig)]
struct AppConfig {
    #[compote(default)]
    pip: Option<PipConfig>,
}

#[test]
fn test_pip_config_in_struct_scalar() {
    let json = r#"{"pip": "requirements.txt"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize scalar in struct field: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().pip,
        Some(PipConfig::Files(vec!["requirements.txt".to_string()]))
    );
}

#[test]
fn test_pip_config_in_struct_array() {
    let json = r#"{"pip": ["a.txt", "b.txt"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize array in struct field: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().pip,
        Some(PipConfig::Files(vec![
            "a.txt".to_string(),
            "b.txt".to_string()
        ]))
    );
}

#[test]
fn test_pip_config_in_struct_bool() {
    let json = r#"{"pip": false}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize bool in struct field: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().pip, Some(PipConfig::Disabled));
}

//! Tests for the fallback field attribute.
//!
//! The `fallback` attribute allows a field to use another field's value
//! when its own value is not provided.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Basic fallback tests
// ============================================================================

/// Simple fallback from one field to another
#[derive(Debug, DeriveConfig, PartialEq)]
struct SimpleFallback {
    name: String,

    #[feuilletage(fallback = "name")]
    display_name: String,
}

#[test]
fn test_fallback_uses_own_value_when_present() {
    let json = r#"{
        "name": "internal",
        "display_name": "External Name"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SimpleFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "internal");
    assert_eq!(result.display_name, "External Name");
}

#[test]
fn test_fallback_uses_fallback_when_missing() {
    let json = r#"{
        "name": "only_name"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SimpleFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "only_name");
    assert_eq!(result.display_name, "only_name"); // Falls back to name
}

// ============================================================================
// Fallback with default
// ============================================================================

/// Fallback with a static default
#[derive(Debug, DeriveConfig, PartialEq)]
struct FallbackWithDefault {
    name: Option<String>,

    #[feuilletage(fallback = "name", default = "unnamed")]
    display_name: String,
}

#[test]
fn test_fallback_uses_default_when_both_missing() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: FallbackWithDefault = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, None);
    assert_eq!(result.display_name, "unnamed");
}

#[test]
fn test_fallback_uses_fallback_over_default() {
    let json = r#"{"name": "from_name"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: FallbackWithDefault = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, Some("from_name".to_string()));
    assert_eq!(result.display_name, "from_name");
}

// ============================================================================
// Bidirectional fallback (cycle)
// ============================================================================

/// Bidirectional fallback - both fields fall back to each other
#[derive(Debug, DeriveConfig, PartialEq)]
struct BidirectionalFallback {
    #[feuilletage(fallback = "choice")]
    id: String,

    #[feuilletage(fallback = "id", default = "default_value")]
    choice: String,
}

#[test]
fn test_cycle_with_id_only() {
    let json = r#"{"id": "my_id"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BidirectionalFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.id, "my_id");
    assert_eq!(result.choice, "my_id"); // Falls back to id
}

#[test]
fn test_cycle_with_choice_only() {
    let json = r#"{"choice": "my_choice"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BidirectionalFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.id, "my_choice"); // Falls back to choice
    assert_eq!(result.choice, "my_choice");
}

#[test]
fn test_cycle_with_both_present() {
    let json = r#"{"id": "my_id", "choice": "my_choice"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BidirectionalFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.id, "my_id");
    assert_eq!(result.choice, "my_choice");
}

#[test]
fn test_cycle_with_neither_uses_default() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BidirectionalFallback = config.deserialize().expect("Should deserialize");
    // Both should get the default from the cycle (choice has default)
    assert_eq!(result.id, "default_value");
    assert_eq!(result.choice, "default_value");
}

// ============================================================================
// Chain fallback (A -> B -> C)
// ============================================================================

/// Chain fallback - A falls back to B which falls back to C
#[derive(Debug, DeriveConfig, PartialEq)]
struct ChainFallback {
    #[feuilletage(fallback = "secondary")]
    primary: String,

    #[feuilletage(fallback = "tertiary")]
    secondary: String,

    #[feuilletage(default = "chain_default")]
    tertiary: String,
}

#[test]
fn test_chain_fallback_with_primary() {
    let json = r#"{"primary": "p"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.primary, "p");
    assert_eq!(result.secondary, "chain_default"); // Falls back to tertiary
    assert_eq!(result.tertiary, "chain_default");
}

#[test]
fn test_chain_fallback_with_secondary() {
    let json = r#"{"secondary": "s"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.primary, "s"); // Falls back to secondary
    assert_eq!(result.secondary, "s");
    assert_eq!(result.tertiary, "chain_default");
}

#[test]
fn test_chain_fallback_with_tertiary() {
    let json = r#"{"tertiary": "t"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.primary, "t"); // Falls back to secondary which falls back to tertiary
    assert_eq!(result.secondary, "t"); // Falls back to tertiary
    assert_eq!(result.tertiary, "t");
}

#[test]
fn test_chain_fallback_with_none() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.primary, "chain_default");
    assert_eq!(result.secondary, "chain_default");
    assert_eq!(result.tertiary, "chain_default");
}

// ============================================================================
// Fallback with Option types
// ============================================================================

/// Fallback with Option fields
#[derive(Debug, DeriveConfig, PartialEq)]
struct OptionFallback {
    source: Option<String>,

    #[feuilletage(fallback = "source")]
    target: Option<String>,
}

#[test]
fn test_option_fallback_with_source() {
    let json = r#"{"source": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: OptionFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.source, Some("value".to_string()));
    assert_eq!(result.target, Some("value".to_string())); // Falls back to source
}

#[test]
fn test_option_fallback_with_neither() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: OptionFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.source, None);
    assert_eq!(result.target, None); // No fallback value, stays None
}

// ============================================================================
// Fallback with rename
// ============================================================================

/// Fallback with renamed fields
#[derive(Debug, DeriveConfig, PartialEq)]
struct RenamedFallback {
    #[feuilletage(rename = "userName")]
    user_name: String,

    #[feuilletage(fallback = "user_name", rename = "displayName")]
    display_name: String,
}

#[test]
fn test_fallback_with_rename() {
    let json = r#"{"userName": "jdoe"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: RenamedFallback = config.deserialize().expect("Should deserialize");
    assert_eq!(result.user_name, "jdoe");
    assert_eq!(result.display_name, "jdoe"); // Falls back to user_name
}

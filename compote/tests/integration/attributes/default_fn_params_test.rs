//! Tests for the default_fn attribute with field parameters.
//!
//! The `default_fn` attribute can now accept field references as parameters:
//! - `#[compote(default_fn = "fn_name")]` - calls fn_name() (existing behavior)
//! - `#[compote(default_fn = "fn_name(field1, field2)")]` - calls fn_name(&field1, &field2)

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// Helper functions for default_fn
// ============================================================================

fn compute_exact(version: &Option<String>) -> bool {
    version.is_some()
}

fn compute_display_name(first_name: &String, last_name: &String) -> String {
    format!("{} {}", first_name, last_name)
}

fn compute_level(min: &i32, max: &i32) -> i32 {
    (min + max) / 2
}

fn compute_enabled(count: &i32) -> bool {
    *count > 0
}

// ============================================================================
// Basic parameterized default_fn tests
// ============================================================================

/// Test default_fn with single Option parameter
#[derive(Debug, DeriveConfig, PartialEq)]
struct ExactVersionConfig {
    version: Option<String>,

    #[compote(default_fn = "compute_exact(version)")]
    exact: bool,
}

#[test]
fn test_default_fn_with_option_some() {
    let json = r#"{"version": "1.0.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExactVersionConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.version, Some("1.0.0".to_string()));
    assert!(result.exact); // compute_exact returns true for Some
}

#[test]
fn test_default_fn_with_option_none() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExactVersionConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.version, None);
    assert!(!result.exact); // compute_exact returns false for None
}

#[test]
fn test_default_fn_explicit_value_overrides() {
    // When exact is explicitly set, it should use that value
    let json = r#"{"version": "1.0.0", "exact": false}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExactVersionConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.version, Some("1.0.0".to_string()));
    assert!(!result.exact); // Explicit value overrides default_fn
}

// ============================================================================
// Multiple parameters tests
// ============================================================================

/// Test default_fn with multiple parameters
#[derive(Debug, DeriveConfig, PartialEq)]
struct MultiParamConfig {
    first_name: String,
    last_name: String,

    #[compote(default_fn = "compute_display_name(first_name, last_name)")]
    display_name: String,
}

#[test]
fn test_default_fn_multiple_params() {
    let json = r#"{"first_name": "John", "last_name": "Doe"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiParamConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.first_name, "John");
    assert_eq!(result.last_name, "Doe");
    assert_eq!(result.display_name, "John Doe");
}

#[test]
fn test_default_fn_multiple_params_explicit_override() {
    let json = r#"{"first_name": "John", "last_name": "Doe", "display_name": "JD"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiParamConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.display_name, "JD"); // Explicit value overrides
}

// ============================================================================
// Numeric parameters tests
// ============================================================================

/// Test default_fn with numeric parameters
#[derive(Debug, DeriveConfig, PartialEq)]
struct NumericConfig {
    min: i32,
    max: i32,

    #[compote(default_fn = "compute_level(min, max)")]
    level: i32,
}

#[test]
fn test_default_fn_numeric_params() {
    let json = r#"{"min": 10, "max": 20}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: NumericConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.min, 10);
    assert_eq!(result.max, 20);
    assert_eq!(result.level, 15); // (10 + 20) / 2
}

// ============================================================================
// Dependency chain tests (default_fn depends on field with default)
// ============================================================================

/// Test default_fn where parameter field has its own default
#[derive(Debug, DeriveConfig, PartialEq)]
struct ChainedDefaultConfig {
    #[compote(default = "0")]
    count: i32,

    #[compote(default_fn = "compute_enabled(count)")]
    enabled: bool,
}

#[test]
fn test_default_fn_with_default_dependency() {
    // Both fields use defaults
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainedDefaultConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.count, 0); // default
    assert!(!result.enabled); // compute_enabled(0) = false
}

#[test]
fn test_default_fn_with_explicit_dependency() {
    let json = r#"{"count": 5}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainedDefaultConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.count, 5);
    assert!(result.enabled); // compute_enabled(5) = true
}

// ============================================================================
// Combined with fallback tests
// ============================================================================

/// Test where fallback and default_fn are independent
fn get_default_fallback_value() -> String {
    "fallback_default".to_string()
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct IndependentFallbackDefaultFn {
    source: Option<String>,

    #[compote(fallback = "source", default_fn = "get_default_fallback_value")]
    target: String,

    #[compote(default_fn = "compute_exact(source)")]
    has_source: bool,
}

#[test]
fn test_fallback_and_default_fn_independent() {
    let json = r#"{"source": "my_source"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: IndependentFallbackDefaultFn = config.deserialize().expect("Should deserialize");
    assert_eq!(result.source, Some("my_source".to_string()));
    assert_eq!(result.target, "my_source"); // fallback from source
    assert!(result.has_source); // compute_exact(Some(...))
}

#[test]
fn test_fallback_uses_default_fn_when_source_missing() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: IndependentFallbackDefaultFn = config.deserialize().expect("Should deserialize");
    assert_eq!(result.source, None);
    assert_eq!(result.target, "fallback_default"); // default_fn since source is None
    assert!(!result.has_source); // compute_exact(None)
}

// ============================================================================
// Mixed with regular default_fn (no params)
// ============================================================================

fn get_default_count() -> i32 {
    42
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct MixedDefaultFnConfig {
    #[compote(default_fn = "get_default_count")]
    count: i32,

    #[compote(default_fn = "compute_enabled(count)")]
    enabled: bool,
}

#[test]
fn test_mixed_default_fn_all_defaults() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MixedDefaultFnConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.count, 42); // get_default_count()
    assert!(result.enabled); // compute_enabled(42) = true
}

#[test]
fn test_mixed_default_fn_partial() {
    let json = r#"{"count": 0}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MixedDefaultFnConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.count, 0);
    assert!(!result.enabled); // compute_enabled(0) = false
}

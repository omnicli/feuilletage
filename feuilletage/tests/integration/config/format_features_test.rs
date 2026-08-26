//! Tests for default format feature flags
//!
//! These tests verify that the `default_format()` method correctly resolves
//! the default configuration format based on enabled feature flags.

use feuilletage::Format;

#[test]
fn test_default_format_returns_valid_format() {
    let format = Format::default_format();
    // Should always return a valid format (not Unknown)
    assert!(
        matches!(format, Format::Yaml | Format::Json | Format::Toml),
        "default_format() should return Yaml, Json, or Toml, got: {:?}",
        format
    );
}

#[test]
#[cfg(feature = "yaml")]
fn test_default_format_with_yaml_feature() {
    // With yaml feature enabled, default should be Yaml
    let format = Format::default_format();
    assert!(
        matches!(format, Format::Yaml),
        "With yaml feature enabled, default_format() should return Yaml, got: {:?}",
        format
    );
}

#[test]
#[cfg(all(not(feature = "yaml"), feature = "toml"))]
fn test_default_format_with_toml_only() {
    // With only toml feature enabled, default should be Toml
    let format = Format::default_format();
    assert!(
        matches!(format, Format::Toml),
        "With only toml feature enabled, default_format() should return Toml, got: {:?}",
        format
    );
}

#[test]
#[cfg(all(not(feature = "yaml"), not(feature = "toml")))]
fn test_default_format_with_json_fallback() {
    // With no yaml or toml features, default should be Json
    let format = Format::default_format();
    assert!(
        matches!(format, Format::Json),
        "With no yaml/toml features, default_format() should return Json, got: {:?}",
        format
    );
}

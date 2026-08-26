//! Datetime format validation tests (chrono feature)
//!
//! Tests for the datetime attribute which validates string fields
//! against date/time format patterns.

#![cfg(all(feature = "json", feature = "chrono"))]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
struct DateConfig {
    #[compote(datetime = "%Y-%m-%d")]
    date_field: String,
}

#[derive(Debug, DeriveConfig)]
struct DateTimeConfig {
    #[compote(datetime = "%Y-%m-%d %H:%M:%S")]
    datetime_field: String,
}

#[derive(Debug, DeriveConfig)]
struct TimeConfig {
    #[compote(datetime = "%H:%M:%S")]
    time_field: String,
}

#[derive(Debug, DeriveConfig)]
struct DateWithDefaultConfig {
    #[compote(datetime = "%Y-%m-%d", default = "2024-01-01")]
    date_field: String,
}

#[test]
fn test_format_valid_date() {
    let json = r#"{"date_field": "2024-01-15"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateConfig>();
    assert!(result.is_ok(), "Valid date should pass validation");

    let cfg = result.unwrap();
    assert_eq!(cfg.date_field, "2024-01-15");
}

#[test]
fn test_format_invalid_date() {
    let json = r#"{"date_field": "not-a-date"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateConfig>();
    assert!(result.is_err(), "Invalid date should fail validation");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("does not match date/time format"),
        "Error should mention format mismatch: {}",
        err_str
    );
}

#[test]
fn test_format_wrong_date_format() {
    // Date in wrong format (MM/DD/YYYY instead of YYYY-MM-DD)
    let json = r#"{"date_field": "01/15/2024"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateConfig>();
    assert!(
        result.is_err(),
        "Date in wrong format should fail validation"
    );
}

#[test]
fn test_format_valid_datetime() {
    let json = r#"{"datetime_field": "2024-01-15 10:30:45"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateTimeConfig>();
    assert!(result.is_ok(), "Valid datetime should pass validation");

    let cfg = result.unwrap();
    assert_eq!(cfg.datetime_field, "2024-01-15 10:30:45");
}

#[test]
fn test_format_invalid_datetime() {
    let json = r#"{"datetime_field": "2024-01-15"}"#; // Missing time part

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateTimeConfig>();
    assert!(
        result.is_err(),
        "Incomplete datetime should fail validation"
    );
}

#[test]
fn test_format_valid_time() {
    let json = r#"{"time_field": "14:30:00"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<TimeConfig>();
    assert!(result.is_ok(), "Valid time should pass validation");

    let cfg = result.unwrap();
    assert_eq!(cfg.time_field, "14:30:00");
}

#[test]
fn test_format_invalid_time() {
    let json = r#"{"time_field": "25:00:00"}"#; // Invalid hour

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<TimeConfig>();
    assert!(result.is_err(), "Invalid time should fail validation");
}

#[test]
fn test_format_with_default() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateWithDefaultConfig>();
    assert!(result.is_ok(), "Missing field with default should succeed");

    let cfg = result.unwrap();
    assert_eq!(cfg.date_field, "2024-01-01");
}

#[test]
fn test_format_with_default_override() {
    let json = r#"{"date_field": "2025-12-25"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateWithDefaultConfig>();
    assert!(result.is_ok(), "Valid date should override default");

    let cfg = result.unwrap();
    assert_eq!(cfg.date_field, "2025-12-25");
}

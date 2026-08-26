//! Unit tests for coerce module (type coercion functions).
//!
//! Extracted from compote/src/coerce.rs

use compote::coerce::{coerce_to_bool, coerce_to_f64, coerce_to_i64, coerce_to_string};
use compote::{Context, ContextValue};

// Helper functions to create ContextValue with default context
fn cv_string(s: &str) -> ContextValue {
    ContextValue::string(s.to_string(), Context::default())
}

fn cv_bool(b: bool) -> ContextValue {
    ContextValue::bool(b, Context::default())
}

fn cv_int(i: i64) -> ContextValue {
    ContextValue::int(i, Context::default())
}

fn cv_float(f: f64) -> ContextValue {
    ContextValue::float(f, Context::default())
}

#[test]
fn test_coerce_string() {
    assert_eq!(
        coerce_to_string(&cv_string("hello")),
        Some("hello".to_string())
    );
    assert_eq!(coerce_to_string(&cv_bool(true)), Some("true".to_string()));
    assert_eq!(coerce_to_string(&cv_int(42)), Some("42".to_string()));
}

#[test]
fn test_coerce_bool() {
    assert_eq!(coerce_to_bool(&cv_bool(true)), Some(true));
    assert_eq!(coerce_to_bool(&cv_string("yes")), Some(true));
    assert_eq!(coerce_to_bool(&cv_int(0)), Some(false));
}

#[test]
fn test_coerce_bool_on_off() {
    // Test "on" is truthy
    assert_eq!(coerce_to_bool(&cv_string("on")), Some(true));
    assert_eq!(coerce_to_bool(&cv_string("ON")), Some(true));
    assert_eq!(coerce_to_bool(&cv_string("On")), Some(true));

    // Test "off" is falsy
    assert_eq!(coerce_to_bool(&cv_string("off")), Some(false));
    assert_eq!(coerce_to_bool(&cv_string("OFF")), Some(false));
    assert_eq!(coerce_to_bool(&cv_string("Off")), Some(false));
}

#[test]
fn test_coerce_bool_all_truthy_values() {
    // All truthy string values
    for val in &[
        "true", "TRUE", "True", "yes", "YES", "Yes", "y", "Y", "on", "ON", "On", "1",
    ] {
        assert_eq!(
            coerce_to_bool(&cv_string(val)),
            Some(true),
            "Expected '{}' to be truthy",
            val
        );
    }

    // All truthy non-string values
    assert_eq!(coerce_to_bool(&cv_bool(true)), Some(true));
    assert_eq!(coerce_to_bool(&cv_int(1)), Some(true));
    assert_eq!(coerce_to_bool(&cv_int(-1)), Some(true)); // Non-zero is truthy
    assert_eq!(coerce_to_bool(&cv_int(42)), Some(true));
    assert_eq!(coerce_to_bool(&cv_float(1.0)), Some(true));
    assert_eq!(coerce_to_bool(&cv_float(-0.5)), Some(true));
}

#[test]
fn test_coerce_bool_all_falsy_values() {
    // All falsy string values
    for val in &[
        "false", "FALSE", "False", "no", "NO", "No", "n", "N", "off", "OFF", "Off", "0",
    ] {
        assert_eq!(
            coerce_to_bool(&cv_string(val)),
            Some(false),
            "Expected '{}' to be falsy",
            val
        );
    }

    // All falsy non-string values
    assert_eq!(coerce_to_bool(&cv_bool(false)), Some(false));
    assert_eq!(coerce_to_bool(&cv_int(0)), Some(false));
    assert_eq!(coerce_to_bool(&cv_float(0.0)), Some(false));
}

#[test]
fn test_coerce_i64() {
    assert_eq!(coerce_to_i64(&cv_int(42)), Some(42));
    assert_eq!(coerce_to_i64(&cv_float(42.0)), Some(42));
    assert_eq!(coerce_to_i64(&cv_float(42.5)), None);
}

#[test]
fn test_coerce_f64() {
    assert_eq!(coerce_to_f64(&cv_float(3.14)), Some(3.14));
    assert_eq!(coerce_to_f64(&cv_int(42)), Some(42.0));
}

//! Merge modifier tests.
//!
//! Tests for merge modifiers:
//! - __toreplace - completely replace an object/value instead of merging
//! - __toappend - append to an array
//! - __toprepend - prepend to an array
//! - __tokeep - keep existing value if present (no-op when value exists)

#![cfg(feature = "json")]

use compote::{Config, Context, ContextValue, Level, Source};

#[test]
fn test_merge_modifiers() {
    let mut config = Config::default();

    // Initial config
    let json1 = r#"{
        "list": ["a", "b"],
        "value": 1
    }"#;
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));

    // Test __tokeep
    let json2 = r#"{"value__tokeep": 999}"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    // value should still be 1
    if let ContextValue::Object(map, _) = config.root() {
        if let Some(value) = map.get("value") {
            if let ContextValue::Int(i, _) = value {
                assert_eq!(*i, 1, "__tokeep should not override");
            }
        }
    }

    // Test __toappend
    let json3 = r#"{"list__toappend": "c"}"#;
    config.load_json(json3, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(list) = map.get("list") {
            if let ContextValue::Array(arr, _) = list {
                assert_eq!(arr.len(), 3);
                if let ContextValue::String(s, _) = &arr[2] {
                    assert_eq!(s, "c", "__toappend should add to array");
                }
            }
        }
    }

    // Test __toprepend
    let json4 = r#"{"list__toprepend": "z"}"#;
    config.load_json(json4, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(list) = map.get("list") {
            if let ContextValue::Array(arr, _) = list {
                assert_eq!(arr.len(), 4);
                if let ContextValue::String(s, _) = &arr[0] {
                    assert_eq!(s, "z", "__toprepend should add to start of array");
                }
            }
        }
    }
}

#[test]
fn test_toreplace_modifier() {
    let mut config = Config::default();

    let json1 = r#"{
        "obj": {
            "a": 1,
            "b": 2,
            "c": 3
        }
    }"#;
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));

    // Use __toreplace to completely replace the object
    let json2 = r#"{
        "obj__toreplace": {
            "x": 10,
            "y": 20
        }
    }"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(obj) = map.get("obj") {
            if let ContextValue::Object(obj_map, _) = obj {
                assert_eq!(
                    obj_map.len(),
                    2,
                    "obj should only have 2 fields after __toreplace"
                );
                assert!(obj_map.contains_key("x"));
                assert!(obj_map.contains_key("y"));
                assert!(!obj_map.contains_key("a"));
                assert!(!obj_map.contains_key("b"));
                assert!(!obj_map.contains_key("c"));
            }
        }
    }
}

#[test]
fn test_tokeep_with_missing_value() {
    let mut config = Config::default();

    // __tokeep when no existing value should set the value
    let json = r#"{"new_field__tokeep": "initial"}"#;
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(field) = map.get("new_field") {
            if let ContextValue::String(s, _) = field {
                assert_eq!(s, "initial", "__tokeep should set value when none exists");
            }
        }
    }
}

#[test]
fn test_toappend_array_of_values() {
    let mut config = Config::default();

    let json1 = r#"{"list": ["a"]}"#;
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));

    // Append an array - the array is appended as a single element (nested)
    // Note: __toappend appends one element at a time, arrays are nested
    let json2 = r#"{"list__toappend": ["b", "c"]}"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(list) = map.get("list") {
            if let ContextValue::Array(arr, _) = list {
                // The array ["b", "c"] is appended as a single nested element
                assert_eq!(
                    arr.len(),
                    2,
                    "Should have 2 elements (original + appended array)"
                );
            }
        }
    }
}

#[test]
fn test_toprepend_array_of_values() {
    let mut config = Config::default();

    let json1 = r#"{"list": ["c"]}"#;
    config.load_json(json1, Context::new(Source::Programmatic, Level::System));

    // Prepend an array - the array is prepended as a single element (nested)
    // Note: __toprepend prepends one element at a time, arrays are nested
    let json2 = r#"{"list__toprepend": ["a", "b"]}"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    if let ContextValue::Object(map, _) = config.root() {
        if let Some(list) = map.get("list") {
            if let ContextValue::Array(arr, _) = list {
                // The array ["a", "b"] is prepended as a single nested element
                assert_eq!(
                    arr.len(),
                    2,
                    "Should have 2 elements (prepended array + original)"
                );
            }
        }
    }
}

//! Unit tests for de module (deserialization / FromContextValue implementations).
//!
//! Extracted from compote/src/de.rs

use compote::error::ErrorTracker;
use compote::{Context, ContextValue, FromContextValue, Level, Source};
use std::path::PathBuf;

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

#[test]
fn test_string_deserialization() {
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::string("hello", test_context());
    let result = String::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_bool_deserialization() {
    let mut tracker = ErrorTracker::new();

    let value = ContextValue::bool(true, test_context());
    let result = bool::from_context_value(&value, &mut tracker).unwrap();
    assert!(result);

    let value = ContextValue::string("yes", test_context());
    let result = bool::from_context_value(&value, &mut tracker).unwrap();
    assert!(result);

    let value = ContextValue::string("false", test_context());
    let result = bool::from_context_value(&value, &mut tracker).unwrap();
    assert!(!result);
}

#[test]
fn test_int_deserialization() {
    let mut tracker = ErrorTracker::new();

    let value = ContextValue::int(42, test_context());
    let result = i64::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 42);

    let value = ContextValue::string("123", test_context());
    let result = i64::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 123);
}

#[test]
fn test_float_deserialization() {
    let mut tracker = ErrorTracker::new();

    let value = ContextValue::float(3.14, test_context());
    let result = f64::from_context_value(&value, &mut tracker).unwrap();
    assert!((result - 3.14).abs() < 0.001);

    let value = ContextValue::int(42, test_context());
    let result = f64::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 42.0);
}

#[test]
fn test_vec_deserialization() {
    let mut tracker = ErrorTracker::new();

    let arr = vec![
        ContextValue::int(1, test_context()),
        ContextValue::int(2, test_context()),
        ContextValue::int(3, test_context()),
    ];
    let value = ContextValue::array(arr, test_context());
    let result = Vec::<i64>::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_option_deserialization() {
    let mut tracker = ErrorTracker::new();

    let value = ContextValue::null(test_context());
    let result = Option::<i64>::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, None);

    let value = ContextValue::int(42, test_context());
    let result = Option::<i64>::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, Some(42));
}

#[test]
fn test_i16_deserialization() {
    let mut tracker = ErrorTracker::new();

    // Valid i16 value
    let value = ContextValue::int(1000, test_context());
    let result = i16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 1000);

    // Negative value
    let value = ContextValue::int(-1000, test_context());
    let result = i16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, -1000);

    // Max i16
    let value = ContextValue::int(i16::MAX as i64, test_context());
    let result = i16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, i16::MAX);

    // Min i16
    let value = ContextValue::int(i16::MIN as i64, test_context());
    let result = i16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, i16::MIN);

    // Out of range (too large)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(i16::MAX as i64 + 1, test_context());
    let result = i16::from_context_value(&value, &mut tracker);
    assert!(result.is_err());

    // Out of range (too small)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(i16::MIN as i64 - 1, test_context());
    let result = i16::from_context_value(&value, &mut tracker);
    assert!(result.is_err());
}

#[test]
fn test_u16_deserialization() {
    let mut tracker = ErrorTracker::new();

    // Valid u16 value
    let value = ContextValue::int(1000, test_context());
    let result = u16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 1000);

    // Max u16
    let value = ContextValue::int(u16::MAX as i64, test_context());
    let result = u16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, u16::MAX);

    // Zero
    let value = ContextValue::int(0, test_context());
    let result = u16::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 0);

    // Out of range (too large)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(u16::MAX as i64 + 1, test_context());
    let result = u16::from_context_value(&value, &mut tracker);
    assert!(result.is_err());

    // Out of range (negative)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(-1, test_context());
    let result = u16::from_context_value(&value, &mut tracker);
    assert!(result.is_err());
}

#[test]
fn test_i8_deserialization() {
    let mut tracker = ErrorTracker::new();

    // Valid i8 value
    let value = ContextValue::int(100, test_context());
    let result = i8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 100);

    // Negative value
    let value = ContextValue::int(-100, test_context());
    let result = i8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, -100);

    // Max i8
    let value = ContextValue::int(i8::MAX as i64, test_context());
    let result = i8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, i8::MAX);

    // Min i8
    let value = ContextValue::int(i8::MIN as i64, test_context());
    let result = i8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, i8::MIN);

    // Out of range (too large)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(i8::MAX as i64 + 1, test_context());
    let result = i8::from_context_value(&value, &mut tracker);
    assert!(result.is_err());

    // Out of range (too small)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(i8::MIN as i64 - 1, test_context());
    let result = i8::from_context_value(&value, &mut tracker);
    assert!(result.is_err());
}

#[test]
fn test_u8_deserialization() {
    let mut tracker = ErrorTracker::new();

    // Valid u8 value
    let value = ContextValue::int(100, test_context());
    let result = u8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 100);

    // Max u8
    let value = ContextValue::int(u8::MAX as i64, test_context());
    let result = u8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, u8::MAX);

    // Zero
    let value = ContextValue::int(0, test_context());
    let result = u8::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, 0);

    // Out of range (too large)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(u8::MAX as i64 + 1, test_context());
    let result = u8::from_context_value(&value, &mut tracker);
    assert!(result.is_err());

    // Out of range (negative)
    let mut tracker = ErrorTracker::new();
    let value = ContextValue::int(-1, test_context());
    let result = u8::from_context_value(&value, &mut tracker);
    assert!(result.is_err());
}

#[test]
fn test_pathbuf_deserialization() {
    let mut tracker = ErrorTracker::new();

    // Test basic path
    let value = ContextValue::string("/home/user/config.yaml", test_context());
    let result = PathBuf::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, PathBuf::from("/home/user/config.yaml"));

    // Test relative path
    let value = ContextValue::string("./relative/path", test_context());
    let result = PathBuf::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, PathBuf::from("./relative/path"));

    // Test empty path
    let value = ContextValue::string("", test_context());
    let result = PathBuf::from_context_value(&value, &mut tracker).unwrap();
    assert_eq!(result, PathBuf::from(""));
}

#[test]
fn test_pathbuf_type_mismatch() {
    use compote::error::Error;

    let mut tracker = ErrorTracker::new();

    // Test type mismatch with integer
    let value = ContextValue::int(42, test_context());
    let result = PathBuf::from_context_value(&value, &mut tracker);
    assert!(result.is_err());

    match result {
        Err(Error::TypeMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, "string (path)");
            assert_eq!(actual, "int");
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_hashmap_deserialization() {
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut tracker = ErrorTracker::new();

    // Create an object/map ContextValue
    let mut fields = IndexMap::new();
    fields.insert(
        "key1".to_string(),
        ContextValue::string("value1", test_context()),
    );
    fields.insert(
        "key2".to_string(),
        ContextValue::string("value2", test_context()),
    );
    fields.insert(
        "key3".to_string(),
        ContextValue::string("value3", test_context()),
    );

    let value = ContextValue::object(fields, test_context());
    let result = HashMap::<String, String>::from_context_value(&value, &mut tracker).unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("key1"), Some(&"value1".to_string()));
    assert_eq!(result.get("key2"), Some(&"value2".to_string()));
    assert_eq!(result.get("key3"), Some(&"value3".to_string()));
}

#[test]
fn test_hashmap_with_integer_values() {
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut tracker = ErrorTracker::new();

    let mut fields = IndexMap::new();
    fields.insert("one".to_string(), ContextValue::int(1, test_context()));
    fields.insert("two".to_string(), ContextValue::int(2, test_context()));
    fields.insert("three".to_string(), ContextValue::int(3, test_context()));

    let value = ContextValue::object(fields, test_context());
    let result = HashMap::<String, i64>::from_context_value(&value, &mut tracker).unwrap();

    assert_eq!(result.len(), 3);
    assert_eq!(result.get("one"), Some(&1));
    assert_eq!(result.get("two"), Some(&2));
    assert_eq!(result.get("three"), Some(&3));
}

#[test]
fn test_hashmap_empty() {
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut tracker = ErrorTracker::new();

    let fields = IndexMap::new();
    let value = ContextValue::object(fields, test_context());
    let result = HashMap::<String, String>::from_context_value(&value, &mut tracker).unwrap();

    assert!(result.is_empty());
}

#[test]
fn test_hashmap_type_mismatch() {
    use compote::error::Error;
    use std::collections::HashMap;

    let mut tracker = ErrorTracker::new();

    // Try to deserialize an array as a HashMap - should fail
    let arr = vec![
        ContextValue::string("a", test_context()),
        ContextValue::string("b", test_context()),
    ];
    let value = ContextValue::array(arr, test_context());
    let result = HashMap::<String, String>::from_context_value(&value, &mut tracker);

    assert!(result.is_err());
    match result {
        Err(Error::TypeMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, "object");
            assert_eq!(actual, "array");
        }
        _ => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn test_hashmap_with_invalid_value() {
    use indexmap::IndexMap;
    use std::collections::HashMap;

    let mut tracker = ErrorTracker::new();

    // Create a map where one value is the wrong type
    let mut fields = IndexMap::new();
    fields.insert("valid".to_string(), ContextValue::int(42, test_context()));
    fields.insert(
        "invalid".to_string(),
        ContextValue::string("not_a_number", test_context()),
    );

    let value = ContextValue::object(fields, test_context());
    // Parsing "not_a_number" as i64 should fail, but the valid entry should still be present
    let result = HashMap::<String, i64>::from_context_value(&value, &mut tracker).unwrap();

    // The valid entry should be in the result
    assert_eq!(result.get("valid"), Some(&42));
    // The invalid entry should be skipped (error was recorded)
    assert_eq!(result.get("invalid"), None);
    // An error should have been recorded
    assert!(tracker.has_errors());
}

#[test]
fn test_hashbrown_hashmap_deserialization() {
    use hashbrown::HashMap;
    use indexmap::IndexMap;

    let mut tracker = ErrorTracker::new();

    let mut fields = IndexMap::new();
    fields.insert(
        "foo".to_string(),
        ContextValue::string("bar", test_context()),
    );
    fields.insert(
        "baz".to_string(),
        ContextValue::string("qux", test_context()),
    );

    let value = ContextValue::object(fields, test_context());
    let result = HashMap::<String, String>::from_context_value(&value, &mut tracker).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result.get("foo"), Some(&"bar".to_string()));
    assert_eq!(result.get("baz"), Some(&"qux".to_string()));
}

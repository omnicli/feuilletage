//! Unit tests for transform module (transformation functions and registry).
//!
//! Extracted from feuilletage/src/transform.rs

use feuilletage::transform::{
    expand_env_vars, parse_duration, parse_duration_f64, parse_duration_ms,
    parse_duration_to_ms_f64, parse_duration_to_ms_u64, parse_duration_to_nanos,
    parse_duration_to_secs, parse_duration_to_secs_f64, parse_duration_to_secs_u64,
    parse_duration_u64, relative_path, to_uppercase, trim, unit_to_nanos, TransformRegistry,
};
use feuilletage::{Context, ContextValue, Level, Source, Value};
use indexmap::IndexMap;
use std::path::PathBuf;

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

#[test]
fn test_exact_path_transform() {
    let mut registry = TransformRegistry::new();
    registry.register_exact("test.path", to_uppercase);

    let mut value = ContextValue::string("hello", test_context());
    registry
        .apply("test.path", &mut value, &test_context())
        .unwrap();

    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "HELLO");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_pattern_transform() {
    let mut registry = TransformRegistry::new();
    registry.register_pattern("*.password", to_uppercase);

    let mut value = ContextValue::string("secret", test_context());
    registry
        .apply("db.password", &mut value, &test_context())
        .unwrap();

    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "SECRET");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_wildcard_pattern() {
    let mut registry = TransformRegistry::new();
    registry.register_pattern("**", trim);

    let mut value = ContextValue::string("  spaced  ", test_context());
    registry
        .apply("any.path", &mut value, &test_context())
        .unwrap();

    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "spaced");
    } else {
        panic!("Expected string");
    }
}

#[test]
#[serial_test::serial]
fn test_env_var_expansion() {
    std::env::set_var("TEST_VAR", "expanded_value");

    let mut value = ContextValue::string("prefix_${TEST_VAR}_suffix", test_context());
    expand_env_vars(&mut value, &test_context()).unwrap();

    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "prefix_expanded_value_suffix");
    } else {
        panic!("Expected string");
    }

    std::env::remove_var("TEST_VAR");
}

#[test]
fn test_tree_transformation() {
    let mut registry = TransformRegistry::new();
    registry.register_pattern("*.name", to_uppercase);

    let mut inner = IndexMap::new();
    inner.insert(
        "name".to_string(),
        ContextValue::string("test", test_context()),
    );
    inner.insert("value".to_string(), ContextValue::int(42, test_context()));

    let mut root = IndexMap::new();
    root.insert(
        "data".to_string(),
        ContextValue::object(inner, test_context()),
    );

    let mut config = ContextValue::object(root, test_context());

    registry.apply_to_tree(&mut config, "").unwrap();

    // Check that data.name was transformed
    if let ContextValue::Object(map, _) = &config {
        if let Some(data) = map.get("data") {
            if let ContextValue::Object(data_map, _) = data {
                if let Some(name) = data_map.get("name") {
                    if let ContextValue::String(s, _) = name {
                        assert_eq!(s, "TEST");
                    } else {
                        panic!("Expected string");
                    }
                }
            }
        }
    }
}

#[test]
fn test_relative_path_transform() {
    // Test with a relative path
    let source_file = PathBuf::from("/a/b/config.toml");
    let context: Context = Context::new(Source::File(source_file), Level::User);

    let mut value = ContextValue::string("xx/yy", context.clone());
    relative_path(&mut value, &context).unwrap();

    if let ContextValue::String(s, _) = &value {
        // Should be resolved to absolute path
        assert!(
            s.ends_with("a/b/xx/yy") || s.ends_with("a\\b\\xx\\yy"),
            "Got: {}",
            s
        );
    } else {
        panic!("Expected string");
    }

    // Test with an absolute path (should not change)
    let mut value2 = ContextValue::string("/absolute/path", context.clone());
    relative_path(&mut value2, &context).unwrap();

    if let ContextValue::String(s, _) = &value2 {
        assert_eq!(s, "/absolute/path");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_relative_path_with_non_file_source() {
    // Test with a non-file source (should succeed and leave unchanged)
    let context: Context = Context::new(Source::Programmatic, Level::User);

    let mut value = ContextValue::string("relative/path", context.clone());
    let result = relative_path(&mut value, &context);

    // Should succeed and leave the relative path unchanged (no file source to resolve against)
    assert!(result.is_ok());
    // Value should remain unchanged
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "relative/path");
    } else {
        panic!("Expected String value");
    }
}

#[test]
fn test_relative_path_with_absolute_and_non_file_source() {
    // Test with an absolute path and non-file source (should succeed)
    let context: Context = Context::new(Source::Programmatic, Level::User);

    let mut value = ContextValue::string("/absolute/path", context.clone());
    let result = relative_path(&mut value, &context);

    // Should succeed because absolute paths don't need resolution
    assert!(result.is_ok());
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "/absolute/path");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_removes_dot() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::string("foo/./bar".to_string(), context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "foo/bar");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_resolves_dotdot() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::string("foo/bar/../baz".to_string(), context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "foo/baz");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_preserves_absolute() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::string("/foo/bar/../baz".to_string(), context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "/foo/baz");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_leading_dotdot() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::string("../foo/bar".to_string(), context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, "../foo/bar");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_all_dots() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::string("./".to_string(), context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::String(s, _) = &value {
        assert_eq!(s, ".");
    } else {
        panic!("Expected string");
    }
}

#[test]
fn test_normalize_path_non_string_unchanged() {
    use feuilletage::transform::normalize_path;

    let context: Context = Context::new(Source::Programmatic, Level::User);
    let mut value = ContextValue::int(42, context.clone());
    normalize_path(&mut value, &context).unwrap();
    if let ContextValue::Int(n, _) = &value {
        assert_eq!(*n, 42);
    } else {
        panic!("Expected int");
    }
}

#[test]
fn test_parse_duration_seconds() {
    assert_eq!(parse_duration_to_secs("30s").unwrap(), 30);
    assert_eq!(parse_duration_to_secs("0s").unwrap(), 0);
    assert_eq!(parse_duration_to_secs("100").unwrap(), 100);
}

#[test]
fn test_parse_duration_minutes() {
    assert_eq!(parse_duration_to_secs("5m").unwrap(), 300);
    assert_eq!(parse_duration_to_secs("1m").unwrap(), 60);
}

#[test]
fn test_parse_duration_hours() {
    assert_eq!(parse_duration_to_secs("2h").unwrap(), 7200);
    assert_eq!(parse_duration_to_secs("1h").unwrap(), 3600);
}

#[test]
fn test_parse_duration_days() {
    assert_eq!(parse_duration_to_secs("1d").unwrap(), 86400);
    assert_eq!(parse_duration_to_secs("7d").unwrap(), 604800);
}

#[test]
fn test_parse_duration_weeks() {
    assert_eq!(parse_duration_to_secs("1w").unwrap(), 604800);
    assert_eq!(parse_duration_to_secs("2w").unwrap(), 1209600);
}

#[test]
fn test_parse_duration_combined() {
    assert_eq!(parse_duration_to_secs("1h30m").unwrap(), 5400);
    assert_eq!(parse_duration_to_secs("2d12h").unwrap(), 216000);
    assert_eq!(parse_duration_to_secs("1w2d3h4m5s").unwrap(), 788645);
}

#[test]
fn test_parse_duration_transform() {
    let mut value = ContextValue::string("5m", test_context());
    parse_duration(&mut value, &test_context()).unwrap();

    if let ContextValue::Int(secs, _) = &value {
        assert_eq!(*secs, 300);
    } else {
        panic!("Expected Int");
    }
}

#[test]
fn test_parse_duration_invalid() {
    assert!(parse_duration_to_secs("").is_err());
    assert!(parse_duration_to_secs("abc").is_err());
    assert!(parse_duration_to_secs("5x").is_err());
}

// ============================================================================
// Comprehensive Duration Parsing Tests
// ============================================================================

// --- Nanoseconds ---

#[test]
fn test_parse_duration_nanoseconds() {
    // Test ns unit
    assert_eq!(parse_duration_to_secs_u64("1000000000ns").unwrap(), 1);
    assert_eq!(parse_duration_to_secs_u64("500000000ns").unwrap(), 0); // Truncated
    assert_eq!(parse_duration_to_ms_u64("1000000ns").unwrap(), 1);
    assert_eq!(parse_duration_to_ms_u64("500000ns").unwrap(), 0); // Truncated

    // f64 versions for precision
    let secs = parse_duration_to_secs_f64("1000ns").unwrap();
    assert!(
        (secs - 0.000001).abs() < 1e-12,
        "Expected 0.000001, got {}",
        secs
    );

    let ms = parse_duration_to_ms_f64("1000ns").unwrap();
    assert!((ms - 0.001).abs() < 1e-9, "Expected 0.001, got {}", ms);
}

// --- Microseconds ---

#[test]
fn test_parse_duration_microseconds_ascii() {
    // Test ASCII 'us' unit
    assert_eq!(parse_duration_to_secs_u64("1000000us").unwrap(), 1);
    assert_eq!(parse_duration_to_secs_u64("500000us").unwrap(), 0); // Truncated
    assert_eq!(parse_duration_to_ms_u64("1000us").unwrap(), 1);
    assert_eq!(parse_duration_to_ms_u64("500us").unwrap(), 0); // Truncated

    // f64 versions for precision
    let secs = parse_duration_to_secs_f64("100us").unwrap();
    assert!(
        (secs - 0.0001).abs() < 1e-9,
        "Expected 0.0001, got {}",
        secs
    );

    let ms = parse_duration_to_ms_f64("100us").unwrap();
    assert!((ms - 0.1).abs() < 1e-9, "Expected 0.1, got {}", ms);
}

#[test]
fn test_parse_duration_microseconds_unicode_micro_sign() {
    // Test Unicode micro sign (U+00B5) - the canonical micro symbol
    assert_eq!(parse_duration_to_secs_u64("1000000\u{00B5}s").unwrap(), 1);
    assert_eq!(parse_duration_to_ms_u64("1000\u{00B5}s").unwrap(), 1);

    let secs = parse_duration_to_secs_f64("100\u{00B5}s").unwrap();
    assert!(
        (secs - 0.0001).abs() < 1e-9,
        "Expected 0.0001, got {}",
        secs
    );
}

#[test]
fn test_parse_duration_microseconds_unicode_greek_mu() {
    // Test Greek letter mu (U+03BC)
    assert_eq!(parse_duration_to_secs_u64("1000000\u{03BC}s").unwrap(), 1);
    assert_eq!(parse_duration_to_ms_u64("1000\u{03BC}s").unwrap(), 1);

    let secs = parse_duration_to_secs_f64("100\u{03BC}s").unwrap();
    assert!(
        (secs - 0.0001).abs() < 1e-9,
        "Expected 0.0001, got {}",
        secs
    );
}

// --- Milliseconds ---

#[test]
fn test_parse_duration_milliseconds() {
    assert_eq!(parse_duration_to_secs_u64("1000ms").unwrap(), 1);
    assert_eq!(parse_duration_to_secs_u64("500ms").unwrap(), 0); // Truncated
    assert_eq!(parse_duration_to_secs_u64("1500ms").unwrap(), 1); // 1.5s truncated to 1

    assert_eq!(parse_duration_to_ms_u64("500ms").unwrap(), 500);
    assert_eq!(parse_duration_to_ms_u64("1000ms").unwrap(), 1000);

    let secs = parse_duration_to_secs_f64("500ms").unwrap();
    assert!((secs - 0.5).abs() < 1e-9, "Expected 0.5, got {}", secs);
}

// --- Seconds ---

#[test]
fn test_parse_duration_seconds_comprehensive() {
    assert_eq!(parse_duration_to_secs_u64("30s").unwrap(), 30);
    assert_eq!(parse_duration_to_secs_u64("0s").unwrap(), 0);
    assert_eq!(parse_duration_to_secs_u64("1s").unwrap(), 1);

    assert_eq!(parse_duration_to_ms_u64("1s").unwrap(), 1000);
    assert_eq!(parse_duration_to_ms_u64("5s").unwrap(), 5000);

    // Plain number = seconds
    assert_eq!(parse_duration_to_secs_u64("100").unwrap(), 100);
    assert_eq!(parse_duration_to_ms_u64("100").unwrap(), 100000);
}

// --- Minutes ---

#[test]
fn test_parse_duration_minutes_comprehensive() {
    assert_eq!(parse_duration_to_secs_u64("1m").unwrap(), 60);
    assert_eq!(parse_duration_to_secs_u64("5m").unwrap(), 300);
    assert_eq!(parse_duration_to_secs_u64("60m").unwrap(), 3600);

    assert_eq!(parse_duration_to_ms_u64("1m").unwrap(), 60000);
    assert_eq!(parse_duration_to_ms_u64("5m").unwrap(), 300000);
}

// --- Hours ---

#[test]
fn test_parse_duration_hours_comprehensive() {
    assert_eq!(parse_duration_to_secs_u64("1h").unwrap(), 3600);
    assert_eq!(parse_duration_to_secs_u64("2h").unwrap(), 7200);
    assert_eq!(parse_duration_to_secs_u64("24h").unwrap(), 86400);

    assert_eq!(parse_duration_to_ms_u64("1h").unwrap(), 3600000);
}

// --- Days ---

#[test]
fn test_parse_duration_days_comprehensive() {
    assert_eq!(parse_duration_to_secs_u64("1d").unwrap(), 86400);
    assert_eq!(parse_duration_to_secs_u64("7d").unwrap(), 604800);

    assert_eq!(parse_duration_to_ms_u64("1d").unwrap(), 86400000);
}

// --- Weeks ---

#[test]
fn test_parse_duration_weeks_comprehensive() {
    assert_eq!(parse_duration_to_secs_u64("1w").unwrap(), 604800);
    assert_eq!(parse_duration_to_secs_u64("2w").unwrap(), 1209600);

    assert_eq!(parse_duration_to_ms_u64("1w").unwrap(), 604800000);
}

// --- Combined Durations ---

#[test]
fn test_parse_duration_combined_comprehensive() {
    // Basic combinations
    assert_eq!(parse_duration_to_secs_u64("1h30m").unwrap(), 5400);
    assert_eq!(parse_duration_to_secs_u64("2d12h").unwrap(), 216000);
    assert_eq!(parse_duration_to_secs_u64("1w2d3h4m5s").unwrap(), 788645);

    // With milliseconds
    assert_eq!(parse_duration_to_ms_u64("1s500ms").unwrap(), 1500);
    assert_eq!(parse_duration_to_ms_u64("1h30m500ms").unwrap(), 5400500);

    // With microseconds
    assert_eq!(parse_duration_to_ms_u64("1s500ms250us").unwrap(), 1500); // 250us truncated

    // f64 for precision
    let ms = parse_duration_to_ms_f64("1s500ms250us").unwrap();
    assert!((ms - 1500.25).abs() < 1e-6, "Expected 1500.25, got {}", ms);

    // Complex combination with all units
    let complex = parse_duration_to_secs_f64("1h30m500ms100us").unwrap();
    let expected = 3600.0 + 1800.0 + 0.5 + 0.0001;
    assert!(
        (complex - expected).abs() < 1e-6,
        "Expected {}, got {}",
        expected,
        complex
    );
}

// --- Edge Cases ---

#[test]
fn test_parse_duration_zero() {
    assert_eq!(parse_duration_to_secs_u64("0").unwrap(), 0);
    assert_eq!(parse_duration_to_secs_u64("0s").unwrap(), 0);
    assert_eq!(parse_duration_to_secs_u64("0ms").unwrap(), 0);
    assert_eq!(parse_duration_to_secs_u64("0ns").unwrap(), 0);

    assert_eq!(parse_duration_to_ms_u64("0").unwrap(), 0);
    assert_eq!(parse_duration_to_ms_u64("0ms").unwrap(), 0);
}

#[test]
fn test_parse_duration_very_large_values() {
    // 1 year in seconds (approximately)
    assert_eq!(parse_duration_to_secs_u64("52w").unwrap(), 52 * 604800);

    // Large millisecond values
    assert_eq!(
        parse_duration_to_ms_u64("365d").unwrap(),
        365 * 86400 * 1000
    );
}

#[test]
fn test_parse_duration_very_small_values() {
    // Single nanosecond - should round to 0 in integer forms
    assert_eq!(parse_duration_to_secs_u64("1ns").unwrap(), 0);
    assert_eq!(parse_duration_to_ms_u64("1ns").unwrap(), 0);

    // But f64 should preserve precision
    let secs = parse_duration_to_secs_f64("1ns").unwrap();
    assert!((secs - 1e-9).abs() < 1e-15, "Expected 1e-9, got {}", secs);

    let ms = parse_duration_to_ms_f64("1ns").unwrap();
    assert!((ms - 1e-6).abs() < 1e-12, "Expected 1e-6, got {}", ms);
}

#[test]
fn test_parse_duration_with_whitespace() {
    // Trimming should work
    assert_eq!(parse_duration_to_secs_u64("  30s  ").unwrap(), 30);
    assert_eq!(parse_duration_to_secs_u64("\t5m\n").unwrap(), 300);
}

#[test]
fn test_parse_duration_fractional_values() {
    // Fractional numbers
    let secs = parse_duration_to_secs_f64("1.5s").unwrap();
    assert!((secs - 1.5).abs() < 1e-9, "Expected 1.5, got {}", secs);

    let ms = parse_duration_to_ms_f64("1.5ms").unwrap();
    assert!((ms - 1.5).abs() < 1e-9, "Expected 1.5, got {}", ms);

    // Fractional with combined units
    let combined = parse_duration_to_secs_f64("1.5h").unwrap();
    assert!(
        (combined - 5400.0).abs() < 1e-9,
        "Expected 5400.0, got {}",
        combined
    );
}

// --- Error Cases ---

#[test]
fn test_parse_duration_errors() {
    // Empty string
    assert!(parse_duration_to_secs_u64("").is_err());

    // Invalid unit
    assert!(parse_duration_to_secs_u64("5x").is_err());
    assert!(parse_duration_to_secs_u64("5y").is_err());

    // No number before unit
    assert!(parse_duration_to_secs_u64("s").is_err());
    assert!(parse_duration_to_secs_u64("ms").is_err());

    // Invalid characters
    assert!(parse_duration_to_secs_u64("5s!").is_err());

    // Partial units (n without s, u without s)
    assert!(parse_duration_to_secs_u64("5n").is_err());
    assert!(parse_duration_to_secs_u64("5u").is_err());

    // Letters only
    assert!(parse_duration_to_secs_u64("abc").is_err());
}

// --- Transform Function Tests ---

#[test]
fn test_parse_duration_transform_comprehensive() {
    // Test with various formats
    let mut value = ContextValue::string("5m", test_context());
    parse_duration(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(300, _)));

    // With milliseconds (truncated to seconds)
    let mut value = ContextValue::string("1500ms", test_context());
    parse_duration(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(1, _)));

    // Already an integer
    let mut value = ContextValue::int(60, test_context());
    parse_duration(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(60, _)));
}

#[test]
fn test_parse_duration_ms_transform() {
    // Test parse_duration_ms transform function
    let mut value = ContextValue::string("5s", test_context());
    parse_duration_ms(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(5000, _)));

    // With milliseconds
    let mut value = ContextValue::string("500ms", test_context());
    parse_duration_ms(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(500, _)));

    // Combined
    let mut value = ContextValue::string("1s500ms", test_context());
    parse_duration_ms(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(1500, _)));

    // Already an integer (assumed to be milliseconds)
    let mut value = ContextValue::int(1000, test_context());
    parse_duration_ms(&mut value, &test_context()).unwrap();
    assert!(matches!(&value, ContextValue::Int(1000, _)));
}

#[test]
fn test_parse_duration_transform_errors() {
    // Invalid string
    let mut value = ContextValue::string("invalid", test_context());
    assert!(parse_duration(&mut value, &test_context()).is_err());

    // Invalid type (float)
    let mut value = ContextValue::new(Value::Float(1.5), test_context());
    assert!(parse_duration(&mut value, &test_context()).is_err());

    // Invalid type (bool)
    let mut value = ContextValue::bool(true, test_context());
    assert!(parse_duration(&mut value, &test_context()).is_err());
}

#[test]
fn test_parse_duration_ms_transform_errors() {
    // Invalid string
    let mut value = ContextValue::string("invalid", test_context());
    assert!(parse_duration_ms(&mut value, &test_context()).is_err());

    // Invalid type (float)
    let mut value = ContextValue::new(Value::Float(1.5), test_context());
    assert!(parse_duration_ms(&mut value, &test_context()).is_err());
}

// --- Backward Compatibility ---

#[test]
fn test_parse_duration_to_secs_backward_compat() {
    // Ensure parse_duration_to_secs works exactly like parse_duration_to_secs_u64
    assert_eq!(
        parse_duration_to_secs("30s").unwrap(),
        parse_duration_to_secs_u64("30s").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("5m").unwrap(),
        parse_duration_to_secs_u64("5m").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("2h").unwrap(),
        parse_duration_to_secs_u64("2h").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("1d").unwrap(),
        parse_duration_to_secs_u64("1d").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("1w").unwrap(),
        parse_duration_to_secs_u64("1w").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("1h30m").unwrap(),
        parse_duration_to_secs_u64("1h30m").unwrap()
    );
    assert_eq!(
        parse_duration_to_secs("500ms").unwrap(),
        parse_duration_to_secs_u64("500ms").unwrap()
    );
}

// ============================================================================
// New API: parse_duration_u64/f64 with unit parameter
// ============================================================================

#[test]
fn test_unit_to_nanos() {
    // Test all supported units
    assert!((unit_to_nanos("ns").unwrap() - 1.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("us").unwrap() - 1_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("ms").unwrap() - 1_000_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("s").unwrap() - 1_000_000_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("m").unwrap() - 60_000_000_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("h").unwrap() - 3_600_000_000_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("d").unwrap() - 86_400_000_000_000.0).abs() < f64::EPSILON);
    assert!((unit_to_nanos("w").unwrap() - 604_800_000_000_000.0).abs() < f64::EPSILON);

    // Test invalid unit
    assert!(unit_to_nanos("x").is_err());
    assert!(unit_to_nanos("invalid").is_err());
    assert!(unit_to_nanos("").is_err());
}

#[test]
fn test_parse_duration_to_nanos_public() {
    // Test that parse_duration_to_nanos is accessible
    let nanos = parse_duration_to_nanos("1s").unwrap();
    assert!((nanos - 1_000_000_000.0).abs() < f64::EPSILON);

    let nanos = parse_duration_to_nanos("500ms").unwrap();
    assert!((nanos - 500_000_000.0).abs() < f64::EPSILON);

    let nanos = parse_duration_to_nanos("1m").unwrap();
    assert!((nanos - 60_000_000_000.0).abs() < f64::EPSILON);
}

#[test]
fn test_parse_duration_u64_basic() {
    // Test basic parsing to various units
    assert_eq!(parse_duration_u64("1h30m", "s").unwrap(), 5400);
    assert_eq!(parse_duration_u64("5s", "ms").unwrap(), 5000);
    assert_eq!(parse_duration_u64("500ms", "ms").unwrap(), 500);
    assert_eq!(parse_duration_u64("2h", "m").unwrap(), 120);
    assert_eq!(parse_duration_u64("1ms", "ns").unwrap(), 1_000_000);
    assert_eq!(parse_duration_u64("1s", "us").unwrap(), 1_000_000);
}

#[test]
fn test_parse_duration_u64_truncation() {
    // Test that fractional values are truncated
    assert_eq!(parse_duration_u64("500ms", "s").unwrap(), 0);
    assert_eq!(parse_duration_u64("1500ms", "s").unwrap(), 1);
    assert_eq!(parse_duration_u64("90s", "m").unwrap(), 1); // 1.5 minutes truncated to 1
    assert_eq!(parse_duration_u64("45m", "h").unwrap(), 0); // 0.75 hours truncated to 0
}

#[test]
fn test_parse_duration_u64_all_units() {
    // Test all unit conversions
    assert_eq!(parse_duration_u64("1us", "ns").unwrap(), 1000);
    assert_eq!(parse_duration_u64("1ms", "us").unwrap(), 1000);
    assert_eq!(parse_duration_u64("1s", "ms").unwrap(), 1000);
    assert_eq!(parse_duration_u64("1m", "s").unwrap(), 60);
    assert_eq!(parse_duration_u64("1h", "m").unwrap(), 60);
    assert_eq!(parse_duration_u64("1d", "h").unwrap(), 24);
    assert_eq!(parse_duration_u64("1w", "d").unwrap(), 7);
    assert_eq!(parse_duration_u64("2w", "w").unwrap(), 2);
}

#[test]
fn test_parse_duration_f64_basic() {
    // Test basic parsing with precision preservation
    let result = parse_duration_f64("1s500ms", "s").unwrap();
    assert!(
        (result - 1.5).abs() < f64::EPSILON,
        "Expected 1.5, got {}",
        result
    );

    let result = parse_duration_f64("500ms", "s").unwrap();
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "Expected 0.5, got {}",
        result
    );

    let result = parse_duration_f64("1s", "ms").unwrap();
    assert!(
        (result - 1000.0).abs() < f64::EPSILON,
        "Expected 1000.0, got {}",
        result
    );

    let result = parse_duration_f64("100us", "ms").unwrap();
    assert!((result - 0.1).abs() < 1e-9, "Expected 0.1, got {}", result);

    let result = parse_duration_f64("90m", "h").unwrap();
    assert!(
        (result - 1.5).abs() < f64::EPSILON,
        "Expected 1.5, got {}",
        result
    );
}

#[test]
fn test_parse_duration_f64_all_units() {
    // Test all unit conversions with f64
    let result = parse_duration_f64("1ms", "s").unwrap();
    assert!(
        (result - 0.001).abs() < 1e-9,
        "Expected 0.001, got {}",
        result
    );

    let result = parse_duration_f64("30s", "m").unwrap();
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "Expected 0.5, got {}",
        result
    );

    let result = parse_duration_f64("30m", "h").unwrap();
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "Expected 0.5, got {}",
        result
    );

    let result = parse_duration_f64("12h", "d").unwrap();
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "Expected 0.5, got {}",
        result
    );

    let result = parse_duration_f64("3d12h", "w").unwrap();
    assert!(
        (result - 0.5).abs() < f64::EPSILON,
        "Expected 0.5, got {}",
        result
    );
}

#[test]
fn test_parse_duration_u64_errors() {
    // Test error cases
    assert!(parse_duration_u64("invalid", "s").is_err());
    assert!(parse_duration_u64("5s", "invalid_unit").is_err());
    assert!(parse_duration_u64("", "s").is_err());
    assert!(parse_duration_u64("5s", "").is_err());
}

#[test]
fn test_parse_duration_f64_errors() {
    // Test error cases
    assert!(parse_duration_f64("invalid", "s").is_err());
    assert!(parse_duration_f64("5s", "invalid_unit").is_err());
    assert!(parse_duration_f64("", "s").is_err());
    assert!(parse_duration_f64("5s", "").is_err());
}

#[test]
fn test_parse_duration_u64_large_values() {
    // Test large duration conversions
    assert_eq!(parse_duration_u64("365d", "h").unwrap(), 365 * 24);
    assert_eq!(parse_duration_u64("52w", "d").unwrap(), 52 * 7);
    assert_eq!(parse_duration_u64("1w", "s").unwrap(), 604800);
    assert_eq!(parse_duration_u64("1w", "ms").unwrap(), 604800000);
}

#[test]
fn test_parse_duration_u64_combined() {
    // Test complex combined durations
    assert_eq!(parse_duration_u64("1h30m500ms", "ms").unwrap(), 5400500);
    assert_eq!(parse_duration_u64("1d2h3m4s", "s").unwrap(), 93784);
    assert_eq!(parse_duration_u64("1w2d3h4m5s", "s").unwrap(), 788645);
}

//! Tests for the Edit API.
//!
//! These tests cover all aspects of the Edit API including:
//! - Navigation (at, at_raw, chaining)
//! - Path parsing (dots, escapes, arrays)
//! - Read operations (exists, get, get_mut, get_or, get_or_insert, is_null, is_array, is_object)
//! - Write operations (set, set_if_missing)
//! - Delete operations (remove, prune)
//! - Array operations (push, map, filter, map_where)

use feuilletage::{Config, Context, ContextValue, Level, Source, Value};

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

// =============================================================================
// Navigation Tests
// =============================================================================

mod navigation {
    use super::*;

    #[test]
    fn test_at_simple_path() {
        let mut config = Config::default();
        config.at("a.b.c").set(42).unwrap();

        assert!(matches!(
            config.at("a.b.c").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_at_chaining() {
        let mut config = Config::default();
        config.at("a").at("b").at("c").set(42).unwrap();

        assert!(matches!(
            config.at("a.b.c").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_at_mixed_chaining() {
        let mut config = Config::default();
        config.at("a.b").at("c").set(42).unwrap();

        assert!(matches!(
            config.at("a.b.c").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_at_raw_single_key() {
        let mut config = Config::default();
        config
            .at("plugins")
            .at_raw("auth.v2")
            .at("enabled")
            .set(true)
            .unwrap();

        // The key "auth.v2" should be a single key, not split
        assert!(config.at("plugins").at_raw("auth.v2").exists());
        assert!(matches!(
            config.at("plugins").at_raw("auth.v2").at("enabled").get(),
            Some(ContextValue::Bool(true, _))
        ));
    }

    #[test]
    fn test_at_array_path() {
        let mut config = Config::default();
        config.at(["a", "b.with.dots", "c"]).set(42).unwrap();

        // The middle key should contain literal dots
        assert!(config.at(["a", "b.with.dots", "c"]).exists());
        assert!(matches!(
            config.at(["a", "b.with.dots", "c"]).get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_at_escaped_dots() {
        let mut config = Config::default();
        config.at("a.b\\.c.d").set(42).unwrap();

        // "a.b\.c.d" should parse as ["a", "b.c", "d"]
        assert!(config.at(["a", "b.c", "d"]).exists());
        assert!(matches!(
            config.at(["a", "b.c", "d"]).get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_at_empty_path() {
        let mut config = Config::default();
        config.at("key").set("value").unwrap();

        // Empty path should return root
        let entry = config.at("");
        assert!(entry.exists());
        assert!(entry.is_object());
    }

    #[test]
    fn test_path_method() {
        let mut config = Config::default();
        let entry = config.at("a.b.c");

        assert_eq!(entry.path(), &["a", "b", "c"]);
    }
}

// =============================================================================
// Read Operation Tests
// =============================================================================

mod read_operations {
    use super::*;

    #[test]
    #[cfg(feature = "json")]
    fn test_exists_true() {
        let mut config = Config::default();
        config.load_json(r#"{"a": {"b": 42}}"#, test_context());

        assert!(config.at("a").exists());
        assert!(config.at("a.b").exists());
    }

    #[test]
    fn test_exists_false() {
        let mut config = Config::default();

        assert!(!config.at("nonexistent").exists());
        assert!(!config.at("a.b.c").exists());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_exists_with_null() {
        let mut config = Config::default();
        config.load_json(r#"{"a": null}"#, test_context());

        // Key exists even though value is null
        assert!(config.at("a").exists());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_get_value() {
        let mut config = Config::default();
        config.load_json(r#"{"name": "test", "count": 42}"#, test_context());

        assert!(matches!(config.at("name").get(), Some(ContextValue::String(s, _)) if s == "test"));
        assert!(matches!(
            config.at("count").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_get_none() {
        let mut config = Config::default();

        assert!(config.at("nonexistent").get().is_none());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_get_mut() {
        let mut config = Config::default();
        config.load_json(r#"{"value": 42}"#, test_context());

        if let Some(ContextValue::Int(ref mut i, _)) = config.at("value").get_config_mut() {
            *i = 100;
        }

        assert!(matches!(
            config.at("value").get(),
            Some(ContextValue::Int(100, _))
        ));
    }

    #[test]
    fn test_get_or_with_value() {
        let mut config = Config::default();
        config.at("key").set(42).unwrap();

        let value = config.at("key").get_or(Value::Int(0));
        assert!(matches!(value, Value::Int(42)));
    }

    #[test]
    fn test_get_or_with_missing() {
        let mut config = Config::default();

        let value = config.at("missing").get_or(Value::Int(0));
        assert!(matches!(value, Value::Int(0)));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_get_or_with_null() {
        let mut config = Config::default();
        config.load_json(r#"{"key": null}"#, test_context());

        // Null should return default
        let value = config.at("key").get_or(Value::Int(0));
        assert!(matches!(value, Value::Int(0)));
    }

    #[test]
    fn test_get_or_insert_creates_value() {
        let mut config = Config::default();

        {
            let mut entry = config.at("new_key");
            let value = entry.get_or_insert(Value::Int(42));
            assert!(matches!(value, ContextValue::Int(42, _)));
        }

        // Value should now exist
        assert!(config.at("new_key").exists());
        assert!(matches!(
            config.at("new_key").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_get_or_insert_returns_existing() {
        let mut config = Config::default();
        config.at("key").set(100).unwrap();

        {
            let mut entry = config.at("key");
            let value = entry.get_or_insert(Value::Int(42));
            assert!(matches!(value, ContextValue::Int(100, _)));
        }
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_is_null() {
        let mut config = Config::default();
        config.load_json(r#"{"null_val": null, "int_val": 42}"#, test_context());

        assert!(config.at("null_val").is_null());
        assert!(!config.at("int_val").is_null());
        assert!(!config.at("missing").is_null());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_is_array() {
        let mut config = Config::default();
        config.load_json(r#"{"arr": [1, 2, 3], "obj": {}}"#, test_context());

        assert!(config.at("arr").is_array());
        assert!(!config.at("obj").is_array());
        assert!(!config.at("missing").is_array());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_is_object() {
        let mut config = Config::default();
        config.load_json(r#"{"obj": {"key": "value"}, "arr": []}"#, test_context());

        assert!(config.at("obj").is_object());
        assert!(!config.at("arr").is_object());
        assert!(!config.at("missing").is_object());
    }
}

// =============================================================================
// Write Operation Tests
// =============================================================================

mod write_operations {
    use super::*;

    #[test]
    fn test_set_simple() {
        let mut config = Config::default();

        config.at("key").set("value").unwrap();
        assert!(matches!(config.at("key").get(), Some(ContextValue::String(s, _)) if s == "value"));
    }

    #[test]
    fn test_set_nested_creates_intermediates() {
        let mut config = Config::default();

        config.at("a.b.c").set(42).unwrap();

        // All intermediate objects should be created
        assert!(config.at("a").is_object());
        assert!(config.at("a.b").is_object());
        assert!(matches!(
            config.at("a.b.c").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_set_various_types() {
        let mut config = Config::default();

        config.at("string").set("hello").unwrap();
        config.at("int").set(42i32).unwrap();
        config.at("int64").set(42i64).unwrap();
        config.at("float").set(3.14f64).unwrap();
        config.at("bool").set(true).unwrap();

        assert!(
            matches!(config.at("string").get(), Some(ContextValue::String(s, _)) if s == "hello")
        );
        assert!(matches!(
            config.at("int").get(),
            Some(ContextValue::Int(42, _))
        ));
        assert!(matches!(
            config.at("int64").get(),
            Some(ContextValue::Int(42, _))
        ));
        assert!(
            matches!(config.at("float").get(), Some(ContextValue::Float(f, _)) if (f - 3.14).abs() < 0.001)
        );
        assert!(matches!(
            config.at("bool").get(),
            Some(ContextValue::Bool(true, _))
        ));
    }

    #[test]
    fn test_set_overwrites() {
        let mut config = Config::default();

        config.at("key").set(1).unwrap();
        config.at("key").set(2).unwrap();

        assert!(matches!(
            config.at("key").get(),
            Some(ContextValue::Int(2, _))
        ));
    }

    #[test]
    fn test_set_error_on_non_object_path() {
        let mut config = Config::default();
        config.at("scalar").set(42).unwrap();

        // Trying to set through a scalar should error
        let result = config.at("scalar.child").set("value");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_if_missing_sets() {
        let mut config = Config::default();

        let was_set = config.at("key").set_if_missing(42).unwrap();
        assert!(was_set);
        assert!(matches!(
            config.at("key").get(),
            Some(ContextValue::Int(42, _))
        ));
    }

    #[test]
    fn test_set_if_missing_does_not_overwrite() {
        let mut config = Config::default();
        config.at("key").set(1).unwrap();

        let was_set = config.at("key").set_if_missing(2).unwrap();
        assert!(!was_set);
        assert!(matches!(
            config.at("key").get(),
            Some(ContextValue::Int(1, _))
        ));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_set_array_index() {
        let mut config = Config::default();
        config.load_json(r#"{"items": [1, 2, 3]}"#, test_context());

        config.at("items.1").set(20).unwrap();

        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(20, _))
        ));
        // Other elements should be unchanged
        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(1, _))
        ));
        assert!(matches!(
            config.at("items.2").get(),
            Some(ContextValue::Int(3, _))
        ));
    }
}

// =============================================================================
// Delete Operation Tests
// =============================================================================

mod delete_operations {
    use super::*;

    #[test]
    fn test_remove_returns_value() {
        let mut config = Config::default();
        config.at("key").set(42).unwrap();

        let removed = config.at("key").remove().value();
        assert!(matches!(removed, Some(Value::Int(42))));
        assert!(!config.at("key").exists());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut config = Config::default();

        let removed = config.at("nonexistent").remove().value();
        assert!(removed.is_none());
    }

    #[test]
    fn test_remove_without_prune() {
        let mut config = Config::default();
        config.at("a.b.c").set(42).unwrap();

        config.at("a.b.c").remove().value();

        // Parent containers should still exist (empty)
        assert!(config.at("a").exists());
        assert!(config.at("a.b").exists());
        assert!(config.at("a.b").is_object());
    }

    #[test]
    fn test_remove_with_prune() {
        let mut config = Config::default();
        config.at("a.b.c").set(42).unwrap();

        let removed = config.at("a.b.c").remove().prune();
        assert!(matches!(removed, Some(Value::Int(42))));

        // Empty ancestors should be pruned
        assert!(!config.at("a.b").exists());
        assert!(!config.at("a").exists());
    }

    #[test]
    fn test_prune_stops_at_non_empty() {
        let mut config = Config::default();
        config.at("a.b.c").set(42).unwrap();
        config.at("a.sibling").set("keep").unwrap();

        config.at("a.b.c").remove().prune();

        // "a" should still exist because it has another child
        assert!(config.at("a").exists());
        assert!(config.at("a.sibling").exists());
        // But "b" should be pruned
        assert!(!config.at("a.b").exists());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_remove_array_element() {
        let mut config = Config::default();
        config.load_json(r#"{"items": [1, 2, 3]}"#, test_context());

        let removed = config.at("items.1").remove().value();
        assert!(matches!(removed, Some(Value::Int(2))));

        // Array should now have 2 elements, shifted
        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(1, _))
        ));
        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(3, _))
        ));
        assert!(!config.at("items.2").exists());
    }
}

// =============================================================================
// Array Operation Tests
// =============================================================================

mod array_operations {
    use super::*;

    #[test]
    fn test_push_creates_array() {
        let mut config = Config::default();

        config.at("items").push("first").unwrap();

        assert!(config.at("items").is_array());
        assert!(
            matches!(config.at("items.0").get(), Some(ContextValue::String(s, _)) if s == "first")
        );
    }

    #[test]
    fn test_push_appends() {
        let mut config = Config::default();
        config.at("items").push(1).unwrap();
        config.at("items").push(2).unwrap();
        config.at("items").push(3).unwrap();

        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(1, _))
        ));
        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(2, _))
        ));
        assert!(matches!(
            config.at("items.2").get(),
            Some(ContextValue::Int(3, _))
        ));
    }

    #[test]
    fn test_push_error_on_non_array() {
        let mut config = Config::default();
        config.at("scalar").set(42).unwrap();

        let result = config.at("scalar").push("value");
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_map_transforms_elements() {
        let mut config = Config::default();
        config.load_json(r#"{"items": [1, 2, 3]}"#, test_context());

        config
            .at("items")
            .map(|v| {
                if let ContextValue::Int(ref mut i, _) = v {
                    *i *= 2;
                }
            })
            .unwrap();

        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(2, _))
        ));
        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(4, _))
        ));
        assert!(matches!(
            config.at("items.2").get(),
            Some(ContextValue::Int(6, _))
        ));
    }

    #[test]
    fn test_map_error_on_non_array() {
        let mut config = Config::default();
        config.at("scalar").set(42).unwrap();

        let result = config.at("scalar").map(|_| {});
        assert!(result.is_err());
    }

    #[test]
    fn test_map_error_on_missing() {
        let mut config = Config::default();

        let result = config.at("missing").map(|_| {});
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_filter_keeps_matching() {
        let mut config = Config::default();
        config.load_json(r#"{"items": [1, 2, 3, 4, 5]}"#, test_context());

        config
            .at("items")
            .filter(|v| matches!(v, ContextValue::Int(i, _) if i % 2 == 0))
            .unwrap();

        // Only even numbers should remain
        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(2, _))
        ));
        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(4, _))
        ));
        assert!(!config.at("items.2").exists());
    }

    #[test]
    fn test_filter_error_on_non_array() {
        let mut config = Config::default();
        config.at("scalar").set(42).unwrap();

        let result = config.at("scalar").filter(|_| true);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_map_where_selective_transform() {
        let mut config = Config::default();
        config.load_json(r#"{"items": [1, 2, 3, 4, 5]}"#, test_context());

        // Double only even numbers
        config
            .at("items")
            .map_where(
                |v| matches!(v, ContextValue::Int(i, _) if i % 2 == 0),
                |v| {
                    if let ContextValue::Int(ref mut i, _) = v {
                        *i *= 2;
                    }
                },
            )
            .unwrap();

        assert!(matches!(
            config.at("items.0").get(),
            Some(ContextValue::Int(1, _))
        )); // odd, unchanged
        assert!(matches!(
            config.at("items.1").get(),
            Some(ContextValue::Int(4, _))
        )); // 2 * 2
        assert!(matches!(
            config.at("items.2").get(),
            Some(ContextValue::Int(3, _))
        )); // odd, unchanged
        assert!(matches!(
            config.at("items.3").get(),
            Some(ContextValue::Int(8, _))
        )); // 4 * 2
        assert!(matches!(
            config.at("items.4").get(),
            Some(ContextValue::Int(5, _))
        )); // odd, unchanged
    }

    #[test]
    fn test_map_where_error_on_non_array() {
        let mut config = Config::default();
        config.at("scalar").set(42).unwrap();

        let result = config.at("scalar").map_where(|_| true, |_| {});
        assert!(result.is_err());
    }
}

// =============================================================================
// Path Parsing Tests
// =============================================================================

mod path_parsing {
    use feuilletage::IntoPath;

    #[test]
    fn test_simple_path() {
        let path: Vec<String> = "a.b.c".into_path();
        assert_eq!(path, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_single_segment() {
        let path: Vec<String> = "single".into_path();
        assert_eq!(path, vec!["single"]);
    }

    #[test]
    fn test_empty_path() {
        let path: Vec<String> = "".into_path();
        assert!(path.is_empty());
    }

    #[test]
    fn test_escaped_dots() {
        let path: Vec<String> = "a.b\\.c.d".into_path();
        assert_eq!(path, vec!["a", "b.c", "d"]);
    }

    #[test]
    fn test_multiple_escaped_dots() {
        let path: Vec<String> = "a\\.b\\.c".into_path();
        assert_eq!(path, vec!["a.b.c"]);
    }

    #[test]
    fn test_array_path() {
        let path: Vec<String> = ["a", "b.c", "d"].into_path();
        assert_eq!(path, vec!["a", "b.c", "d"]);
    }

    #[test]
    fn test_slice_path() {
        let segments = ["a", "b.c", "d"];
        let path: Vec<String> = segments.as_slice().into_path();
        assert_eq!(path, vec!["a", "b.c", "d"]);
    }

    #[test]
    fn test_vec_path() {
        let segments = vec!["a".to_string(), "b".to_string()];
        let path: Vec<String> = segments.into_path();
        assert_eq!(path, vec!["a", "b"]);
    }

    #[test]
    fn test_numeric_segments() {
        let path: Vec<String> = "items.0.name".into_path();
        assert_eq!(path, vec!["items", "0", "name"]);
    }
}

// =============================================================================
// Integration Tests
// =============================================================================

mod integration {
    use super::*;

    #[test]
    #[cfg(feature = "json")]
    fn test_full_workflow() {
        let mut config = Config::default();

        // Load initial config
        config.load_json(
            r#"{"server": {"host": "localhost", "port": 8080}}"#,
            test_context(),
        );

        // Read and verify
        assert!(matches!(
            config.at("server.host").get(),
            Some(ContextValue::String(s, _)) if s == "localhost"
        ));

        // Update
        config.at("server.port").set(3000).unwrap();
        assert!(matches!(
            config.at("server.port").get(),
            Some(ContextValue::Int(3000, _))
        ));

        // Add new nested value
        config.at("server.tls.enabled").set(true).unwrap();
        assert!(config.at("server.tls.enabled").exists());

        // Remove with prune
        config.at("server.tls.enabled").remove().prune();
        assert!(!config.at("server.tls").exists());

        // Server should still exist
        assert!(config.at("server").exists());
    }

    #[test]
    fn test_complex_path_navigation() {
        let mut config = Config::default();

        // Set up complex structure using various navigation methods
        config
            .at("plugins")
            .at_raw("auth.v2")
            .at("settings.timeout")
            .set(30)
            .unwrap();
        config
            .at(["plugins", "auth.v2", "enabled"])
            .set(true)
            .unwrap();

        // Verify using different access methods
        assert!(matches!(
            config
                .at("plugins")
                .at_raw("auth.v2")
                .at("settings")
                .at("timeout")
                .get(),
            Some(ContextValue::Int(30, _))
        ));
        assert!(matches!(
            config.at(["plugins", "auth.v2", "enabled"]).get(),
            Some(ContextValue::Bool(true, _))
        ));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_array_manipulation_workflow() {
        let mut config = Config::default();
        config.load_json(r#"{"tags": ["alpha", "beta"]}"#, test_context());

        // Push new tag
        config.at("tags").push("gamma").unwrap();

        // Filter to keep only beta and gamma
        config
            .at("tags")
            .filter(|v| !matches!(v, ContextValue::String(s, _) if s == "alpha"))
            .unwrap();

        // Transform remaining
        config
            .at("tags")
            .map(|v| {
                if let ContextValue::String(ref mut s, _) = v {
                    *s = s.to_uppercase();
                }
            })
            .unwrap();

        assert!(
            matches!(config.at("tags.0").get(), Some(ContextValue::String(s, _)) if s == "BETA")
        );
        assert!(
            matches!(config.at("tags.1").get(), Some(ContextValue::String(s, _)) if s == "GAMMA")
        );
    }
}

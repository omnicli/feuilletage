//! Unit tests for config module (Config struct operations).
//!
//! Extracted from compote/src/config.rs

use compote::{Config, Context, ContextValue, Format, Level, MutabilityConstraint, Source};

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

#[test]
fn test_load_and_merge_json() {
    let mut config = Config::default();

    let json1 = r#"{"a": 1, "b": 2}"#;
    config.load_json(json1, test_context());

    let json2 = r#"{"b": 3, "c": 4}"#;
    config.load_json(json2, test_context());

    if let ContextValue::Object(map, _) = config.root() {
        assert_eq!(map.len(), 3);
        assert!(matches!(map.get("a").unwrap(), ContextValue::Int(1, _)));
        assert!(matches!(map.get("b").unwrap(), ContextValue::Int(3, _)));
        assert!(matches!(map.get("c").unwrap(), ContextValue::Int(4, _)));
    } else {
        panic!("Expected object");
    }

    assert!(!config.has_errors());
}

#[test]
fn test_merge_with_modifier() {
    let mut config = Config::default();

    let json1 = r#"{"items": [1, 2]}"#;
    config.load_json(json1, test_context());

    let json2 = r#"{"items__toappend": 3}"#;
    config.load_json(json2, test_context());

    if let ContextValue::Object(map, _) = config.root() {
        if let ContextValue::Array(arr, _) = map.get("items").unwrap() {
            assert_eq!(arr.len(), 3);
            assert!(matches!(&arr[0], ContextValue::Int(1, _)));
            assert!(matches!(&arr[1], ContextValue::Int(2, _)));
            assert!(matches!(&arr[2], ContextValue::Int(3, _)));
        } else {
            panic!("Expected array");
        }
    } else {
        panic!("Expected object");
    }

    assert!(!config.has_errors());
}

#[test]
fn test_immutability() {
    let mut config = Config::default();

    // Load initial config with immutable value
    let json1 = r#"{"secret": "password"}"#;
    config.load_json(
        json1,
        test_context().with_mutability_constraint(MutabilityConstraint::Immutable),
    );

    // Try to override it
    let json2 = r#"{"secret": "newpassword"}"#;
    config.load_json(json2, test_context());

    // Value should not have changed
    if let ContextValue::Object(map, _) = config.root() {
        assert!(matches!(
            map.get("secret").unwrap(),
            ContextValue::String(s, _) if s == "password"
        ));
    }

    // Should have recorded an error
    assert!(config.has_errors());
}

#[test]
fn test_nested_merge() {
    let mut config = Config::default();

    let json1 = r#"{"a": {"b": {"c": 1, "d": 2}}}"#;
    config.load_json(json1, test_context());

    let json2 = r#"{"a": {"b": {"c": 3, "e": 4}}}"#;
    config.load_json(json2, test_context());

    if let ContextValue::Object(map, _) = config.root() {
        if let ContextValue::Object(a, _) = map.get("a").unwrap() {
            if let ContextValue::Object(b, _) = a.get("b").unwrap() {
                assert_eq!(b.len(), 3);
                assert!(matches!(b.get("c").unwrap(), ContextValue::Int(3, _)));
                assert!(matches!(b.get("d").unwrap(), ContextValue::Int(2, _)));
                assert!(matches!(b.get("e").unwrap(), ContextValue::Int(4, _)));
            } else {
                panic!("Expected object at a.b");
            }
        } else {
            panic!("Expected object at a");
        }
    } else {
        panic!("Expected object at root");
    }

    assert!(!config.has_errors());
}

#[test]
fn test_get_simple_path() {
    let mut config = Config::default();
    config.load_json(r#"{"a": {"b": {"c": 42}}}"#, test_context());

    // Test nested path access
    let value = config.get("a.b.c");
    assert!(value.is_some());
    assert!(matches!(value.unwrap(), ContextValue::Int(42, _)));

    // Test intermediate path
    let value = config.get("a.b");
    assert!(value.is_some());
    if let ContextValue::Object(map, _) = value.unwrap() {
        assert!(map.contains_key("c"));
    } else {
        panic!("Expected object at a.b");
    }

    // Test single level path
    let value = config.get("a");
    assert!(value.is_some());
    if let ContextValue::Object(map, _) = value.unwrap() {
        assert!(map.contains_key("b"));
    } else {
        panic!("Expected object at a");
    }
}

#[test]
fn test_get_empty_path_returns_root() {
    let mut config = Config::default();
    config.load_json(r#"{"key": "value"}"#, test_context());

    let value = config.get("");
    assert!(value.is_some());
    if let ContextValue::Object(map, _) = value.unwrap() {
        assert!(map.contains_key("key"));
    } else {
        panic!("Expected object at root");
    }
}

#[test]
fn test_get_nonexistent_path() {
    let mut config = Config::default();
    config.load_json(r#"{"a": {"b": 1}}"#, test_context());

    // Nonexistent key
    assert!(config.get("a.c").is_none());
    assert!(config.get("x").is_none());
    assert!(config.get("a.b.c").is_none()); // trying to traverse through int
}

#[test]
#[cfg(feature = "toml")]
fn test_default_format_is_validated_and_propagated() {
    let mut config = Config::default();
    config.set_default_format(Format::Toml).unwrap();
    config.at("first").set(1).unwrap();
    config.at("second").set(2).unwrap();

    assert_eq!(config.default_format(), Format::Toml);
    assert_eq!(
        config
            .select(|key, _| key == "first")
            .unwrap()
            .default_format(),
        Format::Toml,
    );
    assert!(config
        .split_by_key()
        .unwrap()
        .into_iter()
        .all(|(_, part)| part.default_format() == Format::Toml));

    assert!(config.set_default_format(Format::Unknown).is_err());
    assert_eq!(config.default_format(), Format::Toml);
}

#[test]
#[cfg(not(feature = "toml"))]
fn test_default_format_rejects_disabled_format() {
    let mut config = Config::default();
    let original = config.default_format();

    assert!(config.set_default_format(Format::Toml).is_err());
    assert_eq!(config.default_format(), original);
}

#[test]
fn test_get_array_index() {
    let mut config = Config::default();
    config.load_json(
        r#"{"items": [{"name": "first"}, {"name": "second"}]}"#,
        test_context(),
    );

    // Access array element by index
    let value = config.get("items.0");
    assert!(value.is_some());
    if let ContextValue::Object(map, _) = value.unwrap() {
        assert!(matches!(
            map.get("name").unwrap(),
            ContextValue::String(s, _) if s == "first"
        ));
    } else {
        panic!("Expected object at items.0");
    }

    // Access nested value in array element
    let value = config.get("items.1.name");
    assert!(value.is_some());
    assert!(matches!(value.unwrap(), ContextValue::String(s, _) if s == "second"));

    // Out of bounds index
    assert!(config.get("items.5").is_none());

    // Invalid index (not a number)
    assert!(config.get("items.foo").is_none());
}

#[test]
fn test_get_mut_modify_value() {
    let mut config = Config::default();
    config.load_json(r#"{"a": {"b": {"c": 42}}}"#, test_context());

    // Get mutable reference and verify we can access it
    let value = config.get_mut("a.b.c");
    assert!(value.is_some());
    let value = value.unwrap();
    assert!(matches!(value, ContextValue::Int(42, _)));

    // Modify the value by replacing the entire ContextValue (preserving context)
    *value = ContextValue::int(100, value.context().clone());

    // Verify modification via immutable get
    let value = config.get("a.b.c");
    assert!(value.is_some());
    assert!(matches!(value.unwrap(), ContextValue::Int(100, _)));
}

#[test]
fn test_get_mut_empty_path_returns_root() {
    let mut config = Config::default();
    config.load_json(r#"{"key": "value"}"#, test_context());

    let value = config.get_mut("");
    assert!(value.is_some());
    if let ContextValue::Object(map, _) = value.unwrap() {
        assert!(map.contains_key("key"));
    } else {
        panic!("Expected object at root");
    }
}

#[test]
fn test_get_mut_array_index() {
    let mut config = Config::default();
    config.load_json(r#"{"items": [1, 2, 3]}"#, test_context());

    // Modify array element
    let value = config.get_mut("items.1");
    assert!(value.is_some());
    let value = value.unwrap();
    *value = ContextValue::int(20, value.context().clone());

    // Verify modification
    let value = config.get("items.1");
    assert!(value.is_some());
    assert!(matches!(value.unwrap(), ContextValue::Int(20, _)));

    // Other elements unchanged
    assert!(matches!(
        config.get("items.0").unwrap(),
        ContextValue::Int(1, _)
    ));
    assert!(matches!(
        config.get("items.2").unwrap(),
        ContextValue::Int(3, _)
    ));
}

#[test]
fn test_get_mut_nonexistent_path() {
    let mut config = Config::default();
    config.load_json(r#"{"a": {"b": 1}}"#, test_context());

    assert!(config.get_mut("a.c").is_none());
    assert!(config.get_mut("x").is_none());
    assert!(config.get_mut("a.b.c").is_none());
}

// ============================================================================
// Tests for edit_first_writeable
// ============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_writeable_existing_file() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.yaml");

    // Create an existing file with content
    std::fs::write(&file_path, "key: value\n").unwrap();

    // Edit the existing file
    let result = compote::edit_first_writeable(&[&file_path], |config| {
        config.at("new_key").set("new_value").ok();
        true
    })
    .unwrap();

    assert_eq!(result, Some(file_path.clone()));

    // Verify the file was updated
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("new_key"));
    assert!(content.contains("new_value"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_writeable_creates_new_file() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("new_config.yaml");

    // File doesn't exist yet
    assert!(!file_path.exists());

    // Edit should create the file
    let result = compote::edit_first_writeable(&[&file_path], |config| {
        config.at("setting").set("enabled").ok();
        true
    })
    .unwrap();

    assert_eq!(result, Some(file_path.clone()));

    // Verify the file was created
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("setting"));
    assert!(content.contains("enabled"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_writeable_no_save() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let file_path = dir.path().join("config.yaml");

    // Edit with closure returning false
    let result = compote::edit_first_writeable(&[&file_path], |_config| {
        false // Don't save
    })
    .unwrap();

    // Should return None since closure returned false
    assert_eq!(result, None);

    // File should not be created
    assert!(!file_path.exists());
}

#[test]
#[cfg(all(feature = "yaml", unix))]
fn test_edit_first_writeable_skips_readonly_file() {
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let readonly_file = dir.path().join("readonly.yaml");
    let writeable_file = dir.path().join("writeable.yaml");

    // Create a readonly file
    std::fs::write(&readonly_file, "readonly: true\n").unwrap();
    let mut perms = std::fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_mode(0o444); // read-only
    std::fs::set_permissions(&readonly_file, perms).unwrap();

    // Create a writeable file
    std::fs::write(&writeable_file, "writeable: true\n").unwrap();

    // Should skip readonly file and edit writeable one
    let result = compote::edit_first_writeable(&[&readonly_file, &writeable_file], |config| {
        config.at("modified").set(true).ok();
        true
    })
    .unwrap();

    assert_eq!(result, Some(writeable_file.clone()));

    // Verify readonly file was not modified
    let readonly_content = std::fs::read_to_string(&readonly_file).unwrap();
    assert!(!readonly_content.contains("modified"));

    // Verify writeable file was modified
    let writeable_content = std::fs::read_to_string(&writeable_file).unwrap();
    assert!(writeable_content.contains("modified"));

    // Clean up: make readonly file deletable
    let mut perms = std::fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&readonly_file, perms).unwrap();
}

#[test]
#[cfg(all(feature = "yaml", unix))]
fn test_edit_first_writeable_no_writeable_candidates() {
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let readonly_file = dir.path().join("readonly.yaml");

    // Create a readonly file
    std::fs::write(&readonly_file, "readonly: true\n").unwrap();
    let mut perms = std::fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_mode(0o444); // read-only
    std::fs::set_permissions(&readonly_file, perms).unwrap();

    // Only candidate is readonly, no writeable parent either
    // Create path in a readonly directory
    let readonly_dir = dir.path().join("readonly_dir");
    std::fs::create_dir(&readonly_dir).unwrap();
    let mut dir_perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
    dir_perms.set_mode(0o555); // read + execute only
    std::fs::set_permissions(&readonly_dir, dir_perms).unwrap();

    let new_file_in_readonly = readonly_dir.join("new.yaml");

    // All candidates are not writeable
    let result =
        compote::edit_first_writeable(&[&readonly_file, &new_file_in_readonly], |_config| true)
            .unwrap();

    // Should return None because no writeable file was found
    assert_eq!(result, None);

    // Clean up
    let mut perms = std::fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&readonly_file, perms).unwrap();

    let mut dir_perms = std::fs::metadata(&readonly_dir).unwrap().permissions();
    dir_perms.set_mode(0o755);
    std::fs::set_permissions(&readonly_dir, dir_perms).unwrap();
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_writeable_creates_parent_dirs() {
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let nested_path = dir.path().join("a").join("b").join("c").join("config.yaml");

    // Parent directories don't exist
    assert!(!nested_path.parent().unwrap().exists());

    // Edit should create parent directories and file
    let result = compote::edit_first_writeable(&[&nested_path], |config| {
        config.at("deep").set("value").ok();
        true
    })
    .unwrap();

    assert_eq!(result, Some(nested_path.clone()));
    assert!(nested_path.exists());
}

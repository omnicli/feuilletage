//! Tests for file operations (write_file, edit_file, edit_first_existing)

use compote::{Config, Context, ContextValue, Format, Level, Source};
use std::fs;
#[cfg(feature = "json")]
use std::sync::mpsc;
#[cfg(feature = "json")]
use std::thread;
#[cfg(feature = "json")]
use std::time::Duration;
use tempfile::TempDir;

/// Create a temporary directory for testing
fn temp_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Helper to create a test config with some values
fn test_config() -> Config {
    let mut config = Config::default();
    #[cfg(feature = "json")]
    config.load_json(
        r#"{"name": "test", "count": 42}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    config
}

// ============================================================================
// write_file / write_file_as tests
// ============================================================================

#[test]
#[cfg(feature = "json")]
fn test_write_file_json() {
    let dir = temp_dir();
    let path = dir.path().join("config.json");

    let config = test_config();
    config.write_file(&path).unwrap();

    // Verify file exists and content is valid JSON
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"name\""));
    assert!(content.contains("\"test\""));
    assert!(content.contains("\"count\""));
    assert!(content.contains("42"));

    // Verify it's valid JSON by parsing
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[test]
#[cfg(feature = "yaml")]
fn test_write_file_yaml() {
    let dir = temp_dir();
    let path = dir.path().join("config.yaml");

    let mut config = Config::default();
    config.load_yaml(
        "name: test\ncount: 42",
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    // Verify file exists and content is valid YAML
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("name"));
    assert!(content.contains("test"));
    assert!(content.contains("count"));
    assert!(content.contains("42"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_write_file_yml_extension() {
    let dir = temp_dir();
    let path = dir.path().join("config.yml");

    let mut config = Config::default();
    config.load_yaml(
        "name: test",
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("name"));
}

#[test]
#[cfg(feature = "toml")]
fn test_write_file_toml() {
    let dir = temp_dir();
    let path = dir.path().join("config.toml");

    let mut config = Config::default();
    config.load_toml(
        "name = \"test\"\ncount = 42",
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    // Verify file exists and content is valid TOML
    assert!(path.exists());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("name"));
    assert!(content.contains("test"));
    assert!(content.contains("count"));
    assert!(content.contains("42"));
}

#[test]
#[cfg(feature = "json")]
fn test_write_file_as_explicit_format() {
    let dir = temp_dir();
    let path = dir.path().join("config.txt"); // Unknown extension

    let config = test_config();
    config.write_file_as(&path, Format::Json).unwrap();

    // Verify it wrote JSON despite .txt extension
    let content = fs::read_to_string(&path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[test]
#[cfg(feature = "json")]
fn test_write_file_creates_parent_directories() {
    let dir = temp_dir();
    let path = dir.path().join("nested").join("deep").join("config.json");

    let config = test_config();
    config.write_file(&path).unwrap();

    assert!(path.exists());
    assert!(path.parent().unwrap().exists());
}

#[test]
#[cfg(all(feature = "json", unix))]
fn test_write_file_preserves_executable_bits() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir();
    let path = dir.path().join("config.json");
    fs::write(&path, "original").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

    test_config().write_file(&path).unwrap();

    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn test_write_file_unknown_extension_uses_default() {
    let dir = temp_dir();
    let path = dir.path().join("config.conf");

    let config = Config::default();
    // Should use default format based on enabled features
    config.write_file(&path).unwrap();

    assert!(path.exists());
}

#[test]
#[cfg(all(feature = "json", feature = "toml"))]
fn test_unknown_extension_uses_config_default_but_recognized_extension_wins() {
    let dir = temp_dir();
    let unknown_path = dir.path().join("config.conf");
    let json_path = dir.path().join("config.json");

    let mut config = Config::default().with_default_format(Format::Toml).unwrap();
    config.at("name").set("compote").unwrap();

    config.write_file(&unknown_path).unwrap();
    config.write_file(&json_path).unwrap();

    assert!(fs::read_to_string(&unknown_path)
        .unwrap()
        .contains("name = \"compote\""));
    serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&json_path).unwrap()).unwrap();
}

#[test]
#[cfg(all(feature = "json", feature = "toml"))]
fn test_unknown_extension_preserves_loaded_format_ahead_of_config_default() {
    let dir = temp_dir();
    let path = dir.path().join("config.conf");

    let mut config = Config::default().with_default_format(Format::Toml).unwrap();
    config.load_json(
        r#"{"name": "compote"}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    config.write_file(&path).unwrap();

    serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&path).unwrap()).unwrap();
}

#[test]
#[cfg(all(feature = "json", feature = "toml", feature = "yaml"))]
fn test_config_default_format_is_output_only_for_extensionless_file() {
    let dir = temp_dir();
    let path = dir.path().join("config");
    fs::write(&path, r#"{"input":"json"}"#).unwrap();

    let mut config = Config::default().with_default_format(Format::Toml).unwrap();

    assert!(!config.load_file(&path, Level::User));
    assert_eq!(config.loaded_format(), Format::Unknown);
    assert!(config.get("input").is_none());

    config.at("output").set("toml").unwrap();
    assert!(config
        .serialize_raw()
        .unwrap()
        .contains("output = \"toml\""));
}

#[test]
#[cfg(all(feature = "json", feature = "toml", feature = "yaml"))]
fn test_config_load_file_auto_preserves_detected_format() {
    let dir = temp_dir();
    let path = dir.path().join("config");
    fs::write(&path, "name = \"compote\"\n").unwrap();

    let mut config = Config::default().with_default_format(Format::Yaml).unwrap();

    assert!(config.load_file_auto(&path, Level::User));
    assert_eq!(config.loaded_format(), Format::Toml);
    assert_eq!(
        config.get("name").and_then(ContextValue::as_str),
        Some("compote")
    );
    assert!(!config.has_errors());
    assert!(config
        .serialize_raw()
        .unwrap()
        .contains("name = \"compote\""));
}

// ============================================================================
// edit_file tests
// ============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_edit_file_creates_new_file() {
    let dir = temp_dir();
    let path = dir.path().join("new_config.yaml");

    assert!(!path.exists());

    let result = compote::edit_file(&path, |config| {
        config.at("server.port").set(8080).ok();
        true
    });

    assert!(result.is_ok());
    assert!(result.unwrap()); // Should return true (saved)
    assert!(path.exists());

    // Verify content
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("server"));
    assert!(content.contains("port"));
    assert!(content.contains("8080"));
}

#[test]
#[cfg(feature = "json")]
fn test_edit_file_modifies_existing() {
    let dir = temp_dir();
    let path = dir.path().join("config.json");

    // Create initial file
    fs::write(&path, r#"{"name": "original", "count": 1}"#).unwrap();

    let result = compote::edit_file(&path, |config| {
        config.at("name").set("modified").ok();
        config.at("count").set(99).ok();
        true
    });

    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify modifications
    let content = fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["name"], "modified");
    assert_eq!(json["count"], 99);
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_file_conditional_save_false() {
    let dir = temp_dir();
    let path = dir.path().join("config.yaml");

    // Create initial file
    fs::write(&path, "name: original").unwrap();

    let result = compote::edit_file(&path, |config| {
        // Read but don't save
        let _name = config.get("name");
        false // Don't save
    });

    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should return false (not saved)

    // Verify file unchanged
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("original"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_file_empty_file_treated_as_empty_config() {
    let dir = temp_dir();
    let path = dir.path().join("empty.yaml");

    // Create empty file
    fs::write(&path, "").unwrap();

    let result = compote::edit_file(&path, |config| {
        // Should be able to add to empty config
        config.at("new_key").set("new_value").ok();
        true
    });

    assert!(result.is_ok());
    assert!(result.unwrap());

    // Verify content
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("new_key"));
    assert!(content.contains("new_value"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_file_whitespace_only_treated_as_empty() {
    let dir = temp_dir();
    let path = dir.path().join("whitespace.yaml");

    // Create file with only whitespace
    fs::write(&path, "   \n  \n   ").unwrap();

    let result = compote::edit_file(&path, |config| {
        config.at("key").set("value").ok();
        true
    });

    assert!(result.is_ok());
    assert!(result.unwrap());

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("key"));
}

#[test]
#[cfg(feature = "json")]
fn test_edit_file_creates_parent_directories() {
    let dir = temp_dir();
    let path = dir.path().join("deep").join("nested").join("config.json");

    let result = compote::edit_file(&path, |config| {
        config.at("key").set("value").ok();
        true
    });

    assert!(result.is_ok());
    assert!(path.exists());
    assert!(path.parent().unwrap().exists());
}

#[test]
#[cfg(feature = "json")]
fn test_edit_file_preserves_existing_values() {
    let dir = temp_dir();
    let path = dir.path().join("config.json");

    // Create initial file with multiple values
    fs::write(&path, r#"{"a": 1, "b": 2, "c": 3}"#).unwrap();

    compote::edit_file(&path, |config| {
        // Only modify one value
        config.at("b").set(20).ok();
        true
    })
    .unwrap();

    // Verify other values preserved
    let content = fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["a"], 1);
    assert_eq!(json["b"], 20);
    assert_eq!(json["c"], 3);
}

#[test]
#[cfg(feature = "json")]
fn test_concurrent_edits_are_serialized() {
    let dir = temp_dir();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{"count": 0}"#).unwrap();

    let first_path = path.clone();
    let (entered_tx, entered_rx) = mpsc::channel();
    let first = thread::spawn(move || {
        compote::edit_file(&first_path, |config| {
            let count = match config.get("count") {
                Some(ContextValue::Int(count, _)) => *count,
                _ => panic!("count should be an integer"),
            };
            entered_tx.send(()).unwrap();
            thread::sleep(Duration::from_millis(200));
            config.at("count").set(count + 1).unwrap();
            true
        })
        .unwrap();
    });

    entered_rx.recv().unwrap();
    let second_path = path.clone();
    let second = thread::spawn(move || {
        compote::edit_file(&second_path, |config| {
            let count = match config.get("count") {
                Some(ContextValue::Int(count, _)) => *count,
                _ => panic!("count should be an integer"),
            };
            config.at("count").set(count + 1).unwrap();
            true
        })
        .unwrap();
    });

    first.join().unwrap();
    second.join().unwrap();

    let content = fs::read_to_string(path).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&content).unwrap()["count"],
        2
    );
}

#[test]
#[cfg(feature = "yaml")]
fn test_explicit_file_apis_accept_unknown_extension() {
    let dir = temp_dir();
    let path = dir.path().join("config.conf");
    fs::write(&path, "name: original\n").unwrap();

    let mut config = Config::default();
    assert!(config.load_file_with_format(&path, Format::Yaml, Level::User));
    assert_eq!(config.loaded_format(), Format::Yaml);
    assert_eq!(
        config.get("name").and_then(ContextValue::as_str),
        Some("original")
    );

    compote::edit_file_with_format(&path, Format::Yaml, |config| {
        config.at("name").set("modified").unwrap();
        true
    })
    .unwrap();

    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("name: modified"));
}

#[test]
fn test_edit_file_rejects_unknown_extension_without_explicit_format() {
    let dir = temp_dir();
    let path = dir.path().join("config.conf");

    let error = compote::edit_file(&path, |_| true).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!path.exists());
}

#[test]
fn test_edit_file_rejects_explicit_unknown_format() {
    let dir = temp_dir();
    let path = dir.path().join("config.conf");

    let error = compote::edit_file_with_format(&path, Format::Unknown, |_| true).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!path.exists());
}

// ============================================================================
// edit_first_existing tests
// ============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_existing_finds_first_file() {
    let dir = temp_dir();
    let path1 = dir.path().join("first.yaml");
    let path2 = dir.path().join("second.yaml");
    let path3 = dir.path().join("third.yaml");

    // Create only the second file
    fs::write(&path2, "existing: true").unwrap();

    let paths = [&path1, &path2, &path3];
    let result = compote::edit_first_existing(&paths, |config| {
        config.at("modified").set(true).ok();
        true
    });

    assert!(result.is_ok());
    let edited_path = result.unwrap();
    assert!(edited_path.is_some());
    assert_eq!(edited_path.unwrap(), path2);

    // Verify first file was NOT created
    assert!(!path1.exists());
    // Verify second file was modified
    let content = fs::read_to_string(&path2).unwrap();
    assert!(content.contains("modified"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_existing_creates_first_when_none_exist() {
    let dir = temp_dir();
    let path1 = dir.path().join("first.yaml");
    let path2 = dir.path().join("second.yaml");

    let paths = [&path1, &path2];
    let result = compote::edit_first_existing(&paths, |config| {
        config.at("created").set(true).ok();
        true
    });

    assert!(result.is_ok());
    let edited_path = result.unwrap();
    assert!(edited_path.is_some());
    assert_eq!(edited_path.unwrap(), path1);

    // Verify first file was created
    assert!(path1.exists());
    assert!(!path2.exists());

    let content = fs::read_to_string(&path1).unwrap();
    assert!(content.contains("created"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_edit_first_existing_returns_none_when_not_saved() {
    let dir = temp_dir();
    let path1 = dir.path().join("first.yaml");
    let path2 = dir.path().join("second.yaml");

    // Create the second file
    fs::write(&path2, "existing: true").unwrap();

    let paths = [&path1, &path2];
    let result = compote::edit_first_existing(&paths, |_config| {
        false // Don't save
    });

    assert!(result.is_ok());
    let edited_path = result.unwrap();
    assert!(edited_path.is_none());
}

#[test]
fn test_edit_first_existing_empty_paths_returns_error() {
    let paths: [&str; 0] = [];
    let result = compote::edit_first_existing(&paths, |_| true);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
#[cfg(feature = "json")]
fn test_edit_first_existing_with_different_formats() {
    let dir = temp_dir();
    let path_yaml = dir.path().join("config.yaml");
    let path_json = dir.path().join("config.json");
    let path_toml = dir.path().join("config.toml");

    // Create only the JSON file
    fs::write(&path_json, r#"{"format": "json"}"#).unwrap();

    let paths = [&path_yaml, &path_json, &path_toml];
    let result = compote::edit_first_existing(&paths, |config| {
        config.at("edited").set(true).ok();
        true
    });

    assert!(result.is_ok());
    let edited_path = result.unwrap();
    assert_eq!(edited_path.unwrap(), path_json);

    // Verify JSON format preserved
    let content = fs::read_to_string(&path_json).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["format"], "json");
    assert_eq!(json["edited"], true);
}

// ============================================================================
// Round-trip tests (write then read)
// ============================================================================

#[test]
#[cfg(feature = "json")]
fn test_round_trip_json() {
    let dir = temp_dir();
    let path = dir.path().join("roundtrip.json");

    // Create and write config
    let mut config = Config::default();
    config.load_json(
        r#"{"nested": {"value": 42}, "array": [1, 2, 3]}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    // Read it back
    let mut config2 = Config::default();
    config2.load_file(&path, Level::User);

    // Verify values
    assert!(matches!(
        config2.get("nested.value").unwrap(),
        ContextValue::Int(42, _)
    ));
    if let ContextValue::Array(arr, _) = config2.get("array").unwrap() {
        assert_eq!(arr.len(), 3);
    } else {
        panic!("Expected array");
    }
}

#[test]
#[cfg(feature = "yaml")]
fn test_round_trip_yaml() {
    let dir = temp_dir();
    let path = dir.path().join("roundtrip.yaml");

    // Create and write config with simpler structure
    let mut config = Config::default();
    config.load_yaml(
        "name: test\ncount: 42",
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    // Read it back
    let mut config2 = Config::default();
    config2.load_file(&path, Level::User);

    // Verify values
    assert!(matches!(
        config2.get("name").unwrap(),
        ContextValue::String(s, _) if s == "test"
    ));
    assert!(matches!(
        config2.get("count").unwrap(),
        ContextValue::Int(42, _)
    ));
}

#[test]
#[cfg(feature = "toml")]
fn test_round_trip_toml() {
    let dir = temp_dir();
    let path = dir.path().join("roundtrip.toml");

    // Create and write config with simpler structure
    let mut config = Config::default();
    config.load_toml(
        "name = \"test\"\ncount = 42",
        Context::new(Source::Programmatic, Level::User),
    );
    config.write_file(&path).unwrap();

    // Read it back
    let mut config2 = Config::default();
    config2.load_file(&path, Level::User);

    // Verify values
    assert!(matches!(
        config2.get("name").unwrap(),
        ContextValue::String(s, _) if s == "test"
    ));
    assert!(matches!(
        config2.get("count").unwrap(),
        ContextValue::Int(42, _)
    ));
}

//! Tests for relative_path transform attribute.
//!
//! The relative_path transform resolves relative paths against the config file's directory.
//! - Relative paths are expanded to absolute paths based on the config file location
//! - Absolute paths are left unchanged
//! - Non-file sources leave relative paths unchanged

use feuilletage::Level;
use feuilletage_macros::Config as DeriveConfig;
use std::path::PathBuf;

/// Test relative_path expansion when loading from a file
#[test]
fn test_relative_path_expansion_from_file() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[feuilletage(relative_path, default = "/default/path")]
        data_dir: String,
    }

    // Create a temp file to simulate loading from a real file
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_config.yaml");
    let config_content = r#"data_dir: "relative/data""#;

    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: PathConfig = loader.deserialize().expect("Should succeed");

    // The relative path should be expanded relative to the config file's directory
    let _expected_base = temp_dir.join("relative/data");

    // On different platforms, paths might be normalized differently
    // We just check that it's no longer a relative path and contains the expected components
    assert!(
        result.data_dir.contains("relative") && result.data_dir.contains("data"),
        "Path should contain 'relative/data', got: {}",
        result.data_dir
    );

    // Check it's an absolute path (starts with / on Unix or drive letter on Windows)
    let path = PathBuf::from(&result.data_dir);
    assert!(
        path.is_absolute(),
        "Path should be absolute after transform, got: {}",
        result.data_dir
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test that absolute paths are not modified
#[test]
fn test_relative_path_leaves_absolute_unchanged() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[feuilletage(relative_path, default = "/default/path")]
        config_path: String,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_absolute.yaml");
    let config_content = r#"config_path: "/absolute/path/to/file""#;

    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: PathConfig = loader.deserialize().expect("Should succeed");

    // Absolute path should remain unchanged
    assert_eq!(
        result.config_path, "/absolute/path/to/file",
        "Absolute path should not be modified"
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test relative_path with non-file source (programmatic)
#[test]
fn test_relative_path_with_programmatic_source() {
    use feuilletage::{Config, Context, Source};

    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[feuilletage(relative_path, default = "/default/path")]
        data_dir: String,
    }

    // Load from programmatic source (not a file)
    let config_str = r#"{"data_dir": "relative/path"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: PathConfig = config.deserialize().expect("Should succeed");

    // With programmatic source, relative paths cannot be resolved,
    // so they should remain unchanged
    assert_eq!(
        result.data_dir, "relative/path",
        "Relative path should remain unchanged with non-file source"
    );
}

/// Test relative_path with various path formats
#[test]
fn test_relative_path_various_formats() {
    #[derive(DeriveConfig, Debug)]
    struct MultiPathConfig {
        #[feuilletage(relative_path, default = "/default")]
        simple: String,

        #[feuilletage(relative_path, default = "/default")]
        with_dots: String,

        #[feuilletage(relative_path, default = "/default")]
        nested: String,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_paths.yaml");
    let config_content = r#"
simple: "data"
with_dots: "./config/app.yaml"
nested: "a/b/c/d"
"#;

    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: MultiPathConfig = loader.deserialize().expect("Should succeed");

    // All should be absolute after transform
    assert!(
        PathBuf::from(&result.simple).is_absolute(),
        "simple should be absolute, got: {}",
        result.simple
    );
    assert!(
        PathBuf::from(&result.with_dots).is_absolute(),
        "with_dots should be absolute, got: {}",
        result.with_dots
    );
    assert!(
        PathBuf::from(&result.nested).is_absolute(),
        "nested should be absolute, got: {}",
        result.nested
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test relative_path on required field (no default)
#[test]
fn test_relative_path_required_field() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredPathConfig {
        #[feuilletage(relative_path)]
        required_path: String,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_required_path.yaml");
    let config_content = r#"required_path: "logs/app.log""#;

    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: RequiredPathConfig = loader.deserialize().expect("Should succeed");

    // Path should be expanded
    assert!(
        PathBuf::from(&result.required_path).is_absolute(),
        "required_path should be absolute, got: {}",
        result.required_path
    );
    assert!(
        result.required_path.contains("logs") && result.required_path.contains("app.log"),
        "Path should contain original components"
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test that missing required field still fails even with relative_path
#[test]
fn test_relative_path_missing_required_fails() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredPathConfig {
        #[feuilletage(relative_path)]
        required_path: String,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_missing_path.yaml");
    let config_content = r#"other_field: "value""#;

    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: Result<RequiredPathConfig, _> = loader.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required field is missing"
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

/// Test relative_path with Option<String>
#[test]
fn test_relative_path_with_option() {
    #[derive(DeriveConfig, Debug)]
    struct OptionalPathConfig {
        #[feuilletage(relative_path)]
        optional_path: Option<String>,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_optional_path.yaml");

    // Test with value present
    let config_content = r#"optional_path: "some/path""#;
    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: OptionalPathConfig = loader.deserialize().expect("Should succeed");

    assert!(result.optional_path.is_some());
    let path = result.optional_path.unwrap();
    assert!(
        PathBuf::from(&path).is_absolute(),
        "optional_path should be absolute, got: {}",
        path
    );

    // Test with value absent
    let config_content = r#"{}"#;
    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = feuilletage::loader().load_file(&config_path, Level::User);

    let result: OptionalPathConfig = loader.deserialize().expect("Should succeed");

    assert!(
        result.optional_path.is_none(),
        "Optional path should be None when not provided"
    );

    // Cleanup
    let _ = std::fs::remove_file(&config_path);
}

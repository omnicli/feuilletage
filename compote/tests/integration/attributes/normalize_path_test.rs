//! Tests for the `normalize_path` transform attribute shortcut.
//!
//! `#[compote(normalize_path)]` is a shorthand for `#[compote(transform = "normalize_path")]`.
//! It resolves `.` and `..` components in a path string without touching the filesystem
//! and without making relative paths absolute.

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Core acceptance test: `/foo/../bar/./baz` should normalize to `/bar/baz`.
#[test]
fn test_normalize_path_resolves_dot_and_dotdot() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[compote(normalize_path)]
        data_dir: String,
    }

    let config_str = r#"{"data_dir": "/foo/../bar/./baz"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: PathConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.data_dir, "/bar/baz",
        "normalize_path should resolve `.` and `..` components"
    );
}

/// Already-normalized absolute paths should pass through unchanged.
#[test]
fn test_normalize_path_already_normalized_unchanged() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[compote(normalize_path)]
        data_dir: String,
    }

    let config_str = r#"{"data_dir": "/already/normal/path"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: PathConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.data_dir, "/already/normal/path");
}

/// Relative paths should be normalized but stay relative
/// (unlike `relative_path`, which resolves against the config file's directory).
#[test]
fn test_normalize_path_keeps_relative_paths_relative() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[compote(normalize_path)]
        data_dir: String,
    }

    let config_str = r#"{"data_dir": "foo/./bar/../baz"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: PathConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.data_dir, "foo/baz",
        "normalize_path should not promote relative paths to absolute"
    );
}

/// Works on `Option<String>` fields too.
#[test]
fn test_normalize_path_with_option() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[compote(normalize_path)]
        maybe_path: Option<String>,
    }

    // Present case
    let config_str = r#"{"maybe_path": "/a/b/../c"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: PathConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.maybe_path.as_deref(), Some("/a/c"));

    // Absent case
    let config_str = r#"{}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: PathConfig = config.deserialize().expect("Should succeed");
    assert!(result.maybe_path.is_none());
}

/// Loading from a YAML file should normalize the same way.
#[test]
fn test_normalize_path_from_yaml_file() {
    #[derive(DeriveConfig, Debug)]
    struct PathConfig {
        #[compote(normalize_path)]
        path: String,
    }

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("test_normalize_path.yaml");
    let config_content = r#"path: "/foo/../bar/./baz""#;
    std::fs::write(&config_path, config_content).expect("Failed to write temp config");

    let mut loader = compote::loader().load_file(&config_path, Level::User);
    let result: PathConfig = loader.deserialize().expect("Should succeed");

    assert_eq!(result.path, "/bar/baz");

    let _ = std::fs::remove_file(&config_path);
}

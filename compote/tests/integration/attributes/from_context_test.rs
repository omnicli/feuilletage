//! Tests for the from_context field attribute.
//!
//! Fields with from_context are populated from context metadata rather than
//! from the input object, and are skipped during serialization.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;
use std::path::PathBuf;

/// Test from_context with source.file_path (Option<PathBuf>)
#[test]
fn test_from_context_source_file_path_option() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithPath {
        #[compote(default)]
        name: String,
        #[compote(from_context = "source.file_path")]
        source_path: Option<PathBuf>,
    }

    // Load from a file source
    let config_str = r#"{"name": "test"}"#;

    let mut config = Config::default();
    config.load_json(
        config_str,
        Context::new(
            Source::File(PathBuf::from("/etc/myapp/config.json")),
            Level::User,
        ),
    );

    let result: ConfigWithPath = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(
        result.source_path,
        Some(PathBuf::from("/etc/myapp/config.json"))
    );
}

/// Test from_context with source.file_path when no file (programmatic source)
#[test]
fn test_from_context_source_file_path_option_none() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithPath {
        #[compote(default)]
        name: String,
        #[compote(from_context = "source.file_path")]
        source_path: Option<PathBuf>,
    }

    // Load from programmatic source (no file)
    let config_str = r#"{"name": "test"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ConfigWithPath = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(result.source_path, None);
}

/// Test from_context with level.name
#[test]
fn test_from_context_level_name() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithLevel {
        #[compote(default)]
        name: String,
        #[compote(from_context = "level.name")]
        scope: String,
    }

    let config_str = r#"{"name": "test"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ConfigWithLevel = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(result.scope, "user");
}

/// Test from_context with source.display_name
#[test]
fn test_from_context_source_display_name() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithSource {
        #[compote(default)]
        name: String,
        #[compote(from_context = "source.display_name")]
        source_name: String,
    }

    let config_str = r#"{"name": "test"}"#;

    let mut config = Config::default();
    config.load_json(
        config_str,
        Context::new(
            Source::File(PathBuf::from("/etc/myapp/config.json")),
            Level::System,
        ),
    );

    let result: ConfigWithSource = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(result.source_name, "/etc/myapp/config.json");
}

/// Test that from_context fields are skipped in serialization
#[test]
fn test_from_context_skip_serialize() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithContext {
        name: String,
        #[compote(from_context = "level.name")]
        scope: String,
    }

    let config = ConfigWithContext {
        name: "test".to_string(),
        scope: "user".to_string(),
    };

    let serialized = serde_json::to_string(&config).expect("Should serialize");
    // scope should be skipped
    assert_eq!(serialized, r#"{"name":"test"}"#);
}

/// Test from_context with multiple context fields
#[test]
fn test_from_context_multiple_fields() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ConfigWithMultipleContext {
        #[compote(default)]
        name: String,
        #[compote(from_context = "source.file_path")]
        source_path: Option<PathBuf>,
        #[compote(from_context = "level.name")]
        scope: String,
        #[compote(from_context = "source.display_name")]
        source_display: String,
    }

    let config_str = r#"{"name": "multi"}"#;

    let mut config = Config::default();
    config.load_json(
        config_str,
        Context::new(
            Source::File(PathBuf::from("/home/user/.config/app.json")),
            Level::User,
        ),
    );

    let result: ConfigWithMultipleContext = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "multi");
    assert_eq!(
        result.source_path,
        Some(PathBuf::from("/home/user/.config/app.json"))
    );
    assert_eq!(result.scope, "user");
    assert_eq!(result.source_display, "/home/user/.config/app.json");
}

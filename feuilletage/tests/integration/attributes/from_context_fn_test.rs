//! Tests for the from_context_fn field attribute.
//!
//! Fields with from_context_fn are populated by calling a user-provided function
//! with the context metadata, and are skipped during serialization.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;
use std::path::PathBuf;

fn is_not_local<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
    ctx: &feuilletage::Context<S, L>,
) -> bool {
    ctx.level.name() != "local"
}

fn source_path_from_context<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
    ctx: &feuilletage::Context<S, L>,
) -> Option<PathBuf> {
    ctx.source.file_path().map(|p| p.to_path_buf())
}

fn level_name_upper<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
    ctx: &feuilletage::Context<S, L>,
) -> String {
    ctx.level.name().to_uppercase()
}

/// Test from_context_fn with a bool-returning function
#[test]
fn test_from_context_fn_bool() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnBool {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "is_not_local")]
        is_global: bool,
    }

    // With user-level context (not local)
    let config_str = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WithContextFnBool = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert!(result.is_global, "user level should be 'not local'");
}

/// Test from_context_fn with local level returning false
#[test]
fn test_from_context_fn_bool_local() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnBool {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "is_not_local")]
        is_global: bool,
    }

    let config_str = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::Local));

    let result: WithContextFnBool = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert!(!result.is_global, "local level should be 'local'");
}

/// Test from_context_fn with Option return type
#[test]
fn test_from_context_fn_option() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnOption {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "source_path_from_context")]
        source_path: Option<PathBuf>,
    }

    // With file source
    let config_str = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(
        config_str,
        Context::new(
            Source::File(PathBuf::from("/etc/app/config.json")),
            Level::System,
        ),
    );

    let result: WithContextFnOption = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(
        result.source_path,
        Some(PathBuf::from("/etc/app/config.json"))
    );
}

/// Test from_context_fn with Option return type when no file (programmatic source)
#[test]
fn test_from_context_fn_option_none() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnOption {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "source_path_from_context")]
        source_path: Option<PathBuf>,
    }

    let config_str = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WithContextFnOption = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(result.source_path, None);
}

/// Test from_context_fn with String return type
#[test]
fn test_from_context_fn_string() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnString {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "level_name_upper")]
        scope: String,
    }

    let config_str = r#"{"name": "test"}"#;
    let mut config = Config::default();
    config.load_json(
        config_str,
        Context::new(Source::Programmatic, Level::System),
    );

    let result: WithContextFnString = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "test");
    assert_eq!(result.scope, "SYSTEM");
}

/// Test that from_context_fn fields are skipped in serialization
#[test]
fn test_from_context_fn_skip_serialize() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithContextFnSerialize {
        name: String,
        #[feuilletage(from_context_fn = "is_not_local")]
        is_global: bool,
    }

    let value = WithContextFnSerialize {
        name: "test".to_string(),
        is_global: true,
    };

    let serialized = serde_json::to_string(&value).expect("Should serialize");
    // is_global should be skipped
    assert_eq!(serialized, r#"{"name":"test"}"#);
}

/// Test from_context_fn with multiple context fn fields
#[test]
fn test_from_context_fn_multiple_fields() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct WithMultipleContextFn {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(from_context_fn = "is_not_local")]
        is_global: bool,
        #[feuilletage(from_context_fn = "source_path_from_context")]
        source_path: Option<PathBuf>,
        #[feuilletage(from_context_fn = "level_name_upper")]
        scope: String,
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

    let result: WithMultipleContextFn = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "multi");
    assert!(result.is_global);
    assert_eq!(
        result.source_path,
        Some(PathBuf::from("/home/user/.config/app.json"))
    );
    assert_eq!(result.scope, "USER");
}

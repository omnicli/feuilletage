//! default_fn attribute tests

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

fn get_default_name() -> String {
    "default_from_fn".to_string()
}

fn get_default_count() -> i32 {
    42
}

#[derive(Debug, DeriveConfig)]
struct DefaultFnConfig {
    #[compote(default_fn = "get_default_name")]
    name: String,

    #[compote(default_fn = "get_default_count")]
    count: i32,
}

#[test]
fn test_default_fn_when_missing() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DefaultFnConfig>();
    assert!(
        result.is_ok(),
        "Should use default_fn when field is missing"
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "default_from_fn");
    assert_eq!(cfg.count, 42);
}

#[test]
fn test_default_fn_overridden() {
    let json = r#"{"name": "custom", "count": 100}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DefaultFnConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "custom");
    assert_eq!(cfg.count, 100);
}

#[test]
fn test_default_fn_partial_override() {
    let json = r#"{"name": "custom"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DefaultFnConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "custom");
    assert_eq!(cfg.count, 42); // From default_fn
}

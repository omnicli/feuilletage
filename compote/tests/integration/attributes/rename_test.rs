//! Rename attribute tests

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
struct RenameConfig {
    #[compote(rename = "userName")]
    user_name: String,

    #[compote(rename = "itemCount")]
    item_count: i32,

    // Field without rename for comparison
    normal_field: String,
}

#[test]
fn test_rename_basic_deserialization() {
    let json = r#"{
        "userName": "alice",
        "itemCount": 42,
        "normal_field": "value"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RenameConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.user_name, "alice");
    assert_eq!(cfg.item_count, 42);
    assert_eq!(cfg.normal_field, "value");
}

#[test]
fn test_rename_basic_serialization() {
    let cfg = RenameConfig {
        user_name: "bob".to_string(),
        item_count: 100,
        normal_field: "test".to_string(),
    };

    let serialized = compote::to_json_compact(&cfg).unwrap();

    assert!(
        serialized.contains(r#""userName":"bob""#),
        "Should use renamed key userName"
    );
    assert!(
        serialized.contains(r#""itemCount":100"#),
        "Should use renamed key itemCount"
    );
    assert!(
        serialized.contains(r#""normal_field":"test""#),
        "Should use original field name"
    );
}

#[test]
fn test_rename_original_field_name_not_used() {
    let json = r#"{
        "user_name": "should_not_work",
        "item_count": 999,
        "normal_field": "value"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RenameConfig>();
    assert!(
        result.is_err(),
        "Should fail when using original field names instead of renamed keys"
    );
}

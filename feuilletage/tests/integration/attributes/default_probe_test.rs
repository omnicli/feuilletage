//! Tests for diagnostics produced while probing `#[feuilletage(default)]` values.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};

#[test]
fn missing_defaulted_collections_discard_probe_diagnostics() {
    #[derive(Debug, Default, feuilletage::Config, PartialEq)]
    #[feuilletage(transparent)]
    struct TransparentItems(Vec<String>);

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct CollectionDefaults {
        #[feuilletage(default)]
        items: Vec<String>,
        #[feuilletage(default)]
        transparent_items: TransparentItems,
    }

    let mut config = Config::default();
    config.load_json("{}", Context::new(Source::Programmatic, Level::User));

    let result: CollectionDefaults = config.deserialize().expect("defaults should deserialize");
    assert!(result.items.is_empty());
    assert!(result.transparent_items.0.is_empty());
    assert!(!config.errors().has_errors());
    assert!(!config.errors().has_warnings());
}

#[test]
fn missing_nested_struct_uses_derived_field_defaults_without_probe_diagnostics() {
    #[derive(Debug, Default, feuilletage::Config, PartialEq)]
    struct NestedDefaults {
        #[feuilletage(default = "configured")]
        name: String,
    }

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct Parent {
        #[feuilletage(default)]
        nested: NestedDefaults,
    }

    let mut config = Config::default();
    config.load_json("{}", Context::new(Source::Programmatic, Level::User));

    let result: Parent = config
        .deserialize()
        .expect("nested defaults should deserialize");
    assert_eq!(result.nested.name, "configured");
    assert!(!config.errors().has_errors());
}

#[test]
fn missing_nested_required_field_falls_back_to_rust_default_without_probe_diagnostics() {
    #[derive(Debug, Default, feuilletage::Config, PartialEq)]
    struct NestedRequired {
        required: String,
    }

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct Parent {
        #[feuilletage(default)]
        nested: NestedRequired,
    }

    let mut config = Config::default();
    config.load_json("{}", Context::new(Source::Programmatic, Level::User));

    let result: Parent = config.deserialize().expect("Rust default should be used");
    assert_eq!(result.nested, NestedRequired::default());
    assert!(!config.errors().has_errors());
}

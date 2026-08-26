//! Tests for diagnostics produced while probing `#[compote(default)]` values.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};

#[test]
fn missing_defaulted_collections_discard_probe_diagnostics() {
    #[derive(Debug, Default, compote::Config, PartialEq)]
    #[compote(transparent)]
    struct TransparentItems(Vec<String>);

    #[derive(Debug, compote::Config, PartialEq)]
    struct CollectionDefaults {
        #[compote(default)]
        items: Vec<String>,
        #[compote(default)]
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
    #[derive(Debug, Default, compote::Config, PartialEq)]
    struct NestedDefaults {
        #[compote(default = "configured")]
        name: String,
    }

    #[derive(Debug, compote::Config, PartialEq)]
    struct Parent {
        #[compote(default)]
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
    #[derive(Debug, Default, compote::Config, PartialEq)]
    struct NestedRequired {
        required: String,
    }

    #[derive(Debug, compote::Config, PartialEq)]
    struct Parent {
        #[compote(default)]
        nested: NestedRequired,
    }

    let mut config = Config::default();
    config.load_json("{}", Context::new(Source::Programmatic, Level::User));

    let result: Parent = config.deserialize().expect("Rust default should be used");
    assert_eq!(result.nested, NestedRequired::default());
    assert!(!config.errors().has_errors());
}

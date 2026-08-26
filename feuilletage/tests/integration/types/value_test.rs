//! Tests for the public `feuilletage::Value` type as a deserialization target
//! and Default-able field type.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source, Value};
use feuilletage_macros::Config as DeriveConfig;

#[test]
fn value_default_is_null() {
    assert_eq!(Value::default(), Value::Null);
}

#[test]
fn from_context_value_preserves_scalars() {
    let mut config = Config::default();
    config.load_json(
        r#"{"x": 42}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    #[derive(DeriveConfig)]
    struct C {
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    assert_eq!(c.x, Value::Int(42));
}

#[test]
fn from_context_value_preserves_arrays() {
    let mut config = Config::default();
    config.load_json(
        r#"{"x": [1, "two", true]}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    #[derive(DeriveConfig)]
    struct C {
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    assert_eq!(
        c.x,
        Value::Array(vec![
            Value::Int(1),
            Value::String("two".into()),
            Value::Bool(true)
        ])
    );
}

#[test]
fn from_context_value_preserves_objects() {
    let mut config = Config::default();
    config.load_json(
        r#"{"x": {"a": 1, "b": 2}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    #[derive(DeriveConfig)]
    struct C {
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    if let Value::Object(map) = c.x {
        assert_eq!(map.get("a"), Some(&Value::Int(1)));
        assert_eq!(map.get("b"), Some(&Value::Int(2)));
    } else {
        panic!("expected Object, got {:?}", c.x);
    }
}

// Notes on `#[feuilletage(default)]` vs `Value::default()`:
//
// `Value::default()` returns `Value::Null`. However, the derive macro's
// `#[feuilletage(default)]` codegen for non-Option fields tries to deserialize
// an empty `Object` ContextValue via `FromContextValue` first, falling back
// to `Default::default()` only if that fails. This empty-object trick is
// load-bearing for nested derived structs whose `Default` impl does not
// invoke their field-level `#[feuilletage(default = ...)]` annotations.
//
// Because `FromContextValue<S, L> for Value` is lossless (it preserves any
// value tree including an empty Object), the empty-object trick succeeds
// and returns `Value::Object({})` rather than falling back to
// `Value::default() == Value::Null`. The user-level workaround is to write
// `#[feuilletage(default = "feuilletage::Value::Null")]`, which sidesteps the
// empty-object trick entirely.

#[test]
fn missing_value_field_uses_default() {
    let mut config = Config::default();
    config.load_json(r#"{}"#, Context::new(Source::Programmatic, Level::User));

    #[derive(DeriveConfig)]
    struct C {
        #[feuilletage(default = "feuilletage::Value::Null")]
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    assert_eq!(c.x, Value::Null);
}

#[test]
fn explicit_null_value_field() {
    let mut config = Config::default();
    config.load_json(
        r#"{"x": null}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    #[derive(DeriveConfig)]
    struct C {
        #[feuilletage(default = "feuilletage::Value::Null")]
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    assert_eq!(c.x, Value::Null);
}

/// Documents the bare-`#[feuilletage(default)]` interaction with the macro's
/// empty-object trick. Both "missing field" and "explicit null" go through
/// `missing_field_handling`, which synthesizes an empty `Object` ContextValue
/// and calls `FromContextValue::from_context_value`. Since the new
/// `FromContextValue<S, L> for Value` impl preserves that tree, the result
/// is `Value::Object({})`, not `Value::Null`. Use `default = "..."` (above)
/// to get `Value::Null` semantics.
#[test]
fn bare_default_attr_yields_empty_object_due_to_macro_trick() {
    let mut config = Config::default();
    config.load_json(r#"{}"#, Context::new(Source::Programmatic, Level::User));

    #[derive(DeriveConfig)]
    struct C {
        #[feuilletage(default)]
        x: Value,
    }

    let c: C = config.deserialize().expect("should deserialize");
    assert_eq!(c.x, Value::Object(Default::default()));
}

//! Tests for the struct-level `#[compote(transform = "fn")]` attribute.
//!
//! The container-level `transform` attribute runs a normalizer function on
//! the raw `ContextValue` input BEFORE any field deserialization or
//! `scalar_as` / `array_as` wrapping. This lets a struct accept multiple
//! input shapes and normalize them to a canonical form declaratively.

#![cfg(feature = "json")]

use compote::{Config, Context, ContextValue, CustomLevel, CustomSource, Error, Level, Source};
use compote_macros::Config as DeriveConfig;
use indexmap::IndexMap;

// ============================================================================
// Helpers
// ============================================================================

/// If the input Object has key "name" but not "username", rename "name" to
/// "username". This lets the struct accept either spelling.
fn rename_name_to_username<S: CustomSource, L: CustomLevel>(
    value: &mut ContextValue<S, L>,
    _ctx: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::Object(map, _) = value {
        if !map.contains_key("username") {
            if let Some(name_value) = map.shift_remove("name") {
                map.insert("username".to_string(), name_value);
            }
        }
    }
    Ok(())
}

/// If input is not an Object, wrap as `{x: value}`. Otherwise leave alone.
/// (Like scalar_as but unconditional on input shape.)
fn wrap_non_object_as_x<S: CustomSource, L: CustomLevel>(
    value: &mut ContextValue<S, L>,
    _ctx: &Context<S, L>,
) -> Result<(), Error> {
    if matches!(value, ContextValue::Object(_, _)) {
        return Ok(());
    }
    let ctx = value.context().clone();
    let original = std::mem::replace(value, ContextValue::Null(ctx.clone()));
    let mut map = IndexMap::new();
    map.insert("x".to_string(), original);
    *value = ContextValue::object(map, ctx);
    Ok(())
}

/// If input is an Object containing `payload`, leave alone. Otherwise wrap
/// the entire input as `{payload: original}`. (The SuggestConfig pattern.)
fn wrap_unrecognized_as_payload<S: CustomSource, L: CustomLevel>(
    value: &mut ContextValue<S, L>,
    _ctx: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::Object(map, _) = value {
        if map.contains_key("payload") {
            return Ok(());
        }
    }
    let ctx = value.context().clone();
    let original = std::mem::replace(value, ContextValue::Null(ctx.clone()));
    let mut map = IndexMap::new();
    map.insert("payload".to_string(), original);
    *value = ContextValue::object(map, ctx);
    Ok(())
}

/// Convert any Int input to its string representation. Used to test ordering:
/// container transform runs BEFORE scalar_as.
fn int_to_string<S: CustomSource, L: CustomLevel>(
    value: &mut ContextValue<S, L>,
    _ctx: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::Int(n, _) = value {
        let s = n.to_string();
        let ctx = value.context().clone();
        *value = ContextValue::String(s, ctx);
    }
    Ok(())
}

/// Always fail. Used to test error propagation.
fn always_fail<S: CustomSource, L: CustomLevel>(
    _value: &mut ContextValue<S, L>,
    _ctx: &Context<S, L>,
) -> Result<(), Error> {
    Err(Error::InvalidValue {
        path: String::new(),
        message: "intentional failure from transform".to_string(),
    })
}

// ============================================================================
// 1. Rename a field via transform
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transform = "self::rename_name_to_username")]
struct UserConfig {
    username: String,
    age: Option<i64>,
}

#[test]
fn test_container_transform_renames_field() {
    // Input uses 'name'; transform renames it to 'username' before deser.
    let json = r#"{"name": "alice", "age": 30}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: UserConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.username, "alice");
    assert_eq!(result.age, Some(30));
}

#[test]
fn test_container_transform_passthrough_when_canonical() {
    // Input already canonical: transform is a no-op.
    let json = r#"{"username": "bob"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: UserConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.username, "bob");
    assert_eq!(result.age, None);
}

// ============================================================================
// 2. Wrap a scalar input conditionally
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transform = "self::wrap_non_object_as_x")]
struct Boxed {
    x: String,
}

#[test]
fn test_container_transform_wraps_scalar() {
    let json = r#""hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: Boxed = config.deserialize().expect("Should deserialize");
    assert_eq!(result.x, "hello");
}

#[test]
fn test_container_transform_leaves_object_alone() {
    let json = r#"{"x": "world"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: Boxed = config.deserialize().expect("Should deserialize");
    assert_eq!(result.x, "world");
}

// ============================================================================
// 3. The SuggestConfig pattern: object without canonical key
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transform = "self::wrap_unrecognized_as_payload")]
struct SuggestConfig {
    payload: PayloadValue,
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transparent)]
struct PayloadValue(String);

#[test]
fn test_container_transform_wraps_when_payload_missing() {
    // Input is a bare string; transform wraps as {payload: "..."}
    let json = r#""raw-value""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: SuggestConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.payload, PayloadValue("raw-value".to_string()));
}

#[test]
fn test_container_transform_passthrough_when_payload_present() {
    let json = r#"{"payload": "explicit"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: SuggestConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.payload, PayloadValue("explicit".to_string()));
}

// ============================================================================
// 4. Transform runs BEFORE scalar_as (ordering test)
// ============================================================================
//
// Transform converts Int -> String. scalar_as then wraps the String as
// {value: "..."}. If ordering were reversed, scalar_as would have wrapped
// the Int first and the transform would never see it as a scalar.

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transform = "self::int_to_string", scalar_as = "value")]
struct StringifyThenWrap {
    value: String,
}

#[test]
fn test_container_transform_runs_before_scalar_as() {
    // Int input -> transform stringifies -> scalar_as wraps as {value: "42"}.
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: StringifyThenWrap = config.deserialize().expect("Should deserialize");
    assert_eq!(result.value, "42");
}

#[test]
fn test_container_transform_runs_before_scalar_as_string_input() {
    // String input -> transform is a no-op -> scalar_as wraps it.
    let json = r#""already-a-string""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: StringifyThenWrap = config.deserialize().expect("Should deserialize");
    assert_eq!(result.value, "already-a-string");
}

// ============================================================================
// 5. Transform returning Err propagates as deserialize failure
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transform = "self::always_fail")]
struct AlwaysFails {
    #[allow(dead_code)]
    field: Option<String>,
}

#[test]
fn test_container_transform_error_propagates() {
    let json = r#"{"field": "irrelevant"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: Result<AlwaysFails, _> = config.deserialize();
    assert!(result.is_err(), "Expected transform error to propagate");
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("intentional failure"),
        "Expected error message to contain 'intentional failure', got: {}",
        msg
    );
}

// ============================================================================
// 6. Container transform on a transparent newtype struct
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transparent, transform = "self::int_to_string")]
struct StringyNewtype(String);

#[test]
fn test_container_transform_on_transparent_struct() {
    // Int input -> transform stringifies -> transparent wraps the inner String.
    let json = r#"123"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: StringyNewtype = config.deserialize().expect("Should deserialize");
    assert_eq!(result.0, "123");
}

#[test]
fn test_container_transform_on_transparent_struct_passthrough() {
    let json = r#""hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result: StringyNewtype = config.deserialize().expect("Should deserialize");
    assert_eq!(result.0, "hello");
}

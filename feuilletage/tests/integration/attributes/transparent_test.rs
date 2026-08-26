//! Tests for the transparent container attribute.
//!
//! Transparent structs serialize/deserialize as their single inner field directly,
//! without wrapping in an object.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

/// Test transparent with Vec
#[test]
fn test_transparent_vec() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct WrappedVec {
        items: Vec<String>,
    }

    // Deserialize from array directly
    let config_str = r#"["a", "b", "c"]"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WrappedVec = config.deserialize().expect("Should deserialize");
    assert_eq!(result.items, vec!["a", "b", "c"]);

    // Serialize as array directly
    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert_eq!(serialized, r#"["a","b","c"]"#);
}

/// Test transparent with single value
#[test]
fn test_transparent_single_value() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct WrappedString {
        value: String,
    }

    // Deserialize from string directly
    let config_str = r#""hello world""#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WrappedString = config.deserialize().expect("Should deserialize");
    assert_eq!(result.value, "hello world");

    // Serialize as string directly
    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert_eq!(serialized, r#""hello world""#);
}

/// Test transparent with nested struct
#[test]
fn test_transparent_nested_struct() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct Inner {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(default = "0")]
        count: i32,
    }

    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct Wrapper {
        inner: Inner,
    }

    // Deserialize - the object goes directly to Inner
    let config_str = r#"{"name": "test", "count": 42}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Wrapper = config.deserialize().expect("Should deserialize");
    assert_eq!(result.inner.name, "test");
    assert_eq!(result.inner.count, 42);

    // Serialize - outputs Inner directly
    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert!(serialized.contains(r#""name":"test""#));
    assert!(serialized.contains(r#""count":42"#));
}

/// Test transparent with skip_serialize
#[test]
fn test_transparent_skip_serialize() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent, skip_serialize)]
    struct ReadOnlyVec {
        items: Vec<i32>,
    }

    // Deserialize works
    let config_str = r#"[1, 2, 3]"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: ReadOnlyVec = config.deserialize().expect("Should deserialize");
    assert_eq!(result.items, vec![1, 2, 3]);

    // Serialize is not implemented (would fail to compile if we tried)
}

/// Test transparent with integer
#[test]
fn test_transparent_integer() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct WrappedInt {
        value: i64,
    }

    let config_str = r#"42"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WrappedInt = config.deserialize().expect("Should deserialize");
    assert_eq!(result.value, 42);

    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert_eq!(serialized, "42");
}

// ============================================================================
// Tuple-struct transparent (single-field newtype) tests
// ============================================================================

/// Tuple-struct transparent with Vec.
#[test]
fn test_transparent_tuple_vec() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct WrappedVec(pub Vec<String>);

    let config_str = r#"["a", "b", "c"]"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WrappedVec = config.deserialize().expect("Should deserialize");
    assert_eq!(result.0, vec!["a", "b", "c"]);

    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert_eq!(serialized, r#"["a","b","c"]"#);
}

/// Tuple-struct transparent with a scalar.
#[test]
fn test_transparent_tuple_int() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent)]
    struct WrappedInt(pub i64);

    let config_str = "42";
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: WrappedInt = config.deserialize().expect("Should deserialize");
    assert_eq!(result.0, 42);

    let serialized = serde_json::to_string(&result).expect("Should serialize");
    assert_eq!(serialized, "42");
}

/// Tuple-struct transparent wrapping a named-field derive type, with skip_serialize.
#[test]
fn test_transparent_tuple_named_inner_skip_serialize() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct InnerStruct {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(default = "0")]
        count: i32,
    }

    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent, skip_serialize)]
    struct WrappedOnce(pub InnerStruct);

    let config_str = r#"{"name": "hello", "count": 7}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let wrapped: WrappedOnce = config
        .deserialize()
        .expect("Should deserialize WrappedOnce");
    assert_eq!(wrapped.0.name, "hello");
    assert_eq!(wrapped.0.count, 7);

    // Deserializing the same input directly as the inner struct should give the same data.
    let mut config2 = Config::default();
    config2.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let direct: InnerStruct = config2
        .deserialize()
        .expect("Should deserialize InnerStruct");
    assert_eq!(wrapped.0, direct);
}

/// Tuple-struct transparent with `post_process` (the omni `UpConfigBash` shape).
#[test]
fn test_transparent_tuple_post_process() {
    use feuilletage::{ContextValue, ErrorTracker};

    #[derive(DeriveConfig, Debug, PartialEq)]
    struct Inner {
        #[feuilletage(default)]
        name: String,
        #[feuilletage(default = "0")]
        version: i32,
    }

    fn finalize<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
        config: &mut Wrapped,
        _value: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<(), feuilletage::Error> {
        // Mutate the inner struct via `self.0`.
        config.0.name = format!("{}!", config.0.name);
        if config.0.version == 0 {
            config.0.version = 99;
        }
        Ok(())
    }

    #[derive(DeriveConfig, Debug, PartialEq)]
    #[feuilletage(transparent, post_process = "finalize")]
    struct Wrapped(pub Inner);

    // Default version triggers the post_process override.
    let mut config = Config::default();
    config.load_json(
        r#"{"name": "bash"}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    let result: Wrapped = config.deserialize().expect("Should deserialize");
    assert_eq!(result.0.name, "bash!");
    assert_eq!(result.0.version, 99);

    // Provided version is preserved (only the `!` suffix is added).
    let mut config2 = Config::default();
    config2.load_json(
        r#"{"name": "zsh", "version": 5}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    let result2: Wrapped = config2.deserialize().expect("Should deserialize");
    assert_eq!(result2.0.name, "zsh!");
    assert_eq!(result2.0.version, 5);

    // Serializes as the inner struct directly.
    let serialized = serde_json::to_string(&result2).expect("Should serialize");
    assert!(serialized.contains(r#""name":"zsh!""#));
    assert!(serialized.contains(r#""version":5"#));
}

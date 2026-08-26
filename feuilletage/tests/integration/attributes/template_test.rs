//! Integration tests for template attribute.
//!
//! The `#[feuilletage(template)]` attribute enables template interpolation using
//! `%{field}` syntax to reference other fields in the same struct.
//!
//! Unit tests for the template module are in feuilletage/tests/unit/template_test.rs.
//! These integration tests verify the full macro-driven behavior.

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Basic template interpolation tests
// ============================================================================

#[test]
fn test_template_basic_interpolation() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct BasicTemplateConfig {
        host: String,
        port: i32,

        #[feuilletage(template)]
        url: String,
    }

    let json = r#"{
        "host": "localhost",
        "port": 8080,
        "url": "http://%{host}:%{port}/api"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BasicTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.host, "localhost");
    assert_eq!(result.port, 8080);
    assert_eq!(result.url, "http://localhost:8080/api");
    assert!(!config.errors().has_errors());
}

#[test]
fn test_template_single_reference() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SingleRefConfig {
        name: String,

        #[feuilletage(template)]
        greeting: String,
    }

    let json = r#"{
        "name": "World",
        "greeting": "Hello, %{name}!"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SingleRefConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.name, "World");
    assert_eq!(result.greeting, "Hello, World!");
}

#[test]
fn test_template_no_references() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct NoRefConfig {
        value: String,

        #[feuilletage(template)]
        plain: String,
    }

    let json = r#"{
        "value": "unused",
        "plain": "no references here"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: NoRefConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.plain, "no references here");
}

// ============================================================================
// Template with explicit refs
// ============================================================================

#[test]
fn test_template_with_refs() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ExplicitRefsConfig {
        name: String,
        unused: String,

        #[feuilletage(template(refs = ["name"]))]
        greeting: String,
    }

    let json = r#"{
        "name": "Alice",
        "unused": "not referenced",
        "greeting": "Hello, %{name}!"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExplicitRefsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.greeting, "Hello, Alice!");
}

#[test]
fn test_template_with_multiple_refs() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MultiRefsConfig {
        first_name: String,
        last_name: String,

        #[feuilletage(template(refs = ["first_name", "last_name"]))]
        full_name: String,
    }

    let json = r#"{
        "first_name": "John",
        "last_name": "Doe",
        "full_name": "%{first_name} %{last_name}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiRefsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.full_name, "John Doe");
}

// ============================================================================
// Template with vec_delimiter
// ============================================================================

// Note: Template interpolation with Vec references and vec_delimiter is an advanced
// feature. The current implementation may have limitations with referencing Vec fields.
// These tests document the expected behavior when this feature is fully supported.

// TODO: Vec template interpolation tests can be added once the feature is fully implemented
// The template module supports vec_delimiter in value_to_string(), but the macro-generated
// code may not properly collect Vec values into the template context.

// ============================================================================
// Template chains (template referencing other templates)
// ============================================================================

#[test]
fn test_template_chain() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct ChainConfig {
        scheme: String,
        host: String,
        port: i32,

        #[feuilletage(template)]
        base_url: String,

        #[feuilletage(template)]
        api_url: String,

        #[feuilletage(template)]
        health_url: String,
    }

    let json = r#"{
        "scheme": "https",
        "host": "api.example.com",
        "port": 443,
        "base_url": "%{scheme}://%{host}:%{port}",
        "api_url": "%{base_url}/v1",
        "health_url": "%{api_url}/health"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ChainConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.base_url, "https://api.example.com:443");
    assert_eq!(result.api_url, "https://api.example.com:443/v1");
    assert_eq!(result.health_url, "https://api.example.com:443/v1/health");
}

// ============================================================================
// Template with different types
// ============================================================================

#[test]
fn test_template_with_int() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct IntTemplateConfig {
        count: i32,

        #[feuilletage(template)]
        message: String,
    }

    let json = r#"{
        "count": 42,
        "message": "Count is %{count}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: IntTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.message, "Count is 42");
}

#[test]
fn test_template_with_bool() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct BoolTemplateConfig {
        enabled: bool,

        #[feuilletage(template)]
        status: String,
    }

    let json = r#"{
        "enabled": true,
        "status": "Feature is %{enabled}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BoolTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.status, "Feature is true");
}

#[test]
fn test_template_with_float() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct FloatTemplateConfig {
        ratio: f64,

        #[feuilletage(template)]
        display: String,
    }

    let json = r#"{
        "ratio": 3.14,
        "display": "Ratio: %{ratio}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: FloatTemplateConfig = config.deserialize().expect("Should succeed");

    assert!(result.display.starts_with("Ratio: 3.14"));
}

// ============================================================================
// Template escape sequence
// ============================================================================

#[test]
fn test_template_escape_sequence() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct EscapeConfig {
        name: String,

        #[feuilletage(template)]
        message: String,
    }

    let json = r#"{
        "name": "test",
        "message": "Literal %%{name} and interpolated %{name}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: EscapeConfig = config.deserialize().expect("Should succeed");

    // %%{ should become literal %{
    assert_eq!(result.message, "Literal %{name} and interpolated test");
}

// ============================================================================
// Template with default value
// ============================================================================

#[test]
fn test_template_with_default() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DefaultTemplateConfig {
        host: String,

        #[feuilletage(template, default = "http://%{host}/default")]
        url: String,
    }

    // Provide explicit value
    let json = r#"{
        "host": "localhost",
        "url": "https://%{host}/custom"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DefaultTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.url, "https://localhost/custom");
}

#[test]
fn test_template_uses_default_when_missing() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct DefaultMissingConfig {
        host: String,

        #[feuilletage(template, default = "default_url")]
        url: String,
    }

    // url is missing, should use default (but default is not a template)
    let json = r#"{"host": "localhost"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: DefaultMissingConfig = config.deserialize().expect("Should succeed");

    // Default value is used as-is (not interpolated since it's the default)
    assert_eq!(result.url, "default_url");
}

// ============================================================================
// Template combined with other attributes
// ============================================================================

#[test]
fn test_template_with_rename() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct RenameTemplateConfig {
        host: String,

        #[feuilletage(rename = "apiUrl", template)]
        api_url: String,
    }

    let json = r#"{
        "host": "example.com",
        "apiUrl": "https://%{host}/api"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: RenameTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.api_url, "https://example.com/api");
}

// ============================================================================
// Multiple template fields
// ============================================================================

#[test]
fn test_multiple_template_fields() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MultiTemplateConfig {
        app_name: String,
        version: String,
        port: i32,

        #[feuilletage(template)]
        title: String,

        #[feuilletage(template)]
        endpoint: String,

        #[feuilletage(template)]
        user_agent: String,
    }

    let json = r#"{
        "app_name": "MyApp",
        "version": "1.0.0",
        "port": 3000,
        "title": "%{app_name} v%{version}",
        "endpoint": "http://localhost:%{port}",
        "user_agent": "%{app_name}/%{version}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MultiTemplateConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.title, "MyApp v1.0.0");
    assert_eq!(result.endpoint, "http://localhost:3000");
    assert_eq!(result.user_agent, "MyApp/1.0.0");
}

// ============================================================================
// Template with missing referenced field (error case)
// ============================================================================

#[test]
fn test_template_missing_reference_left_as_is() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MissingRefConfig {
        name: String,

        #[feuilletage(template)]
        message: String,
    }

    // References non-existent field "missing"
    let json = r#"{
        "name": "test",
        "message": "Hello %{missing}!"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: MissingRefConfig = config.deserialize().expect("Should succeed");

    // Missing references are left as-is (not an error by default)
    assert_eq!(result.message, "Hello %{missing}!");
}

// ============================================================================
// Template repeated reference
// ============================================================================

#[test]
fn test_template_repeated_reference() {
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct RepeatedRefConfig {
        word: String,

        #[feuilletage(template)]
        repeated: String,
    }

    let json = r#"{
        "word": "echo",
        "repeated": "%{word} %{word} %{word}"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: RepeatedRefConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.repeated, "echo echo echo");
}

// ============================================================================
// Template serialization behavior
// ============================================================================

#[test]
fn test_template_serializes_interpolated_value() {
    // Note: DeriveConfig automatically implements serde::Serialize
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct SerializeTemplateConfig {
        host: String,

        #[feuilletage(template)]
        url: String,
    }

    let json = r#"{
        "host": "example.com",
        "url": "https://%{host}/api"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: SerializeTemplateConfig = config.deserialize().expect("Should succeed");

    // Serialization should output the interpolated value, not the template
    let serialized = feuilletage::to_json_compact(&result).unwrap();
    assert!(serialized.contains("https://example.com/api"));
    assert!(!serialized.contains("%{host}"));
}

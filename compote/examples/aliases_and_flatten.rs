//! Aliases and Flatten Pattern: Complex nested configs with aliases
//!
//! This pattern demonstrates:
//! - Fields with multiple accepted names (aliases)
//! - Nested struct configurations
//! - Flatten for struct composition
//!
//! Compote Solution:
//! - `#[compote(aliases = ["alt_name"])]` for field aliases
//! - `#[compote(flatten)]` for struct composition
//! - `#[compote(rename = "key")]` for key renaming

use compote::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Nested config with aliased fields
#[derive(Debug, compote::Config, PartialEq)]
struct NestedConfig {
    /// Field with an alias ("pattern" also accepted)
    #[compote(aliases = ["pattern"])]
    name: Option<String>,
    /// Field with a short alias ("ext" also accepted)
    #[compote(aliases = ["ext"])]
    extension: Option<String>,
}

/// Config demonstrating field aliases at various levels
#[derive(Debug, compote::Config, PartialEq)]
struct AliasedFieldsConfig {
    /// Primary field with multiple aliases (short and abbreviated forms)
    #[compote(aliases = ["source", "s"])]
    primary_field: String,

    /// Optional field with abbreviated aliases
    #[compote(aliases = ["ver", "v"])]
    version: Option<String>,

    /// Field with a single alias
    #[compote(aliases = ["tag"])]
    aliased_field: Option<String>,

    /// Boolean flag with default
    #[compote(default = "false")]
    enabled: bool,

    /// Nested config field
    #[compote(default)]
    filter: Option<NestedConfig>,

    /// Output path with multiple destination aliases
    #[compote(aliases = ["dest", "out", "output"])]
    target: Option<String>,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Aliases and Flatten Examples ===\n");

    // Basic aliases
    println!("--- Basic Aliases ---");

    let json = r#"{"primary_field": "value1", "version": "2.0.0"}"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("Primary keys");
    println!("Using primary_field: {}", config.primary_field);
    assert_eq!(config.primary_field, "value1");

    let json = r#"{"s": "value1", "v": "2.0.0"}"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("Short aliases");
    println!(
        "Using s and v aliases: primary_field={}, version={:?}",
        config.primary_field, config.version
    );
    assert_eq!(config.primary_field, "value1");
    assert_eq!(config.version, Some("2.0.0".to_string()));

    let json = r#"{"source": "value1", "ver": "14.0.0"}"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("source/ver aliases");
    println!(
        "Using source/ver aliases: primary_field={}, version={:?}",
        config.primary_field, config.version
    );
    assert_eq!(config.primary_field, "value1");
    assert_eq!(config.version, Some("14.0.0".to_string()));

    // Target aliases
    println!("\n--- Target Field Aliases ---");
    for (key, _expected) in [
        ("target", "dest1"),
        ("dest", "dest2"),
        ("out", "dest3"),
        ("output", "dest4"),
    ] {
        let json = format!(r#"{{"source": "foo/bar", "{key}": "{key}_path"}}"#);
        let config: AliasedFieldsConfig = deserialize_json(&json)
            .unwrap_or_else(|_| panic!("Should deserialize with {} alias", key));
        println!("Using {} alias: target={:?}", key, config.target);
        assert_eq!(config.target, Some(format!("{key}_path")));
    }

    // Primary takes precedence
    println!("\n--- Primary Takes Precedence ---");
    let json = r#"{"primary_field": "primary_value", "source": "alias_value"}"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("Primary should win");
    println!(
        "primary_field and source both present: {}",
        config.primary_field
    );
    assert_eq!(config.primary_field, "primary_value");

    // Nested config
    println!("\n--- Nested Config ---");
    let json = r#"{
        "source": "value1",
        "filter": {
            "name": "pattern-.*-test",
            "extension": "tar.gz"
        }
    }"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("Nested config");
    println!("filter.name: {:?}", config.filter.as_ref().unwrap().name);
    println!(
        "filter.extension: {:?}",
        config.filter.as_ref().unwrap().extension
    );
    assert!(config.filter.is_some());
    let filter = config.filter.unwrap();
    assert_eq!(filter.name, Some("pattern-.*-test".to_string()));
    assert_eq!(filter.extension, Some("tar.gz".to_string()));

    // Nested config with aliases
    println!("\n--- Nested Config with Aliases ---");
    let json = r#"{
        "source": "value1",
        "filter": {
            "pattern": "pattern-.*-test",
            "ext": "tar.gz"
        }
    }"#;
    let config: AliasedFieldsConfig = deserialize_json(json).expect("Nested with aliases");
    assert!(config.filter.is_some());
    let filter = config.filter.unwrap();
    println!(
        "Using pattern/ext aliases: name={:?}, extension={:?}",
        filter.name, filter.extension
    );
    assert_eq!(filter.name, Some("pattern-.*-test".to_string()));
    assert_eq!(filter.extension, Some("tar.gz".to_string()));

    // Flatten tests
    println!("\n--- Flatten ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct FlattenableConfig {
        #[compote(aliases = ["token"])]
        api_token: Option<String>,
        #[compote(default = "false")]
        authenticated: bool,
    }

    #[derive(Debug, compote::Config, PartialEq)]
    struct ComposedConfig {
        #[compote(aliases = ["source"])]
        primary_field: String,
        #[compote(flatten)]
        auth: FlattenableConfig,
        #[compote(default = "false")]
        enabled: bool,
    }

    let json = r#"{
        "source": "value1",
        "api_token": "token_xxx",
        "authenticated": true,
        "enabled": false
    }"#;
    let config: ComposedConfig = deserialize_json(json).expect("Flatten basic");
    println!("primary_field: {}", config.primary_field);
    println!("auth.api_token: {:?}", config.auth.api_token);
    println!("auth.authenticated: {}", config.auth.authenticated);
    println!("enabled: {}", config.enabled);
    assert_eq!(config.primary_field, "value1");
    assert_eq!(config.auth.api_token, Some("token_xxx".to_string()));
    assert!(config.auth.authenticated);
    assert!(!config.enabled);

    // Flatten with alias
    println!("\n--- Flatten with Alias ---");
    let json = r#"{"source": "value1", "token": "token_yyy"}"#;
    let config: ComposedConfig = deserialize_json(json).expect("Flatten with alias");
    println!(
        "Using token alias: auth.api_token={:?}",
        config.auth.api_token
    );
    assert_eq!(config.auth.api_token, Some("token_yyy".to_string()));

    // Flatten defaults
    println!("\n--- Flatten Defaults ---");
    let json = r#"{"source": "value1"}"#;
    let config: ComposedConfig = deserialize_json(json).expect("Flatten with defaults");
    println!("auth.api_token: {:?}", config.auth.api_token);
    println!("auth.authenticated: {}", config.auth.authenticated);
    println!("enabled: {}", config.enabled);
    assert_eq!(config.auth.api_token, None);
    assert!(!config.auth.authenticated);
    assert!(!config.enabled);

    // Serialization uses primary keys
    println!("\n--- Serialization Uses Primary Keys ---");
    let json = r#"{"source": "value1", "v": "2.0.0"}"#;
    let config: AliasedFieldsConfig = deserialize_json(json).unwrap();
    let serialized = compote::to_json_compact(&config).unwrap();
    println!("Serialized: {}", serialized);
    assert!(
        serialized.contains("primary_field"),
        "Should use primary key 'primary_field'"
    );
    assert!(
        !serialized.contains("\"source\""),
        "Should not use alias 'source'"
    );

    println!("\n=== All aliases and flatten examples passed! ===");
}

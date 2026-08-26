//! Flatten attribute tests

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Common configuration that can be embedded in other configs
#[derive(Debug, DeriveConfig, PartialEq)]
struct CommonConfig {
    debug: bool,
    #[compote(default = "false")]
    verbose: bool,
}

/// Main config that flattens CommonConfig fields
#[derive(Debug, DeriveConfig)]
struct AppConfig {
    name: String,

    #[compote(flatten)]
    common: CommonConfig,
}

/// Config with multiple flattened fields
#[derive(Debug, DeriveConfig)]
struct DatabaseConfig {
    host: String,
    port: i32,
}

#[derive(Debug, DeriveConfig)]
struct CacheConfig {
    enabled: bool,
    ttl: i32,
}

#[derive(Debug, DeriveConfig)]
struct FullConfig {
    app_name: String,

    #[compote(flatten)]
    db: DatabaseConfig,

    #[compote(flatten)]
    cache: CacheConfig,
}

#[test]
fn test_basic_flatten() {
    // Fields from CommonConfig appear at same level as AppConfig
    let json = r#"{
        "name": "my-app",
        "debug": true,
        "verbose": false
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Flatten deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "my-app");
    assert!(cfg.common.debug);
    assert!(!cfg.common.verbose);
}

#[test]
fn test_flatten_with_defaults() {
    // verbose has a default, so it can be omitted
    let json = r#"{
        "name": "my-app",
        "debug": false
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Flatten with defaults should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "my-app");
    assert!(!cfg.common.debug);
    assert!(!cfg.common.verbose); // Default value
}

#[test]
fn test_multiple_flattened_fields() {
    // Multiple flattened structs
    let json = r#"{
        "app_name": "my-service",
        "host": "localhost",
        "port": 5432,
        "enabled": true,
        "ttl": 3600
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FullConfig>();
    assert!(
        result.is_ok(),
        "Multiple flatten should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.app_name, "my-service");
    assert_eq!(cfg.db.host, "localhost");
    assert_eq!(cfg.db.port, 5432);
    assert!(cfg.cache.enabled);
    assert_eq!(cfg.cache.ttl, 3600);
}

#[test]
fn test_flatten_missing_required_field() {
    // Missing required field from flattened struct should fail
    let json = r#"{
        "name": "my-app"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_err(),
        "Missing required field in flattened struct should fail"
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_flatten_yaml() {
    // Test with YAML format
    let yaml = r#"
name: yaml-app
debug: true
verbose: true
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "YAML flatten should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "yaml-app");
    assert!(cfg.common.debug);
    assert!(cfg.common.verbose);
}

#[test]
fn test_flatten_preserves_non_flattened_key() {
    // Ensure that keys that belong to the parent struct are not passed to flattened struct
    let json = r#"{
        "name": "test-app",
        "debug": false,
        "verbose": true
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AppConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize correctly: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    // name should be in parent, debug/verbose in flattened CommonConfig
    assert_eq!(cfg.name, "test-app");
    assert!(!cfg.common.debug);
    assert!(cfg.common.verbose);
}

// ============================================================================
// Tests for flattened internally-tagged enums (PromptConfig/PromptType pattern)
// ============================================================================

/// Internally-tagged enum that reads "type" field and variant-specific sibling fields
#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(tag = "type")]
enum PromptType {
    /// Select prompt with choices
    #[compote(rename = "select")]
    Select {
        choices: Vec<String>,
        #[compote(default)]
        multi: bool,
    },

    /// Input prompt (no extra fields)
    #[compote(rename = "input")]
    Input,

    /// Confirm prompt with optional default
    #[compote(rename = "confirm")]
    Confirm {
        #[compote(default)]
        default_value: bool,
    },

    /// Number prompt with min/max range
    #[compote(rename = "number")]
    Number {
        #[compote(default = "0")]
        min: i32,
        #[compote(default = "100")]
        max: i32,
    },
}

/// Parent struct that flattens the enum - the enum reads sibling fields
#[derive(Debug, DeriveConfig, PartialEq)]
struct PromptConfig {
    id: String,
    prompt: String,

    #[compote(flatten)]
    prompt_type: PromptType,
}

#[test]
fn test_flatten_enum_select_variant() {
    // The "type" and "choices" are sibling fields at the same level as "id" and "prompt"
    let json = r#"{
        "id": "env_choice",
        "prompt": "Select environment",
        "type": "select",
        "choices": ["dev", "staging", "prod"]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "env_choice");
    assert_eq!(cfg.prompt, "Select environment");
    assert_eq!(
        cfg.prompt_type,
        PromptType::Select {
            choices: vec!["dev".to_string(), "staging".to_string(), "prod".to_string()],
            multi: false,
        }
    );
}

#[test]
fn test_flatten_enum_select_with_multi() {
    let json = r#"{
        "id": "tags",
        "prompt": "Select tags",
        "type": "select",
        "choices": ["rust", "python", "go"],
        "multi": true
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum with multi should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "tags");
    assert_eq!(
        cfg.prompt_type,
        PromptType::Select {
            choices: vec!["rust".to_string(), "python".to_string(), "go".to_string()],
            multi: true,
        }
    );
}

#[test]
fn test_flatten_enum_input_variant() {
    // Input variant has no extra fields
    let json = r#"{
        "id": "username",
        "prompt": "Enter your username",
        "type": "input"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum input variant should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "username");
    assert_eq!(cfg.prompt, "Enter your username");
    assert_eq!(cfg.prompt_type, PromptType::Input);
}

#[test]
fn test_flatten_enum_confirm_variant() {
    let json = r#"{
        "id": "proceed",
        "prompt": "Continue with deployment?",
        "type": "confirm",
        "default_value": true
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum confirm variant should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "proceed");
    assert_eq!(
        cfg.prompt_type,
        PromptType::Confirm {
            default_value: true
        }
    );
}

#[test]
fn test_flatten_enum_number_variant_with_range() {
    let json = r#"{
        "id": "age",
        "prompt": "Enter your age",
        "type": "number",
        "min": 18,
        "max": 120
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum number variant should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "age");
    assert_eq!(cfg.prompt_type, PromptType::Number { min: 18, max: 120 });
}

#[test]
fn test_flatten_enum_number_variant_defaults() {
    // min and max have defaults, so they can be omitted
    let json = r#"{
        "id": "count",
        "prompt": "Enter count",
        "type": "number"
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Flatten enum with defaults should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(
        cfg.prompt_type,
        PromptType::Number { min: 0, max: 100 } // Default values
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_flatten_enum_yaml() {
    // Test the pattern with YAML (more realistic for omni configs)
    let yaml = r#"
id: env_select
prompt: "Choose environment"
type: select
choices:
  - development
  - staging
  - production
multi: false
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "YAML flatten enum should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.id, "env_select");
    assert_eq!(cfg.prompt, "Choose environment");
    assert_eq!(
        cfg.prompt_type,
        PromptType::Select {
            choices: vec![
                "development".to_string(),
                "staging".to_string(),
                "production".to_string()
            ],
            multi: false,
        }
    );
}

/// Test serialization of flattened enum
/// Flattened fields should merge their keys into the parent object
#[test]
fn test_flatten_enum_serialization() {
    let config = PromptConfig {
        id: "test".to_string(),
        prompt: "Test prompt".to_string(),
        prompt_type: PromptType::Select {
            choices: vec!["a".to_string(), "b".to_string()],
            multi: true,
        },
    };

    let serialized = serde_json::to_value(&config).expect("Should serialize");

    // Parent fields are at top level
    assert_eq!(serialized["id"], "test");
    assert_eq!(serialized["prompt"], "Test prompt");

    // Flattened fields should appear at the same level (not nested)
    // The enum's tag and variant fields merge into the parent
    assert_eq!(serialized["type"], "select");
    assert_eq!(serialized["choices"], serde_json::json!(["a", "b"]));
    assert_eq!(serialized["multi"], true);

    // There should be no nested prompt_type object
    assert!(serialized.get("prompt_type").is_none());
}

#[test]
fn test_flatten_enum_serialization_unit_variant() {
    let config = PromptConfig {
        id: "name".to_string(),
        prompt: "Enter name".to_string(),
        prompt_type: PromptType::Input,
    };

    let serialized = serde_json::to_value(&config).expect("Should serialize");

    assert_eq!(serialized["id"], "name");
    assert_eq!(serialized["prompt"], "Enter name");

    // Flattened unit variant - just the tag at the same level
    assert_eq!(serialized["type"], "input");

    // There should be no nested prompt_type object
    assert!(serialized.get("prompt_type").is_none());
}

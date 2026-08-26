//! Tagged Enum Pattern: Internally tagged enums
//!
//! Common use cases:
//! - Different operation types (set, append, unset)
//! - Plugin configurations (npm, cargo, pip)
//! - Action variants in workflows
//! - Message types in event systems
//!
//! Feuilletage Solution: `#[feuilletage(tag = "field")]` at the enum level
//!
//! Example:
//! ```yaml
//! # Discriminated by "type" field
//! type: "password"
//! min_length: 16
//! ```

use feuilletage::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Example: Prompt types with different configurations
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "type")]
enum PromptType {
    #[feuilletage(rename = "text")]
    Text,

    #[feuilletage(rename = "password")]
    Password {
        #[feuilletage(default = "8")]
        min_length: i32,
    },

    #[feuilletage(rename = "choice")]
    Choice { choices: Vec<String> },

    #[feuilletage(rename = "number")]
    Number { min: Option<i64>, max: Option<i64> },
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Tagged Enum Examples ===\n");

    // Unit variant
    println!("--- Unit Variant ---");
    let json = r#"{"type": "text"}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize unit variant");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(prompt, PromptType::Text);

    // Struct variant with defaults
    println!("\n--- Struct Variant with Defaults ---");
    let json = r#"{"type": "password"}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with defaults");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(prompt, PromptType::Password { min_length: 8 });

    // Struct variant with fields
    println!("\n--- Struct Variant with Fields ---");
    let json = r#"{"type": "password", "min_length": 16}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with fields");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(prompt, PromptType::Password { min_length: 16 });

    // Struct variant with Vec
    println!("\n--- Struct Variant with Vec ---");
    let json = r#"{"type": "choice", "choices": ["yes", "no", "maybe"]}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with Vec");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(
        prompt,
        PromptType::Choice {
            choices: vec!["yes".to_string(), "no".to_string(), "maybe".to_string()],
        }
    );

    // Struct variant with Options
    println!("\n--- Struct Variant with Options ---");
    let json = r#"{"type": "number", "min": 0, "max": 100}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with both options");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(
        prompt,
        PromptType::Number {
            min: Some(0),
            max: Some(100)
        }
    );

    let json = r#"{"type": "number", "min": 10}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with one option");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(
        prompt,
        PromptType::Number {
            min: Some(10),
            max: None
        }
    );

    let json = r#"{"type": "number"}"#;
    let prompt: PromptType = deserialize_json(json).expect("Should deserialize with no options");
    println!("Input: {}", json);
    println!("Result: {:?}", prompt);
    assert_eq!(
        prompt,
        PromptType::Number {
            min: None,
            max: None
        }
    );

    // Unknown tag error
    println!("\n--- Unknown Tag Error ---");
    let json = r#"{"type": "unknown_type"}"#;
    let result = deserialize_json::<PromptType>(json);
    match &result {
        Ok(_) => println!("Unexpected success"),
        Err(_) => println!("Error (expected): unknown tag"),
    }
    assert!(result.is_err(), "Should fail for unknown tag");

    // Missing tag error
    println!("\n--- Missing Tag Error ---");
    let json = r#"{"choices": ["a", "b"]}"#;
    let result = deserialize_json::<PromptType>(json);
    assert!(result.is_err(), "Should fail when tag is missing");
    println!("Missing tag error as expected");

    // Enum with aliases
    println!("\n--- Enum with Aliases ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(tag = "kind")]
    enum Status {
        #[feuilletage(rename = "active", alias = "enabled", alias = "on")]
        Active,

        #[feuilletage(rename = "inactive", alias = "disabled", alias = "off")]
        Inactive,

        #[feuilletage(rename = "pending", alias = "waiting")]
        Pending,
    }

    for (input, _expected_str) in [
        (r#"{"kind": "active"}"#, "Active"),
        (r#"{"kind": "enabled"}"#, "Active"),
        (r#"{"kind": "on"}"#, "Active"),
        (r#"{"kind": "off"}"#, "Inactive"),
        (r#"{"kind": "waiting"}"#, "Pending"),
    ] {
        let status: Status =
            deserialize_json(input).unwrap_or_else(|_| panic!("Should parse {}", input));
        println!("Input: {} -> {:?}", input, status);
    }

    // Enum with rename_all
    println!("\n--- Enum with rename_all ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(tag = "action", rename_all = "snake_case")]
    enum ActionType {
        DoSomething,
        DoSomethingElse,
        PerformAction { value: i32 },
    }

    let json = r#"{"action": "do_something"}"#;
    let action: ActionType = deserialize_json(json).expect("snake_case should work");
    println!("Input: {} -> {:?}", json, action);
    assert_eq!(action, ActionType::DoSomething);

    let json = r#"{"action": "do_something_else"}"#;
    let action: ActionType = deserialize_json(json).expect("snake_case should work");
    println!("Input: {} -> {:?}", json, action);
    assert_eq!(action, ActionType::DoSomethingElse);

    // Real-world: Environment operations
    println!("\n--- Real-World: Environment Operations ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(tag = "operation")]
    enum EnvOperation {
        #[feuilletage(rename = "set")]
        Set { key: String, value: String },

        #[feuilletage(rename = "append")]
        Append {
            key: String,
            value: String,
            #[feuilletage(default = ":")]
            separator: String,
        },

        #[feuilletage(rename = "prepend")]
        Prepend {
            key: String,
            value: String,
            #[feuilletage(default = ":")]
            separator: String,
        },

        #[feuilletage(rename = "unset")]
        Unset { key: String },
    }

    let json = r#"{"operation": "set", "key": "PATH", "value": "/usr/local/bin"}"#;
    let op: EnvOperation = deserialize_json(json).expect("Set operation");
    println!("Input: {}", json);
    println!("Result: {:?}", op);
    assert_eq!(
        op,
        EnvOperation::Set {
            key: "PATH".to_string(),
            value: "/usr/local/bin".to_string(),
        }
    );

    let json = r#"{"operation": "append", "key": "PATH", "value": "/opt/bin"}"#;
    let op: EnvOperation = deserialize_json(json).expect("Append with default separator");
    println!("Input: {}", json);
    println!("Result: {:?}", op);
    assert_eq!(
        op,
        EnvOperation::Append {
            key: "PATH".to_string(),
            value: "/opt/bin".to_string(),
            separator: ":".to_string(),
        }
    );

    let json =
        r#"{"operation": "append", "key": "CLASSPATH", "value": "lib.jar", "separator": ";"}"#;
    let op: EnvOperation = deserialize_json(json).expect("Append with custom separator");
    println!("Input: {}", json);
    println!("Result: {:?}", op);
    assert_eq!(
        op,
        EnvOperation::Append {
            key: "CLASSPATH".to_string(),
            value: "lib.jar".to_string(),
            separator: ";".to_string(),
        }
    );

    // Real-world: Plugin configuration
    println!("\n--- Real-World: Plugin Configuration ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(tag = "plugin_type")]
    enum PluginConfig {
        #[feuilletage(rename = "npm")]
        Npm {
            #[feuilletage(default)]
            packages: Vec<String>,
            #[feuilletage(default = "false")]
            global: bool,
        },

        #[feuilletage(rename = "cargo")]
        Cargo {
            #[feuilletage(default)]
            crates: Vec<String>,
            #[feuilletage(default = "false")]
            locked: bool,
        },

        #[feuilletage(rename = "pip")]
        Pip {
            #[feuilletage(default)]
            packages: Vec<String>,
            #[feuilletage(default = "false")]
            user: bool,
        },
    }

    let json = r#"{"plugin_type": "npm", "packages": ["typescript", "eslint"], "global": true}"#;
    let plugin: PluginConfig = deserialize_json(json).expect("NPM plugin");
    println!("Input: {}", json);
    println!("Result: {:?}", plugin);
    assert_eq!(
        plugin,
        PluginConfig::Npm {
            packages: vec!["typescript".to_string(), "eslint".to_string()],
            global: true,
        }
    );

    let json = r#"{"plugin_type": "cargo", "crates": ["ripgrep", "fd-find"]}"#;
    let plugin: PluginConfig = deserialize_json(json).expect("Cargo plugin");
    println!("Input: {}", json);
    println!("Result: {:?}", plugin);
    assert_eq!(
        plugin,
        PluginConfig::Cargo {
            crates: vec!["ripgrep".to_string(), "fd-find".to_string()],
            locked: false,
        }
    );

    // Nested enum
    println!("\n--- Nested Enum ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ConfigWithEnum {
        name: String,
        prompt: PromptType,
    }

    let json = r#"{
        "name": "user_prompt",
        "prompt": {"type": "choice", "choices": ["opt1", "opt2"]}
    }"#;
    let config: ConfigWithEnum = deserialize_json(json).expect("Nested enum");
    println!("Input: {}", json);
    println!("name: {}", config.name);
    println!("prompt: {:?}", config.prompt);
    assert_eq!(config.name, "user_prompt");
    assert_eq!(
        config.prompt,
        PromptType::Choice {
            choices: vec!["opt1".to_string(), "opt2".to_string()],
        }
    );

    // Serialization
    println!("\n--- Serialization ---");
    let prompt = PromptType::Text;
    let serialized = feuilletage::to_json_compact(&prompt).unwrap();
    println!("PromptType::Text serialized: {}", serialized);
    assert!(serialized.contains(r#""type":"text""#));

    let prompt = PromptType::Choice {
        choices: vec!["a".to_string(), "b".to_string()],
    };
    let serialized = feuilletage::to_json_compact(&prompt).unwrap();
    println!("PromptType::Choice serialized: {}", serialized);
    assert!(serialized.contains(r#""type":"choice""#));
    assert!(serialized.contains(r#""choices""#));

    println!("\n=== All tagged enum examples passed! ===");
}

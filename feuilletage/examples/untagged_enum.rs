//! Untagged Enum Pattern: Type-based variant selection
//!
//! Common use cases:
//! - Different types for the same field (string OR object)
//! - Shorthand vs full config notation
//! - Package specs (name only vs full dependency object)
//! - Commands (simple string vs structured with args)
//!
//! Feuilletage Solution: `#[feuilletage(untagged)]` at the enum level
//!
//! Example:
//! ```yaml
//! # Simple string variant
//! nix: "shell.nix"
//!
//! # Or detailed object variant
//! nix:
//!   file: "flake.nix"
//!   packages: ["ripgrep", "fd"]
//! ```

use feuilletage::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Simple untagged enum: string or detailed config
#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(untagged)]
enum NixConfig {
    /// Just a file path
    Simple(String),
    /// Full configuration
    Detailed {
        file: Option<String>,
        #[feuilletage(default)]
        packages: Vec<String>,
    },
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Untagged Enum Examples ===\n");

    // String variant
    println!("--- String Variant ---");
    let json = r#""shell.nix""#;
    let config: NixConfig = deserialize_json(json).expect("Should match string variant");
    println!("Input: {}", json);
    println!("Result: {:?}", config);
    assert_eq!(config, NixConfig::Simple("shell.nix".to_string()));

    // Object variant
    println!("\n--- Object Variant ---");
    let json = r#"{"file": "flake.nix", "packages": ["ripgrep"]}"#;
    let config: NixConfig = deserialize_json(json).expect("Should match object variant");
    println!("Input: {}", json);
    println!("Result: {:?}", config);
    assert_eq!(
        config,
        NixConfig::Detailed {
            file: Some("flake.nix".to_string()),
            packages: vec!["ripgrep".to_string()],
        }
    );

    // Partial object
    println!("\n--- Partial Object ---");
    let json = r#"{"packages": ["git", "curl"]}"#;
    let config: NixConfig = deserialize_json(json).expect("Should match partial object");
    println!("Input: {}", json);
    println!("Result: {:?}", config);
    assert_eq!(
        config,
        NixConfig::Detailed {
            file: None,
            packages: vec!["git".to_string(), "curl".to_string()],
        }
    );

    // Multiple primitive variants
    println!("\n--- Multiple Primitive Variants ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(untagged)]
    enum FlexibleValue {
        Text(String),
        Number(i64),
        Flag(bool),
    }

    let json = r#""hello world""#;
    let value: FlexibleValue = deserialize_json(json).expect("Should match text");
    println!("Input: {} -> {:?}", json, value);
    assert_eq!(value, FlexibleValue::Text("hello world".to_string()));

    let json = r#"42"#;
    let value: FlexibleValue = deserialize_json(json).expect("Should match number");
    println!("Input: {} -> {:?}", json, value);
    assert_eq!(value, FlexibleValue::Number(42));

    let json = r#"true"#;
    let value: FlexibleValue = deserialize_json(json).expect("Should match flag");
    println!("Input: {} -> {:?}", json, value);
    assert_eq!(value, FlexibleValue::Flag(true));

    // Package spec pattern
    println!("\n--- Package Spec Pattern ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(untagged)]
    enum PackageSpec {
        Name(String),
        Full {
            name: String,
            version: String,
            features: Vec<String>,
            #[feuilletage(default = "false")]
            optional: bool,
        },
        NameVersion {
            name: String,
            version: String,
        },
    }

    let json = r#""serde""#;
    let spec: PackageSpec = deserialize_json(json).expect("Name only");
    println!("Input: {} -> {:?}", json, spec);
    assert_eq!(spec, PackageSpec::Name("serde".to_string()));

    let json = r#"{"name": "serde", "version": "1.0"}"#;
    let spec: PackageSpec = deserialize_json(json).expect("Name and version");
    println!("Input: {} -> {:?}", json, spec);
    assert_eq!(
        spec,
        PackageSpec::NameVersion {
            name: "serde".to_string(),
            version: "1.0".to_string(),
        }
    );

    let json = r#"{"name": "serde", "version": "1.0", "features": ["derive"], "optional": true}"#;
    let spec: PackageSpec = deserialize_json(json).expect("Full spec");
    println!("Input: {} -> {:?}", json, spec);
    assert_eq!(
        spec,
        PackageSpec::Full {
            name: "serde".to_string(),
            version: "1.0".to_string(),
            features: vec!["derive".to_string()],
            optional: true,
        }
    );

    // Vec variants
    println!("\n--- Vec Variants ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(untagged)]
    enum ArrayOrSingle {
        Single(String),
        Multiple(Vec<String>),
    }

    let json = r#""single""#;
    let value: ArrayOrSingle = deserialize_json(json).expect("Single string");
    println!("Input: {} -> {:?}", json, value);
    assert_eq!(value, ArrayOrSingle::Single("single".to_string()));

    let json = r#"["a", "b", "c"]"#;
    let value: ArrayOrSingle = deserialize_json(json).expect("Multiple strings");
    println!("Input: {} -> {:?}", json, value);
    assert_eq!(
        value,
        ArrayOrSingle::Multiple(vec!["a".to_string(), "b".to_string(), "c".to_string(),])
    );

    // Nested untagged enums
    println!("\n--- Nested Untagged Enums ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ConfigWithUntagged {
        name: String,
        nix: NixConfig,
    }

    let json = r#"{"name": "project", "nix": "shell.nix"}"#;
    let config: ConfigWithUntagged = deserialize_json(json).expect("Nested untagged string");
    println!("Input: {}", json);
    println!("name: {}", config.name);
    println!("nix: {:?}", config.nix);
    assert_eq!(config.name, "project");
    assert_eq!(config.nix, NixConfig::Simple("shell.nix".to_string()));

    let json = r#"{"name": "project", "nix": {"packages": ["git"]}}"#;
    let config: ConfigWithUntagged = deserialize_json(json).expect("Nested untagged object");
    println!("Input: {}", json);
    println!("name: {}", config.name);
    println!("nix: {:?}", config.nix);
    assert_eq!(config.name, "project");
    assert_eq!(
        config.nix,
        NixConfig::Detailed {
            file: None,
            packages: vec!["git".to_string()],
        }
    );

    // Real-world: Command pattern
    println!("\n--- Real-World: Command Pattern ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(untagged)]
    enum Command {
        Simple(String),
        Structured {
            #[feuilletage(aliases = ["cmd"])]
            command: String,
            #[feuilletage(default)]
            args: Vec<String>,
            #[feuilletage(default)]
            env: Vec<String>,
            #[feuilletage(default = ".")]
            working_dir: String,
        },
    }

    let json = r#""npm install""#;
    let cmd: Command = deserialize_json(json).expect("Simple command");
    println!("Input: {} -> {:?}", json, cmd);
    assert_eq!(cmd, Command::Simple("npm install".to_string()));

    let json = r#"{
        "command": "cargo",
        "args": ["build", "--release"],
        "working_dir": "/project"
    }"#;
    let cmd: Command = deserialize_json(json).expect("Structured command");
    println!("Input: {}", json);
    println!("Result: {:?}", cmd);
    assert_eq!(
        cmd,
        Command::Structured {
            command: "cargo".to_string(),
            args: vec!["build".to_string(), "--release".to_string()],
            env: vec![],
            working_dir: "/project".to_string(),
        }
    );

    // Real-world: Path input
    println!("\n--- Real-World: Path Input ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    #[feuilletage(untagged)]
    enum PathInput {
        Path(String),
        PathWithOptions {
            path: String,
            #[feuilletage(default = "true")]
            recursive: bool,
            #[feuilletage(default)]
            exclude: Vec<String>,
        },
    }

    let json = r#""/usr/local/bin""#;
    let path: PathInput = deserialize_json(json).expect("Simple path");
    println!("Input: {} -> {:?}", json, path);
    assert_eq!(path, PathInput::Path("/usr/local/bin".to_string()));

    let json = r#"{"path": "/home/user", "recursive": false, "exclude": ["*.tmp"]}"#;
    let path: PathInput = deserialize_json(json).expect("Path with options");
    println!("Input: {}", json);
    println!("Result: {:?}", path);
    assert_eq!(
        path,
        PathInput::PathWithOptions {
            path: "/home/user".to_string(),
            recursive: false,
            exclude: vec!["*.tmp".to_string()],
        }
    );

    // Serialization
    println!("\n--- Serialization ---");
    let config = NixConfig::Simple("shell.nix".to_string());
    let serialized = feuilletage::to_json_compact(&config).unwrap();
    println!("NixConfig::Simple serialized: {}", serialized);
    assert_eq!(serialized, r#""shell.nix""#);

    let config = NixConfig::Detailed {
        file: Some("flake.nix".to_string()),
        packages: vec!["git".to_string()],
    };
    let serialized = feuilletage::to_json_compact(&config).unwrap();
    println!("NixConfig::Detailed serialized: {}", serialized);
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    assert!(parsed.is_object());
    assert_eq!(parsed["file"], "flake.nix");

    // Vec of untagged enums
    println!("\n--- Vec of Untagged Enums ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct MultiCommandConfig {
        #[feuilletage(default)]
        commands: Vec<Command>,
    }

    let json = r#"{
        "commands": [
            "npm install",
            {"command": "cargo", "args": ["build"]}
        ]
    }"#;
    let config: MultiCommandConfig = deserialize_json(json).expect("Vec of untagged");
    println!("Input: {}", json);
    println!("commands.len: {}", config.commands.len());
    println!("commands[0]: {:?}", config.commands[0]);
    println!("commands[1]: {:?}", config.commands[1]);
    assert_eq!(config.commands.len(), 2);
    assert_eq!(
        config.commands[0],
        Command::Simple("npm install".to_string())
    );
    assert_eq!(
        config.commands[1],
        Command::Structured {
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
            env: vec![],
            working_dir: ".".to_string(),
        }
    );

    println!("\n=== All untagged enum examples passed! ===");
}

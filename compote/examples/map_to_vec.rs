//! Map to Vec Pattern: allow_map with scalar_as for compact notation
//!
//! This pattern is useful for configs that:
//! - Use map keys as item identifiers
//! - Allow scalar values as shorthand for a primary field
//! - Support object values for full configuration
//!
//! Example use case: Package lists where:
//! - `{ripgrep: "14.0.0"}` means package "ripgrep" version "14.0.0"
//! - `{fd: {version: "9.0", features: ["tls"]}}` provides full config
//!
//! Compote Solution: `#[compote(allow_map(key = "name", scalar_as = "version"))]`

use compote::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Represents a single package specification
#[derive(Debug, compote::Config, PartialEq, Clone)]
struct PackageSpec {
    name: String,
    version: String,
    #[compote(default)]
    features: Vec<String>,
    #[compote(default = "false")]
    locked: bool,
}

/// Container for multiple packages using allow_map
#[derive(Debug, compote::Config, PartialEq)]
struct PackageListConfig {
    #[compote(allow_map(key = "name", scalar_as = "version"))]
    packages: Vec<PackageSpec>,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Map to Vec Examples ===\n");

    // Map with scalar version
    println!("--- Map with Scalar Version ---");
    let json = r#"{"packages": {"ripgrep": "14.0.0"}}"#;
    let config: PackageListConfig =
        deserialize_json(json).expect("Should deserialize map with scalar");
    println!("Input: {}", json);
    println!("packages[0].name: {}", config.packages[0].name);
    println!("packages[0].version: {}", config.packages[0].version);
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "ripgrep");
    assert_eq!(config.packages[0].version, "14.0.0");
    assert!(config.packages[0].features.is_empty());
    assert!(!config.packages[0].locked);

    // Map with object version
    println!("\n--- Map with Object Version ---");
    let json = r#"{"packages": {"fd": {"version": "9.0", "features": ["tls"], "locked": true}}}"#;
    let config: PackageListConfig =
        deserialize_json(json).expect("Should deserialize map with object");
    println!("Input: {}", json);
    println!("packages[0].name: {}", config.packages[0].name);
    println!("packages[0].version: {}", config.packages[0].version);
    println!("packages[0].features: {:?}", config.packages[0].features);
    println!("packages[0].locked: {}", config.packages[0].locked);
    assert_eq!(config.packages.len(), 1);
    assert_eq!(config.packages[0].name, "fd");
    assert_eq!(config.packages[0].version, "9.0");
    assert_eq!(config.packages[0].features, vec!["tls".to_string()]);
    assert!(config.packages[0].locked);

    // Map with mixed formats
    println!("\n--- Map with Mixed Formats ---");
    let json = r#"{"packages": {
        "ripgrep": "14.0.0",
        "fd": {"version": "9.0", "features": ["regex"]},
        "bat": "0.24.0"
    }}"#;
    let config: PackageListConfig = deserialize_json(json).expect("Should deserialize mixed map");
    println!("Input: {}", json);
    println!("packages.len: {}", config.packages.len());
    assert_eq!(config.packages.len(), 3);

    let ripgrep = config
        .packages
        .iter()
        .find(|c| c.name == "ripgrep")
        .unwrap();
    println!(
        "ripgrep: version={}, features={:?}",
        ripgrep.version, ripgrep.features
    );
    assert_eq!(ripgrep.version, "14.0.0");
    assert!(ripgrep.features.is_empty());

    let fd = config.packages.iter().find(|c| c.name == "fd").unwrap();
    println!("fd: version={}, features={:?}", fd.version, fd.features);
    assert_eq!(fd.version, "9.0");
    assert_eq!(fd.features, vec!["regex".to_string()]);

    let bat = config.packages.iter().find(|c| c.name == "bat").unwrap();
    println!("bat: version={}, features={:?}", bat.version, bat.features);
    assert_eq!(bat.version, "0.24.0");

    // Empty map
    println!("\n--- Empty Map ---");
    let json = r#"{"packages": {}}"#;
    let config: PackageListConfig = deserialize_json(json).expect("Should deserialize empty map");
    println!("Input: {}", json);
    println!("packages.len: {}", config.packages.len());
    assert!(config.packages.is_empty());

    // Array input (allow_map also accepts arrays)
    println!("\n--- Array Input ---");
    let json = r#"{"packages": [
        {"name": "ripgrep", "version": "14.0.0"},
        {"name": "fd", "version": "9.0", "features": ["regex"]}
    ]}"#;
    let config: PackageListConfig = deserialize_json(json).expect("Should deserialize array");
    println!("Input: {}", json);
    println!("packages.len: {}", config.packages.len());
    assert_eq!(config.packages.len(), 2);
    assert_eq!(config.packages[0].name, "ripgrep");
    assert_eq!(config.packages[1].name, "fd");

    // Serialization - compact when default
    println!("\n--- Serialization: Compact When Default ---");
    let json = r#"{"packages": {"curl": "8.0.0"}}"#;
    let config: PackageListConfig = deserialize_json(json).unwrap();
    let serialized = compote::to_json_compact(&config).unwrap();
    println!("Input: {}", json);
    println!("Serialized: {}", serialized);
    assert_eq!(serialized, r#"{"packages":{"curl":"8.0.0"}}"#);

    // Serialization - full when non-default
    println!("\n--- Serialization: Full When Non-default ---");
    let json = r#"{"packages": {"wget": {"version": "2.0", "features": ["ssl"], "locked": true}}}"#;
    let config: PackageListConfig = deserialize_json(json).unwrap();
    let serialized = compote::to_json_compact(&config).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
    println!("Input: {}", json);
    println!("Serialized: {}", serialized);
    let wget = &parsed["packages"]["wget"];
    assert!(
        wget.is_object(),
        "Should be object when non-default fields present"
    );
    assert_eq!(wget["version"], "2.0");
    assert_eq!(wget["features"][0], "ssl");
    assert_eq!(wget["locked"], true);

    // Boolean Scalar Pattern
    println!("\n--- Boolean Scalar Pattern ---");

    #[derive(Debug, compote::Config, PartialEq, Clone)]
    struct FeatureSpec {
        name: String,
        #[compote(default = "true")]
        enabled: bool,
        #[compote(default)]
        options: Option<String>,
    }

    #[derive(Debug, compote::Config, PartialEq)]
    struct FeatureConfig {
        #[compote(allow_map(key = "name", scalar_as = "enabled"))]
        features: Vec<FeatureSpec>,
    }

    let json = r#"{"features": {
        "logging": true,
        "debug": false,
        "metrics": true
    }}"#;
    let config: FeatureConfig = deserialize_json(json).expect("Should deserialize bool scalar map");
    println!("Input: {}", json);

    let logging = config
        .features
        .iter()
        .find(|p| p.name == "logging")
        .unwrap();
    println!("logging: enabled={}", logging.enabled);
    assert!(logging.enabled);

    let debug = config.features.iter().find(|p| p.name == "debug").unwrap();
    println!("debug: enabled={}", debug.enabled);
    assert!(!debug.enabled);

    let metrics = config
        .features
        .iter()
        .find(|p| p.name == "metrics")
        .unwrap();
    println!("metrics: enabled={}", metrics.enabled);
    assert!(metrics.enabled);

    // Dependency-style Pattern
    println!("\n--- Dependency-style Pattern ---");

    #[derive(Debug, compote::Config, PartialEq, Clone)]
    struct DependencySpec {
        name: String,
        version: String,
        #[compote(default = "false")]
        dev: bool,
        #[compote(default = "false")]
        optional: bool,
    }

    #[derive(Debug, compote::Config, PartialEq)]
    struct DependencyConfig {
        #[compote(allow_map(key = "name", scalar_as = "version"))]
        dependencies: Vec<DependencySpec>,
    }

    let json = r#"{"dependencies": {
        "serde": "^1.0.0",
        "tokio": "~1.0.0",
        "log": {"version": "0.4.0", "dev": true}
    }}"#;
    let config: DependencyConfig =
        deserialize_json(json).expect("Should deserialize dependency style");
    println!("Input: {}", json);
    assert_eq!(config.dependencies.len(), 3);

    let serde = config
        .dependencies
        .iter()
        .find(|p| p.name == "serde")
        .unwrap();
    println!("serde: version={}, dev={}", serde.version, serde.dev);
    assert_eq!(serde.version, "^1.0.0");
    assert!(!serde.dev);

    let log = config
        .dependencies
        .iter()
        .find(|p| p.name == "log")
        .unwrap();
    println!("log: version={}, dev={}", log.version, log.dev);
    assert_eq!(log.version, "0.4.0");
    assert!(log.dev);

    println!("\n=== All map to vec examples passed! ===");
}

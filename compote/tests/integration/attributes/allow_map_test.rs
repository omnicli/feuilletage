//! allow_map tests - map key injection
//!
//! Tests for the allow_map attribute which converts map input to Vec
//! by injecting map keys into a specified field.
//!
//! Also includes serialization tests for allow_map.

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

/// Test struct for allow_map with object values
#[derive(Debug, DeriveConfig, PartialEq)]
struct PackageEntry {
    name: String,
    #[compote(default = "false")]
    install: bool,
}

#[derive(Debug, DeriveConfig)]
struct PackageConfig {
    #[compote(allow_map = "name")]
    packages: Vec<PackageEntry>,
}

/// Test struct for allow_map with value field for scalar values
#[derive(Debug, DeriveConfig, PartialEq)]
struct FeatureEntry {
    name: String,
    enabled: bool,
}

#[derive(Debug, DeriveConfig)]
struct FeatureConfig {
    #[compote(allow_map(key = "name", scalar_as = "enabled"))]
    features: Vec<FeatureEntry>,
}

/// Test struct for allow_map with scalar values that become objects
#[derive(Debug, DeriveConfig, PartialEq)]
struct DependencyEntry {
    package: String,
    version: String,
}

#[derive(Debug, DeriveConfig)]
struct DependencyConfig {
    #[compote(allow_map(key = "package", scalar_as = "version"))]
    dependencies: Vec<DependencyEntry>,
}

#[test]
fn test_allow_map_injects_key_into_object() {
    let json = r#"{
        "packages": {
            "curl": { "install": true },
            "wget": { "install": false }
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PackageConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.packages.len(), 2);

    let curl = cfg.packages.iter().find(|p| p.name == "curl");
    assert!(curl.is_some(), "curl package should exist");
    assert!(curl.unwrap().install);

    let wget = cfg.packages.iter().find(|p| p.name == "wget");
    assert!(wget.is_some(), "wget package should exist");
    assert!(!wget.unwrap().install);
}

#[test]
fn test_allow_map_with_array_input() {
    let json = r#"{
        "packages": [
            { "name": "curl", "install": true },
            { "name": "wget", "install": false }
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PackageConfig>();
    assert!(
        result.is_ok(),
        "Array input should work: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.packages.len(), 2);
    assert_eq!(cfg.packages[0].name, "curl");
    assert!(cfg.packages[0].install);
}

#[test]
fn test_allow_map_with_scalar_value_and_scalar_as_field() {
    let json = r#"{
        "features": {
            "dark_mode": true,
            "notifications": false,
            "auto_save": true
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FeatureConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.features.len(), 3);

    let dark_mode = cfg.features.iter().find(|f| f.name == "dark_mode");
    assert!(dark_mode.is_some(), "dark_mode feature should exist");
    assert!(dark_mode.unwrap().enabled);

    let notifications = cfg.features.iter().find(|f| f.name == "notifications");
    assert!(
        notifications.is_some(),
        "notifications feature should exist"
    );
    assert!(!notifications.unwrap().enabled);
}

#[test]
fn test_allow_map_with_string_scalar_values() {
    let json = r#"{
        "dependencies": {
            "serde": "1.0",
            "tokio": "2.0",
            "reqwest": "0.11"
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DependencyConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.dependencies.len(), 3);

    let serde = cfg.dependencies.iter().find(|d| d.package == "serde");
    assert!(serde.is_some(), "serde dependency should exist");
    assert_eq!(serde.unwrap().version, "1.0");
}

#[test]
fn test_allow_map_empty_input() {
    let json = r#"{ "packages": {} }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PackageConfig>();
    assert!(result.is_ok(), "Empty map should succeed");

    let cfg = result.unwrap();
    assert_eq!(cfg.packages.len(), 0);
}

#[test]
fn test_allow_map_missing_field() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PackageConfig>();
    assert!(result.is_ok(), "Missing field should default to empty vec");

    let cfg = result.unwrap();
    assert_eq!(cfg.packages.len(), 0);
}

#[cfg(feature = "yaml")]
#[test]
fn test_allow_map_yaml_format() {
    let yaml = r#"
packages:
  curl:
    install: true
  wget:
    install: false
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PackageConfig>();
    assert!(
        result.is_ok(),
        "YAML deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.packages.len(), 2);

    let curl = cfg.packages.iter().find(|p| p.name == "curl");
    assert!(curl.is_some());
    assert!(curl.unwrap().install);
}

// ============================================================================
// Serialization tests for allow_map
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq, Clone)]
struct PackageSpec {
    name: String,
    version: String,
    #[compote(default)]
    features: Vec<String>,
    #[compote(default = "false")]
    locked: bool,
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct PackageListConfig {
    #[compote(allow_map(key = "name", scalar_as = "version"))]
    packages: Vec<PackageSpec>,
}

#[test]
fn test_allow_map_serialization_compact() {
    // Packages with only default values for other fields should serialize compactly
    let json = r#"{"packages": {"curl": "8.0.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let cfg: PackageListConfig = config.deserialize().expect("Should deserialize");

    let serialized = compote::to_json_compact(&cfg).unwrap();
    // Should serialize compactly as map with scalar values
    assert_eq!(serialized, r#"{"packages":{"curl":"8.0.0"}}"#);
}

#[test]
fn test_allow_map_serialization_full() {
    // Packages with non-default values should serialize as full objects
    let json = r#"{"packages": {"wget": {"version": "2.0", "features": ["ssl"], "locked": true}}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let cfg: PackageListConfig = config.deserialize().expect("Should deserialize");

    let serialized = compote::to_json_compact(&cfg).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

    let wget = &parsed["packages"]["wget"];
    assert!(
        wget.is_object(),
        "Should be object when non-default fields present"
    );
    assert_eq!(wget["version"], "2.0");
    assert_eq!(wget["features"][0], "ssl");
    assert_eq!(wget["locked"], true);
}

#[test]
fn test_allow_map_roundtrip() {
    // Test that we can deserialize and serialize back correctly
    let json = r#"{"packages": {
        "serde": "1.0",
        "tokio": {"version": "2.0", "features": ["rt", "macros"]}
    }}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let cfg: PackageListConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(cfg.packages.len(), 2);

    // Verify the data was correctly parsed
    let serde = cfg.packages.iter().find(|p| p.name == "serde").unwrap();
    assert_eq!(serde.version, "1.0");
    assert!(serde.features.is_empty());

    let tokio = cfg.packages.iter().find(|p| p.name == "tokio").unwrap();
    assert_eq!(tokio.version, "2.0");
    assert_eq!(tokio.features, vec!["rt".to_string(), "macros".to_string()]);
}

// ============================================================================
// Struct-level allow_map tests
// ============================================================================

/// Test struct with allow_map container attribute
/// Allows the struct to be created from a single-key map where:
/// - The map key becomes the `repository` field value
/// - A scalar map value becomes the `version` field value
/// - An object map value provides other fields
#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(allow_map(key = repository, scalar_as = version), scalar_as = "repository")]
struct GithubRelease {
    #[compote(alias = "repo")]
    repository: String,
    version: Option<String>,
}

#[test]
fn test_struct_allow_map_single_key_scalar() {
    // Input: {"owner/repo": "1.0.0"} should become {repository: "owner/repo", version: "1.0.0"}
    let json = r#"{"owner/repo": "1.0.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, Some("1.0.0".to_string()));
}

#[test]
fn test_struct_allow_map_single_key_object() {
    // Input: {"owner/repo": {"version": "2.0"}} should set repository from key
    let json = r#"{"owner/repo": {"version": "2.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, Some("2.0".to_string()));
}

#[test]
fn test_struct_allow_map_explicit_fields() {
    // Input with explicit repository field should work normally (no transformation)
    let json = r#"{"repository": "owner/repo", "version": "3.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, Some("3.0".to_string()));
}

#[test]
fn test_struct_allow_map_with_alias() {
    // Input using the alias should also work (and NOT trigger allow_map transformation)
    let json = r#"{"repo": "owner/repo", "version": "4.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, Some("4.0".to_string()));
}

#[test]
fn test_struct_allow_map_scalar_as_value() {
    // Using the struct-level scalar_as: a plain string becomes {repository: value}
    let json = r#""owner/repo""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, None);
}

#[test]
fn test_struct_allow_map_single_key_null() {
    // Input: {"owner/repo": null} should become {repository: "owner/repo", version: None}
    let json = r#"{"owner/repo": null}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubRelease>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let release = result.unwrap();
    assert_eq!(release.repository, "owner/repo");
    assert_eq!(release.version, None);
}

// ============================================================================
// Vec with parameterless allow_map (uses AllowMapKeys trait)
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct GithubReleasesConfig {
    // Parameterless allow_map uses the inner type's AllowMapKeys trait
    #[compote(allow_map)]
    releases: Vec<GithubRelease>,
}

// ============================================================================
// Vec with explicit allow_map(key = ...) also uses AllowMapKeys for detection
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
struct GithubReleasesExplicitKeyConfig {
    // Explicit key form also uses AllowMapKeys for single-item detection
    #[compote(allow_map(key = "repository", scalar_as = "version"))]
    releases: Vec<GithubRelease>,
}

#[test]
fn test_vec_allow_map_trait_single_item() {
    // A single-key map where the key doesn't match a known field should be treated as a single item
    let json = r#"{"releases": {"owner/repo": "1.0.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 1);
    assert_eq!(cfg.releases[0].repository, "owner/repo");
    assert_eq!(cfg.releases[0].version, Some("1.0.0".to_string()));
}

#[test]
fn test_vec_allow_map_trait_multiple_items() {
    // Multiple keys should become multiple items
    let json = r#"{"releases": {"repo1": "1.0", "repo2": "2.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 2);

    let repo1 = cfg.releases.iter().find(|r| r.repository == "repo1");
    assert!(repo1.is_some());
    assert_eq!(repo1.unwrap().version, Some("1.0".to_string()));
}

#[test]
fn test_vec_allow_map_trait_explicit_field_is_single() {
    // When the map contains a known field name (repository or alias repo), it's a single item
    let json = r#"{"releases": {"repository": "owner/repo", "version": "1.5"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 1);
    assert_eq!(cfg.releases[0].repository, "owner/repo");
    assert_eq!(cfg.releases[0].version, Some("1.5".to_string()));
}

#[test]
fn test_vec_allow_map_trait_alias_is_single() {
    // Using the alias "repo" should also detect single item
    let json = r#"{"releases": {"repo": "owner/repo"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 1);
    assert_eq!(cfg.releases[0].repository, "owner/repo");
}

#[test]
fn test_vec_allow_map_trait_array_input() {
    // Array input should work normally
    let json = r#"{"releases": [{"repository": "r1"}, {"repository": "r2"}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 2);
    assert_eq!(cfg.releases[0].repository, "r1");
    assert_eq!(cfg.releases[1].repository, "r2");
}

// ============================================================================
// Tests for explicit key form with AllowMapKeys detection
// ============================================================================

#[test]
fn test_vec_allow_map_explicit_key_detects_single_item() {
    // Object with "repository" key should be detected as single item via AllowMapKeys
    let json = r#"{"releases": {"repository": "owner/repo", "version": "1.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesExplicitKeyConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 1);
    assert_eq!(cfg.releases[0].repository, "owner/repo");
    assert_eq!(cfg.releases[0].version, Some("1.0".to_string()));
}

#[test]
fn test_vec_allow_map_explicit_key_detects_alias_as_single_item() {
    // Object with "repo" alias should ALSO be detected as single item via AllowMapKeys
    let json = r#"{"releases": {"repo": "owner/repo", "version": "2.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesExplicitKeyConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 1);
    assert_eq!(cfg.releases[0].repository, "owner/repo");
    assert_eq!(cfg.releases[0].version, Some("2.0".to_string()));
}

#[test]
fn test_vec_allow_map_explicit_key_map_notation() {
    // Object without key field should be treated as map notation
    let json = r#"{"releases": {"owner/repo1": "1.0", "owner/repo2": "2.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesExplicitKeyConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 2);

    let r1 = cfg.releases.iter().find(|r| r.repository == "owner/repo1");
    assert!(r1.is_some());
    assert_eq!(r1.unwrap().version, Some("1.0".to_string()));

    let r2 = cfg.releases.iter().find(|r| r.repository == "owner/repo2");
    assert!(r2.is_some());
    assert_eq!(r2.unwrap().version, Some("2.0".to_string()));
}

#[test]
fn test_vec_allow_map_explicit_key_array_passthrough() {
    // Array input should pass through unchanged
    let json = r#"{"releases": [{"repository": "r1"}, {"repository": "r2"}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<GithubReleasesExplicitKeyConfig>();
    assert!(
        result.is_ok(),
        "Deserialization should succeed: {:?}",
        result.err()
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.releases.len(), 2);
    assert_eq!(cfg.releases[0].repository, "r1");
    assert_eq!(cfg.releases[1].repository, "r2");
}

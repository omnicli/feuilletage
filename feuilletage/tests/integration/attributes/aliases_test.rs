//! Field aliases tests

#![cfg(feature = "json")]

use feuilletage::{Config, Context, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

#[derive(Debug, DeriveConfig)]
struct BasicAliasConfig {
    #[feuilletage(aliases = ["repo"])]
    repository: String,
}

#[derive(Debug, DeriveConfig)]
struct MultiAliasConfig {
    #[feuilletage(aliases = ["repo", "r", "source"])]
    repository: String,
}

#[derive(Debug, DeriveConfig)]
struct SingularAndPluralAliasConfig {
    #[feuilletage(alias = "repo", aliases = ["r", "source"])]
    repository: String,
}

#[derive(Debug, DeriveConfig)]
struct AliasWithDefaultConfig {
    #[feuilletage(aliases = ["repo"], default = "default-repo")]
    repository: String,
}

#[derive(Debug, DeriveConfig)]
struct AliasWithValidationConfig {
    #[feuilletage(aliases = ["num", "n"], range(1, 100))]
    number: i32,
}

#[derive(Debug, DeriveConfig)]
struct VecAliasConfig {
    #[feuilletage(aliases = ["pkgs", "packages"], allow_single)]
    package: Vec<String>,
}

#[derive(Debug, DeriveConfig)]
struct MultiFieldAliasConfig {
    #[feuilletage(aliases = ["repo"])]
    repository: String,
    #[feuilletage(aliases = ["ver", "v"])]
    version: String,
}

#[test]
fn test_alias_basic_usage() {
    // Use the alias instead of the primary name
    let json = r#"{"repo": "my-repository"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicAliasConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize using alias: {:?}",
        result
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.repository, "my-repository");
}

#[test]
fn test_alias_primary_name_still_works() {
    // Primary name should still work
    let json = r#"{"repository": "my-repository"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicAliasConfig>();
    assert!(result.is_ok(), "Should deserialize using primary name");

    let cfg = result.unwrap();
    assert_eq!(cfg.repository, "my-repository");
}

#[test]
fn test_alias_primary_takes_precedence() {
    // When both primary and alias are present, primary wins
    let json = r#"{"repository": "primary-value", "repo": "alias-value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicAliasConfig>();
    assert!(result.is_ok(), "Should deserialize with both keys present");

    let cfg = result.unwrap();
    assert_eq!(
        cfg.repository, "primary-value",
        "Primary name should take precedence over alias"
    );
}

#[test]
fn test_alias_multiple_aliases() {
    // Test multiple aliases - each should work
    for (key, expected) in [
        ("repository", "v1"),
        ("repo", "v2"),
        ("r", "v3"),
        ("source", "v4"),
    ] {
        let json = format!(r#"{{"{key}": "{expected}"}}"#);

        let mut config = Config::default();
        config.load_json(&json, Context::new(Source::Programmatic, Level::User));

        let result = config.deserialize::<MultiAliasConfig>();
        assert!(
            result.is_ok(),
            "Should deserialize using key '{}': {:?}",
            key,
            result
        );

        let cfg = result.unwrap();
        assert_eq!(
            cfg.repository, expected,
            "Value should be '{}' when using key '{}'",
            expected, key
        );
    }
}

#[test]
fn test_singular_and_plural_aliases_accumulate() {
    for key in ["repo", "r", "source"] {
        let json = format!(r#"{{"{key}": "my-repository"}}"#);
        let mut config = Config::default();
        config.load_json(&json, Context::new(Source::Programmatic, Level::User));

        let result: SingularAndPluralAliasConfig = config.deserialize().unwrap();
        assert_eq!(result.repository, "my-repository");
    }
}

#[test]
fn test_alias_first_match_wins() {
    // When multiple aliases are present, earlier ones win (primary > first alias > second alias)
    let json = r#"{"repo": "alias1-value", "r": "alias2-value", "source": "alias3-value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<MultiAliasConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(
        cfg.repository, "alias1-value",
        "First alias in order should win"
    );
}

#[test]
fn test_alias_with_default() {
    // Missing both primary and alias should use default
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AliasWithDefaultConfig>();
    assert!(result.is_ok(), "Should use default when field is missing");

    let cfg = result.unwrap();
    assert_eq!(cfg.repository, "default-repo");
}

#[test]
fn test_alias_with_default_overridden_by_alias() {
    // Alias should override default
    let json = r#"{"repo": "alias-override"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AliasWithDefaultConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(
        cfg.repository, "alias-override",
        "Alias should override default"
    );
}

#[test]
fn test_alias_with_validation() {
    // Valid value using alias
    let json = r#"{"num": 50}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AliasWithValidationConfig>();
    assert!(result.is_ok(), "Valid value should pass validation");

    let cfg = result.unwrap();
    assert_eq!(cfg.number, 50);
}

#[test]
fn test_alias_with_validation_fails() {
    // Invalid value using alias should still fail validation
    let json = r#"{"n": 200}"#; // Out of range

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AliasWithValidationConfig>();
    assert!(
        result.is_err(),
        "Out of range value should fail validation even when using alias"
    );
}

#[test]
fn test_alias_with_vec_allow_single() {
    // Single value using alias
    let json = r#"{"pkgs": "single-package"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<VecAliasConfig>();
    assert!(
        result.is_ok(),
        "Should handle alias with allow_single: {:?}",
        result
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.package, vec!["single-package"]);
}

#[test]
fn test_alias_with_vec_array() {
    // Array using alias
    let json = r#"{"packages": ["pkg1", "pkg2", "pkg3"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<VecAliasConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.package, vec!["pkg1", "pkg2", "pkg3"]);
}

#[test]
fn test_alias_multiple_fields() {
    // Multiple fields with aliases
    let json = r#"{"repo": "my-repo", "v": "1.0.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<MultiFieldAliasConfig>();
    assert!(result.is_ok());

    let cfg = result.unwrap();
    assert_eq!(cfg.repository, "my-repo");
    assert_eq!(cfg.version, "1.0.0");
}

#[cfg(feature = "yaml")]
#[test]
fn test_alias_yaml_format() {
    // Test aliases work in YAML
    let yaml = r#"
repo: yaml-repo
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicAliasConfig>();
    assert!(result.is_ok(), "Aliases should work in YAML: {:?}", result);

    let cfg = result.unwrap();
    assert_eq!(cfg.repository, "yaml-repo");
}

#[test]
fn test_alias_missing_required_field() {
    // Neither primary nor alias present - should fail for required field
    #[derive(Debug, DeriveConfig)]
    struct RequiredAliasConfig {
        #[feuilletage(aliases = ["r"])]
        required_field: String,
    }

    let json = r#"{"other_field": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RequiredAliasConfig>();
    assert!(result.is_err(), "Missing required field should fail");
}

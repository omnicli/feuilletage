// This entire test file requires JSON support since most tests use load_json
#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};

// ============================================================================
// Tagged Enum Tests
// ============================================================================

/// Test enum with internally tagged representation using the "type" field
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type")]
enum PromptType {
    #[compote(rename = "text")]
    Text,
    #[compote(rename = "choice")]
    Choice { choices: Vec<String> },
    #[compote(rename = "int")]
    Int { min: Option<i64>, max: Option<i64> },
}

#[test]
fn test_tagged_enum_unit_variant() {
    let json = r#"{"type": "text"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(
        result.is_ok(),
        "Should deserialize unit variant: {:?}",
        result
    );
    assert_eq!(result.unwrap(), PromptType::Text);
}

#[test]
fn test_tagged_enum_struct_variant_with_vec() {
    let json = r#"{
        "type": "choice",
        "choices": ["yes", "no", "maybe"]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(
        result.is_ok(),
        "Should deserialize struct variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        PromptType::Choice {
            choices: vec!["yes".to_string(), "no".to_string(), "maybe".to_string()]
        }
    );
}

#[test]
fn test_tagged_enum_struct_variant_with_options() {
    // With both min and max
    let json = r#"{
        "type": "int",
        "min": 0,
        "max": 100
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(
        result.is_ok(),
        "Should deserialize with both options: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        PromptType::Int {
            min: Some(0),
            max: Some(100)
        }
    );

    // With only min
    let json2 = r#"{
        "type": "int",
        "min": 10
    }"#;

    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));

    let result2 = config2.deserialize::<PromptType>();
    assert!(
        result2.is_ok(),
        "Should deserialize with partial options: {:?}",
        result2
    );
    assert_eq!(
        result2.unwrap(),
        PromptType::Int {
            min: Some(10),
            max: None
        }
    );

    // With neither
    let json3 = r#"{
        "type": "int"
    }"#;

    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));

    let result3 = config3.deserialize::<PromptType>();
    assert!(
        result3.is_ok(),
        "Should deserialize with no options: {:?}",
        result3
    );
    assert_eq!(
        result3.unwrap(),
        PromptType::Int {
            min: None,
            max: None
        }
    );
}

#[test]
fn test_tagged_enum_unknown_tag_error() {
    let json = r#"{"type": "unknown"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(result.is_err(), "Should fail for unknown tag");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("Unknown tag value") || err_str.contains("unknown"),
        "Error should mention unknown tag: {}",
        err_str
    );
}

#[test]
fn test_tagged_enum_missing_tag_error() {
    let json = r#"{"choices": ["a", "b"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(result.is_err(), "Should fail when tag is missing");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("Missing") || err_str.contains("type"),
        "Error should mention missing field: {}",
        err_str
    );
}

// ============================================================================
// Tagged Enum with Aliases
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "kind")]
enum Status {
    #[compote(rename = "active", alias = "enabled", alias = "on")]
    Active,
    #[compote(rename = "inactive", alias = "disabled", alias = "off")]
    Inactive,
}

#[test]
fn test_tagged_enum_with_aliases() {
    // Test primary rename
    let json1 = r#"{"kind": "active"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<Status>();
    assert_eq!(result1.unwrap(), Status::Active);

    // Test first alias
    let json2 = r#"{"kind": "enabled"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<Status>();
    assert_eq!(result2.unwrap(), Status::Active);

    // Test second alias
    let json3 = r#"{"kind": "on"}"#;
    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));
    let result3 = config3.deserialize::<Status>();
    assert_eq!(result3.unwrap(), Status::Active);

    // Test other variant alias
    let json4 = r#"{"kind": "off"}"#;
    let mut config4 = Config::default();
    config4.load_json(json4, Context::new(Source::Programmatic, Level::User));
    let result4 = config4.deserialize::<Status>();
    assert_eq!(result4.unwrap(), Status::Inactive);
}

// ============================================================================
// Tagged Enum without rename (default snake_case)
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "variant")]
enum DefaultNaming {
    SimpleUnit,
    StructVariant { value: i32 },
}

#[test]
fn test_tagged_enum_default_naming() {
    // Should use snake_case by default
    let json1 = r#"{"variant": "simple_unit"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<DefaultNaming>();
    assert_eq!(result1.unwrap(), DefaultNaming::SimpleUnit);

    let json2 = r#"{"variant": "struct_variant", "value": 42}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<DefaultNaming>();
    assert_eq!(result2.unwrap(), DefaultNaming::StructVariant { value: 42 });
}

// ============================================================================
// Untagged Enum Tests
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum NixConfig {
    Simple(String),
    Complex { packages: Vec<String> },
}

#[test]
fn test_untagged_enum_simple_string() {
    let json = r#""flake.nix""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize simple string: {:?}",
        result
    );
    assert_eq!(result.unwrap(), NixConfig::Simple("flake.nix".to_string()));
}

#[test]
fn test_untagged_enum_complex_object() {
    let json = r#"{
        "packages": ["gcc", "make", "cmake"]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize complex object: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        NixConfig::Complex {
            packages: vec!["gcc".to_string(), "make".to_string(), "cmake".to_string()]
        }
    );
}

// ============================================================================
// Untagged Enum with Multiple Types
// ============================================================================

// Note: For untagged enums, variant order matters! More specific types should come first.
// Because our FromContextValue allows type coercion (int -> bool, int -> string, etc.),
// we need to put the most specific types first (Int before Bool, since ints coerce to bools).
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum FlexibleValue {
    Int(i64),   // More specific - must come before Bool
    Bool(bool), // Less specific - integers coerce to bool
    Text(String),
}

#[test]
fn test_untagged_enum_bool() {
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<FlexibleValue>();
    assert_eq!(result.unwrap(), FlexibleValue::Bool(true));
}

#[test]
fn test_untagged_enum_int() {
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<FlexibleValue>();
    assert_eq!(result.unwrap(), FlexibleValue::Int(42));
}

#[test]
fn test_untagged_enum_string() {
    let json = r#""hello world""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<FlexibleValue>();
    assert_eq!(
        result.unwrap(),
        FlexibleValue::Text("hello world".to_string())
    );
}

// ============================================================================
// Untagged Enum No Match Error
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum StrictValue {
    Config { name: String, value: i32 },
}

#[test]
fn test_untagged_enum_no_match_error() {
    // Provide an object that doesn't have required fields
    let json = r#"{"unrelated": "data"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StrictValue>();
    assert!(result.is_err(), "Should fail when no variant matches");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("does not match any variant"),
        "Error should indicate no variant matched: {}",
        err_str
    );
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum NamedAlternatives {
    First { first: String },
    Second { second: String },
}

#[derive(Debug, compote::Config, PartialEq)]
struct NestedNamedAlternative {
    choice: NamedAlternatives,
}

#[test]
fn test_untagged_failed_named_alternative_does_not_leak_diagnostics() {
    let mut config = Config::default();
    config.load_json(
        r#"{"choice":{"second":"selected"}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let result: NestedNamedAlternative = config.deserialize().expect("second variant should match");
    assert_eq!(
        result.choice,
        NamedAlternatives::Second {
            second: "selected".to_string(),
        }
    );
    assert!(!config.errors().has_errors());
}

#[test]
fn test_untagged_no_match_keeps_aggregate_error_and_parent_path() {
    let mut config = Config::default();
    config.load_json(
        r#"{"choice":{"unrelated":true}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let error = config
        .deserialize::<NestedNamedAlternative>()
        .expect_err("no variant should match");
    assert_eq!(error.location(), "choice");
    assert!(error.to_string().contains("does not match any variant"));
    assert!(!config.errors().has_errors());
}

fn warn_selected_alternative<S: compote::CustomSource, L: compote::CustomLevel>(
    _config: &mut WarningAlternative,
    _value: &compote::ContextValue<S, L>,
    tracker: &mut compote::ErrorTracker,
) -> Result<(), compote::Error> {
    tracker.record_warning("selected warning alternative");
    Ok(())
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(post_process = "warn_selected_alternative")]
struct WarningAlternative {
    value: String,
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum WarningAlternatives {
    First { first: String },
    Second(WarningAlternative),
}

#[derive(Debug, compote::Config, PartialEq)]
struct NestedWarningAlternative {
    choice: WarningAlternatives,
}

#[test]
fn test_untagged_successful_alternative_commits_warning_with_parent_path() {
    let mut config = Config::default();
    config.load_json(
        r#"{"choice":{"value":"selected"}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let result: NestedWarningAlternative =
        config.deserialize().expect("second variant should match");
    assert_eq!(
        result.choice,
        WarningAlternatives::Second(WarningAlternative {
            value: "selected".to_string(),
        })
    );
    assert_eq!(config.errors().warnings().len(), 1);
    assert_eq!(config.errors().warnings()[0].path, "choice");
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum DiagnosticAlternatives {
    First {
        #[compote(default = "7")]
        count: i32,
    },
    Second {
        count: String,
    },
}

#[derive(Debug, compote::Config, PartialEq)]
struct NestedDiagnosticAlternative {
    choice: DiagnosticAlternatives,
}

#[test]
fn test_untagged_recoverable_errored_alternative_does_not_win_or_leak_diagnostics() {
    let mut config = Config::default();
    config.load_json(
        r#"{"choice":{"count":"selected"}}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let result: NestedDiagnosticAlternative = config
        .deserialize()
        .expect("clean later variant should match");
    assert_eq!(
        result.choice,
        DiagnosticAlternatives::Second {
            count: "selected".to_string(),
        }
    );
    assert!(!config.errors().has_errors());
    assert!(!config.errors().has_warnings());
}

// ============================================================================
// Enum as a Field in a Struct
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
struct PromptConfig {
    name: String,
    prompt: PromptType,
}

#[test]
fn test_enum_as_struct_field() {
    let json = r#"{
        "name": "age_prompt",
        "prompt": {
            "type": "int",
            "min": 0,
            "max": 150
        }
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize struct with enum field: {:?}",
        result
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.name, "age_prompt");
    assert_eq!(
        cfg.prompt,
        PromptType::Int {
            min: Some(0),
            max: Some(150)
        }
    );
}

#[test]
fn test_enum_in_vec() {
    let json = r#"{
        "prompts": [
            {"type": "text"},
            {"type": "choice", "choices": ["a", "b"]},
            {"type": "int", "min": 1}
        ]
    }"#;

    #[derive(Debug, compote::Config)]
    struct MultiPrompt {
        prompts: Vec<PromptType>,
    }

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<MultiPrompt>();
    assert!(
        result.is_ok(),
        "Should deserialize vec of enums: {:?}",
        result
    );

    let cfg = result.unwrap();
    assert_eq!(cfg.prompts.len(), 3);
    assert_eq!(cfg.prompts[0], PromptType::Text);
    assert_eq!(
        cfg.prompts[1],
        PromptType::Choice {
            choices: vec!["a".to_string(), "b".to_string()]
        }
    );
    assert_eq!(
        cfg.prompts[2],
        PromptType::Int {
            min: Some(1),
            max: None
        }
    );
}

// ============================================================================
// YAML format tests
// ============================================================================

#[cfg(feature = "yaml")]
#[test]
fn test_tagged_enum_yaml() {
    // Note: YAML parses unquoted "yes"/"no" as booleans, so we use quoted strings
    let yaml = r#"
type: choice
choices:
  - "yes"
  - "no"
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PromptType>();
    assert!(result.is_ok(), "Should deserialize from YAML: {:?}", result);
    assert_eq!(
        result.unwrap(),
        PromptType::Choice {
            choices: vec!["yes".to_string(), "no".to_string()]
        }
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_untagged_enum_yaml_string() {
    let yaml = r#"flake.nix"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize string from YAML: {:?}",
        result
    );
    assert_eq!(result.unwrap(), NixConfig::Simple("flake.nix".to_string()));
}

#[cfg(feature = "yaml")]
#[test]
fn test_untagged_enum_yaml_object() {
    let yaml = r#"
packages:
  - gcc
  - make
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize object from YAML: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        NixConfig::Complex {
            packages: vec!["gcc".to_string(), "make".to_string()]
        }
    );
}

// ============================================================================
// Struct-level scalar_as and array_as Tests
// ============================================================================

/// Test struct with scalar_as attribute - accepts string input and wraps as object
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "file")]
struct NixFileSpec {
    file: Option<String>,
    packages: Option<Vec<String>>,
}

#[test]
fn test_scalar_as_string_input() {
    // String input should be wrapped as {file: "shell.nix"}
    let json = r#""shell.nix""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixFileSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize scalar as object: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("shell.nix".to_string()));
    assert_eq!(spec.packages, None);
}

#[test]
fn test_scalar_as_object_passthrough() {
    // Object input should pass through unchanged
    let json = r#"{"file": "flake.nix", "packages": ["gcc"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixFileSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize object normally: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("flake.nix".to_string()));
    assert_eq!(spec.packages, Some(vec!["gcc".to_string()]));
}

/// Test struct with array_as attribute - accepts array input and wraps as object
#[derive(Debug, compote::Config, PartialEq)]
#[compote(array_as = "packages")]
struct NixPackagesSpec {
    file: Option<String>,
    packages: Option<Vec<String>>,
}

#[test]
fn test_array_as_array_input() {
    // Array input should be wrapped as {packages: ["pkg1", "pkg2"]}
    let json = r#"["pkg1", "pkg2", "pkg3"]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixPackagesSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize array as object: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, None);
    assert_eq!(
        spec.packages,
        Some(vec![
            "pkg1".to_string(),
            "pkg2".to_string(),
            "pkg3".to_string()
        ])
    );
}

#[test]
fn test_array_as_object_passthrough() {
    // Object input should pass through unchanged
    let json = r#"{"file": "default.nix"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixPackagesSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize object normally: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("default.nix".to_string()));
    assert_eq!(spec.packages, None);
}

/// Test struct with both scalar_as and array_as attributes
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "file", array_as = "packages")]
struct NixSpec {
    file: Option<String>,
    packages: Option<Vec<String>>,
}

#[test]
fn test_both_scalar_as_and_array_as_with_string() {
    // String input should be wrapped as {file: "shell.nix"}
    let json = r#""shell.nix""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(result.is_ok(), "Should deserialize scalar: {:?}", result);
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("shell.nix".to_string()));
    assert_eq!(spec.packages, None);
}

#[test]
fn test_both_scalar_as_and_array_as_with_array() {
    // Array input should be wrapped as {packages: ["pkg1", "pkg2"]}
    let json = r#"["pkg1", "pkg2"]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(result.is_ok(), "Should deserialize array: {:?}", result);
    let spec = result.unwrap();
    assert_eq!(spec.file, None);
    assert_eq!(
        spec.packages,
        Some(vec!["pkg1".to_string(), "pkg2".to_string()])
    );
}

#[test]
fn test_both_scalar_as_and_array_as_with_object() {
    // Object input should pass through unchanged
    let json = r#"{"file": "shell.nix", "packages": ["make", "cmake"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(result.is_ok(), "Should deserialize object: {:?}", result);
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("shell.nix".to_string()));
    assert_eq!(
        spec.packages,
        Some(vec!["make".to_string(), "cmake".to_string()])
    );
}

#[test]
fn test_scalar_as_with_int_input() {
    // Int input should also be wrapped as scalar
    #[derive(Debug, compote::Config, PartialEq)]
    #[compote(scalar_as = "count")]
    struct CountSpec {
        count: Option<i64>,
        name: Option<String>,
    }

    let json = r#"42"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<CountSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize int scalar: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.count, Some(42));
    assert_eq!(spec.name, None);
}

#[test]
fn test_scalar_as_with_bool_input() {
    // Bool input should also be wrapped as scalar
    #[derive(Debug, compote::Config, PartialEq)]
    #[compote(scalar_as = "enabled")]
    struct EnabledSpec {
        enabled: Option<bool>,
        name: Option<String>,
    }

    let json = r#"true"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<EnabledSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize bool scalar: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.enabled, Some(true));
    assert_eq!(spec.name, None);
}

#[cfg(feature = "yaml")]
#[test]
fn test_scalar_as_yaml() {
    // Test with YAML format
    let yaml = r#"shell.nix"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize YAML scalar: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("shell.nix".to_string()));
    assert_eq!(spec.packages, None);
}

#[cfg(feature = "yaml")]
#[test]
fn test_array_as_yaml() {
    // Test with YAML format
    let yaml = r#"
- gcc
- make
- cmake
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize YAML array: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, None);
    assert_eq!(
        spec.packages,
        Some(vec![
            "gcc".to_string(),
            "make".to_string(),
            "cmake".to_string()
        ])
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_object_yaml() {
    // Test with YAML object format
    let yaml = r#"
file: default.nix
packages:
  - hello
  - world
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NixSpec>();
    assert!(
        result.is_ok(),
        "Should deserialize YAML object: {:?}",
        result
    );
    let spec = result.unwrap();
    assert_eq!(spec.file, Some("default.nix".to_string()));
    assert_eq!(
        spec.packages,
        Some(vec!["hello".to_string(), "world".to_string()])
    );
}

// ============================================================================
// rename_all Attribute Tests
// ============================================================================

/// Test enum with rename_all = "kebab-case"
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "kebab-case")]
enum KebabCaseEnum {
    FirstVariant,
    SecondOne,
    ThirdOption { value: i32 },
}

#[test]
fn test_rename_all_kebab_case() {
    // Test first variant with kebab-case
    let json1 = r#"{"type": "first-variant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<KebabCaseEnum>();
    assert!(
        result1.is_ok(),
        "Should deserialize kebab-case variant: {:?}",
        result1
    );
    assert_eq!(result1.unwrap(), KebabCaseEnum::FirstVariant);

    // Test second variant
    let json2 = r#"{"type": "second-one"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<KebabCaseEnum>();
    assert_eq!(result2.unwrap(), KebabCaseEnum::SecondOne);

    // Test struct variant
    let json3 = r#"{"type": "third-option", "value": 42}"#;
    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));
    let result3 = config3.deserialize::<KebabCaseEnum>();
    assert_eq!(result3.unwrap(), KebabCaseEnum::ThirdOption { value: 42 });
}

/// Test enum with rename_all = "camelCase"
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "kind", rename_all = "camelCase")]
enum CamelCaseEnum {
    FirstVariant,
    SecondOne,
    MyThirdOption,
}

#[test]
fn test_rename_all_camel_case() {
    let json1 = r#"{"kind": "firstVariant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<CamelCaseEnum>();
    assert!(
        result1.is_ok(),
        "Should deserialize camelCase variant: {:?}",
        result1
    );
    assert_eq!(result1.unwrap(), CamelCaseEnum::FirstVariant);

    let json2 = r#"{"kind": "secondOne"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<CamelCaseEnum>();
    assert_eq!(result2.unwrap(), CamelCaseEnum::SecondOne);

    let json3 = r#"{"kind": "myThirdOption"}"#;
    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));
    let result3 = config3.deserialize::<CamelCaseEnum>();
    assert_eq!(result3.unwrap(), CamelCaseEnum::MyThirdOption);
}

/// Test enum with rename_all = "PascalCase"
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "PascalCase")]
enum PascalCaseEnum {
    FirstVariant,
    SecondOne,
}

#[test]
fn test_rename_all_pascal_case() {
    // PascalCase keeps the original naming
    let json1 = r#"{"type": "FirstVariant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<PascalCaseEnum>();
    assert!(
        result1.is_ok(),
        "Should deserialize PascalCase variant: {:?}",
        result1
    );
    assert_eq!(result1.unwrap(), PascalCaseEnum::FirstVariant);

    let json2 = r#"{"type": "SecondOne"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<PascalCaseEnum>();
    assert_eq!(result2.unwrap(), PascalCaseEnum::SecondOne);
}

/// Test enum with rename_all = "SCREAMING_SNAKE_CASE"
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
enum ScreamingSnakeCaseEnum {
    FirstVariant,
    SecondOne,
}

#[test]
fn test_rename_all_screaming_snake_case() {
    let json1 = r#"{"type": "FIRST_VARIANT"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<ScreamingSnakeCaseEnum>();
    assert!(
        result1.is_ok(),
        "Should deserialize SCREAMING_SNAKE_CASE variant: {:?}",
        result1
    );
    assert_eq!(result1.unwrap(), ScreamingSnakeCaseEnum::FirstVariant);

    let json2 = r#"{"type": "SECOND_ONE"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<ScreamingSnakeCaseEnum>();
    assert_eq!(result2.unwrap(), ScreamingSnakeCaseEnum::SecondOne);
}

/// Test enum with rename_all = "snake_case" (explicit, same as default)
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "snake_case")]
enum SnakeCaseEnum {
    FirstVariant,
    SecondOne,
}

#[test]
fn test_rename_all_snake_case_explicit() {
    let json1 = r#"{"type": "first_variant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<SnakeCaseEnum>();
    assert!(
        result1.is_ok(),
        "Should deserialize snake_case variant: {:?}",
        result1
    );
    assert_eq!(result1.unwrap(), SnakeCaseEnum::FirstVariant);

    let json2 = r#"{"type": "second_one"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<SnakeCaseEnum>();
    assert_eq!(result2.unwrap(), SnakeCaseEnum::SecondOne);
}

/// Test that explicit rename overrides rename_all
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "kebab-case")]
enum RenameOverrideEnum {
    FirstVariant,
    #[compote(rename = "custom_name")]
    SecondVariant,
    ThirdVariant,
}

#[test]
fn test_rename_all_with_explicit_rename_override() {
    // First variant uses rename_all
    let json1 = r#"{"type": "first-variant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<RenameOverrideEnum>();
    assert_eq!(result1.unwrap(), RenameOverrideEnum::FirstVariant);

    // Second variant uses explicit rename, overriding rename_all
    let json2 = r#"{"type": "custom_name"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<RenameOverrideEnum>();
    assert_eq!(result2.unwrap(), RenameOverrideEnum::SecondVariant);

    // Third variant uses rename_all
    let json3 = r#"{"type": "third-variant"}"#;
    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));
    let result3 = config3.deserialize::<RenameOverrideEnum>();
    assert_eq!(result3.unwrap(), RenameOverrideEnum::ThirdVariant);

    // Second variant should NOT match kebab-case
    let json4 = r#"{"type": "second-variant"}"#;
    let mut config4 = Config::default();
    config4.load_json(json4, Context::new(Source::Programmatic, Level::User));
    let result4 = config4.deserialize::<RenameOverrideEnum>();
    assert!(
        result4.is_err(),
        "Should fail for non-matching variant name"
    );
}

/// Test rename_all with aliases
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type", rename_all = "kebab-case")]
enum RenameAllWithAliases {
    #[compote(alias = "v1", alias = "variant-one")]
    FirstVariant,
    SecondVariant,
}

#[test]
fn test_rename_all_with_aliases() {
    // Primary name (generated from rename_all)
    let json1 = r#"{"type": "first-variant"}"#;
    let mut config1 = Config::default();
    config1.load_json(json1, Context::new(Source::Programmatic, Level::User));
    let result1 = config1.deserialize::<RenameAllWithAliases>();
    assert_eq!(result1.unwrap(), RenameAllWithAliases::FirstVariant);

    // First alias
    let json2 = r#"{"type": "v1"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<RenameAllWithAliases>();
    assert_eq!(result2.unwrap(), RenameAllWithAliases::FirstVariant);

    // Second alias
    let json3 = r#"{"type": "variant-one"}"#;
    let mut config3 = Config::default();
    config3.load_json(json3, Context::new(Source::Programmatic, Level::User));
    let result3 = config3.deserialize::<RenameAllWithAliases>();
    assert_eq!(result3.unwrap(), RenameAllWithAliases::FirstVariant);

    // Regular variant with rename_all
    let json4 = r#"{"type": "second-variant"}"#;
    let mut config4 = Config::default();
    config4.load_json(json4, Context::new(Source::Programmatic, Level::User));
    let result4 = config4.deserialize::<RenameAllWithAliases>();
    assert_eq!(result4.unwrap(), RenameAllWithAliases::SecondVariant);
}

/// Test serialization with rename_all
#[test]
fn test_rename_all_serialization() {
    // Test kebab-case serialization
    let kebab = KebabCaseEnum::FirstVariant;
    let json = compote::to_json_compact(&kebab).unwrap();
    assert!(
        json.contains("first-variant"),
        "Serialized JSON should use kebab-case: {}",
        json
    );

    // Test camelCase serialization
    let camel = CamelCaseEnum::SecondOne;
    let json = compote::to_json_compact(&camel).unwrap();
    assert!(
        json.contains("secondOne"),
        "Serialized JSON should use camelCase: {}",
        json
    );

    // Test PascalCase serialization
    let pascal = PascalCaseEnum::FirstVariant;
    let json = compote::to_json_compact(&pascal).unwrap();
    assert!(
        json.contains("FirstVariant"),
        "Serialized JSON should use PascalCase: {}",
        json
    );

    // Test SCREAMING_SNAKE_CASE serialization
    let screaming = ScreamingSnakeCaseEnum::SecondOne;
    let json = compote::to_json_compact(&screaming).unwrap();
    assert!(
        json.contains("SECOND_ONE"),
        "Serialized JSON should use SCREAMING_SNAKE_CASE: {}",
        json
    );
}

/// Test that explicit rename is used for serialization even with rename_all
#[test]
fn test_rename_override_serialization() {
    let variant = RenameOverrideEnum::SecondVariant;
    let json = compote::to_json_compact(&variant).unwrap();
    assert!(
        json.contains("custom_name"),
        "Serialized JSON should use explicit rename: {}",
        json
    );
    assert!(
        !json.contains("second-variant"),
        "Should not use rename_all for overridden variant"
    );
}

// ============================================================================
// Value-Matched Enum Tests
// ============================================================================

/// Test enum with value_matched - variant determined by scalar value
#[derive(Debug, compote::Config, PartialEq)]
#[compote(value_matched)]
#[derive(Default)]
enum SelfUpdateConfig {
    #[compote(variant = true | "true" | "yes" | "y" | 1)]
    True,
    #[compote(variant = false | "false" | "no" | "n" | 0)]
    False,
    #[compote(variant = "nocheck")]
    NoCheck,
    #[compote(fallback)]
    #[default]
    Ask,
}

#[test]
fn test_value_matched_bool_true() {
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match true: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::True);
}

#[test]
fn test_value_matched_bool_false() {
    let json = r#"false"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match false: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::False);
}

#[test]
fn test_value_matched_string_true() {
    let json = r#""yes""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match 'yes': {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::True);
}

#[test]
fn test_value_matched_string_false() {
    let json = r#""no""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match 'no': {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::False);
}

#[test]
fn test_value_matched_int_one() {
    let json = r#"1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match 1: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::True);
}

#[test]
fn test_value_matched_int_zero() {
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match 0: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::False);
}

#[test]
fn test_value_matched_string_nocheck() {
    let json = r#""nocheck""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should match 'nocheck': {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::NoCheck);
}

#[test]
fn test_value_matched_fallback() {
    // "ask" should fall through to the fallback variant
    let json = r#""ask""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should fallback: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::Ask);
}

#[test]
fn test_value_matched_unknown_fallback() {
    // Unknown value should fall through to the fallback variant
    let json = r#""unknown_value""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    assert!(result.is_ok(), "Should fallback for unknown: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::Ask);
}

/// Test value_matched enum with truthy/falsy predicates
#[derive(Debug, compote::Config, PartialEq)]
#[compote(value_matched)]
enum TruthyFalsyEnum {
    #[compote(variant = truthy)]
    Truthy,
    #[compote(variant = falsy)]
    Falsy,
}

#[test]
fn test_value_matched_truthy_bool() {
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(result.is_ok(), "Should match truthy: {:?}", result);
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Truthy);
}

#[test]
fn test_value_matched_truthy_string_yes() {
    let json = r#""yes""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match truthy string 'yes': {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Truthy);
}

#[test]
fn test_value_matched_truthy_nonzero_int() {
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match truthy non-zero int: {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Truthy);
}

#[test]
fn test_value_matched_falsy_bool() {
    let json = r#"false"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(result.is_ok(), "Should match falsy: {:?}", result);
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Falsy);
}

#[test]
fn test_value_matched_falsy_string_no() {
    let json = r#""no""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match falsy string 'no': {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Falsy);
}

#[test]
fn test_value_matched_falsy_zero() {
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(result.is_ok(), "Should match falsy zero: {:?}", result);
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Falsy);
}

#[test]
fn test_value_matched_truthy_string_on() {
    let json = r#""on""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match truthy string 'on': {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Truthy);
}

#[test]
fn test_value_matched_truthy_string_on_uppercase() {
    let json = r#""ON""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match truthy string 'ON' (case insensitive): {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Truthy);
}

#[test]
fn test_value_matched_falsy_string_off() {
    let json = r#""off""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match falsy string 'off': {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Falsy);
}

#[test]
fn test_value_matched_falsy_string_off_uppercase() {
    let json = r#""OFF""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<TruthyFalsyEnum>();
    assert!(
        result.is_ok(),
        "Should match falsy string 'OFF' (case insensitive): {:?}",
        result
    );
    assert_eq!(result.unwrap(), TruthyFalsyEnum::Falsy);
}

/// Test value_matched enum without fallback - should error on unknown values
#[derive(Debug, compote::Config, PartialEq)]
#[compote(value_matched)]
enum StrictValueMatched {
    #[compote(variant = "on")]
    On,
    #[compote(variant = "off")]
    Off,
}

#[test]
fn test_value_matched_no_fallback_error() {
    let json = r#""unknown""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StrictValueMatched>();
    assert!(
        result.is_err(),
        "Should fail for unmatched value without fallback"
    );
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("no variant") || err_str.contains("matches"),
        "Error should indicate no match: {}",
        err_str
    );
}

/// Test value_matched enum as struct field
#[derive(Debug, compote::Config, PartialEq)]
struct ConfigWithValueMatchedField {
    name: String,
    #[compote(default)]
    enabled: SelfUpdateConfig,
}

#[test]
fn test_value_matched_as_struct_field() {
    let json = r#"{"name": "test", "enabled": true}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ConfigWithValueMatchedField>();
    assert!(
        result.is_ok(),
        "Should deserialize struct with value_matched field: {:?}",
        result
    );
    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    assert_eq!(cfg.enabled, SelfUpdateConfig::True);
}

#[test]
fn test_value_matched_as_struct_field_string() {
    let json = r#"{"name": "test", "enabled": "nocheck"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ConfigWithValueMatchedField>();
    assert!(
        result.is_ok(),
        "Should deserialize struct with string value: {:?}",
        result
    );
    let cfg = result.unwrap();
    assert_eq!(cfg.enabled, SelfUpdateConfig::NoCheck);
}

#[test]
fn test_value_matched_serialization() {
    // Test serialization of value_matched enum
    let val = SelfUpdateConfig::True;
    let json = compote::to_json_compact(&val).unwrap();
    // Should serialize as "true" (the first match value)
    assert!(
        json.contains("true"),
        "Should serialize as 'true': {}",
        json
    );

    let val2 = SelfUpdateConfig::NoCheck;
    let json2 = compote::to_json_compact(&val2).unwrap();
    assert!(
        json2.contains("nocheck"),
        "Should serialize as 'nocheck': {}",
        json2
    );

    let val3 = SelfUpdateConfig::Ask;
    let json3 = compote::to_json_compact(&val3).unwrap();
    // Fallback variant serializes as snake_case variant name
    assert!(
        json3.contains("ask"),
        "Should serialize fallback as 'ask': {}",
        json3
    );
}

/// Test value_matched with negative integer
#[derive(Debug, compote::Config, PartialEq)]
#[compote(value_matched)]
enum SignedIntEnum {
    #[compote(variant = -1)]
    NegativeOne,
    #[compote(variant = 0)]
    Zero,
    #[compote(variant = 1)]
    PositiveOne,
    #[compote(fallback)]
    Other,
}

#[test]
fn test_value_matched_negative_int() {
    let json = r#"-1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SignedIntEnum>();
    assert!(result.is_ok(), "Should match -1: {:?}", result);
    assert_eq!(result.unwrap(), SignedIntEnum::NegativeOne);
}

#[test]
fn test_value_matched_zero_int() {
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SignedIntEnum>();
    assert!(result.is_ok(), "Should match 0: {:?}", result);
    assert_eq!(result.unwrap(), SignedIntEnum::Zero);
}

#[test]
fn test_value_matched_positive_int() {
    let json = r#"1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SignedIntEnum>();
    assert!(result.is_ok(), "Should match 1: {:?}", result);
    assert_eq!(result.unwrap(), SignedIntEnum::PositiveOne);
}

#[cfg(feature = "yaml")]
#[test]
fn test_value_matched_yaml() {
    // Test with YAML format
    let yaml = r#"yes"#;
    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SelfUpdateConfig>();
    // YAML parses unquoted "yes" as boolean true
    assert!(result.is_ok(), "Should parse YAML: {:?}", result);
    assert_eq!(result.unwrap(), SelfUpdateConfig::True);
}

// ============================================================================
// Tagged Enum with Fallback Tests
// ============================================================================

/// Test internally tagged enum with fallback unit variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type")]
enum EnvValueType {
    #[compote(rename = "path")]
    Path,
    #[compote(rename = "text", fallback)]
    Text,
}

#[test]
fn test_tagged_enum_fallback_unit_variant_with_tag() {
    // When tag is present, should use the tag
    let json = r#"{"type": "path"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueType>();
    assert!(result.is_ok(), "Should deserialize with tag: {:?}", result);
    assert_eq!(result.unwrap(), EnvValueType::Path);
}

#[test]
fn test_tagged_enum_fallback_unit_variant_without_tag() {
    // When tag is missing, should use fallback
    let json = r#"{"value": "something"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueType>();
    assert!(
        result.is_ok(),
        "Should use fallback when tag is missing: {:?}",
        result
    );
    assert_eq!(result.unwrap(), EnvValueType::Text);
}

#[test]
fn test_tagged_enum_fallback_empty_object() {
    // Empty object should use fallback
    let json = r#"{}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueType>();
    assert!(
        result.is_ok(),
        "Should use fallback for empty object: {:?}",
        result
    );
    assert_eq!(result.unwrap(), EnvValueType::Text);
}

/// Test internally tagged enum with fallback struct variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(tag = "type")]
enum EnvValueSpec {
    #[compote(rename = "path")]
    Path { value: String },
    #[compote(rename = "text", fallback)]
    Text { value: Option<String> },
}

#[test]
fn test_tagged_enum_fallback_struct_variant_with_tag() {
    // When tag is present, should use the tag
    let json = r#"{"type": "path", "value": "/some/path"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueSpec>();
    assert!(result.is_ok(), "Should deserialize with tag: {:?}", result);
    assert_eq!(
        result.unwrap(),
        EnvValueSpec::Path {
            value: "/some/path".to_string()
        }
    );
}

#[test]
fn test_tagged_enum_fallback_struct_variant_without_tag() {
    // When tag is missing, should use fallback and parse fields
    let json = r#"{"value": "some text"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueSpec>();
    assert!(
        result.is_ok(),
        "Should use fallback when tag is missing: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        EnvValueSpec::Text {
            value: Some("some text".to_string())
        }
    );
}

#[test]
fn test_tagged_enum_fallback_struct_variant_empty_object() {
    // Empty object should use fallback with None for optional fields
    let json = r#"{}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueSpec>();
    assert!(
        result.is_ok(),
        "Should use fallback for empty object: {:?}",
        result
    );
    assert_eq!(result.unwrap(), EnvValueSpec::Text { value: None });
}

#[test]
fn test_tagged_enum_fallback_tuple_variant_with_tag() {
    // Note: tagged enums with tuple variants may need special handling
    // The tuple inner value typically comes from a "value" field or the entire object
    // For now, let's test the basic case
    #[derive(Debug, compote::Config, PartialEq)]
    #[compote(tag = "type")]
    enum SimpleTupleTagged {
        #[compote(rename = "text")]
        Text { content: String },
        #[compote(rename = "number", fallback)]
        Number { content: i64 },
    }

    let json = r#"{"type": "text", "content": "hello"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SimpleTupleTagged>();
    assert!(result.is_ok(), "Should deserialize with tag: {:?}", result);
    assert_eq!(
        result.unwrap(),
        SimpleTupleTagged::Text {
            content: "hello".to_string()
        }
    );
}

#[test]
fn test_tagged_enum_fallback_tuple_variant_without_tag() {
    #[derive(Debug, compote::Config, PartialEq)]
    #[compote(tag = "type")]
    enum SimpleTupleTagged {
        #[compote(rename = "text")]
        Text { content: String },
        #[compote(rename = "number", fallback)]
        Number { content: i64 },
    }

    let json = r#"{"content": 123}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SimpleTupleTagged>();
    assert!(
        result.is_ok(),
        "Should use fallback when tag is missing: {:?}",
        result
    );
    assert_eq!(result.unwrap(), SimpleTupleTagged::Number { content: 123 });
}

/// Test that fallback works correctly with nested objects
#[derive(Debug, compote::Config, PartialEq)]
struct OuterConfig {
    name: String,
    value_spec: EnvValueSpec,
}

#[test]
fn test_tagged_enum_fallback_in_struct_field() {
    // When nested and tag is missing, should use fallback
    let json = r#"{"name": "test", "value_spec": {"value": "nested value"}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<OuterConfig>();
    assert!(
        result.is_ok(),
        "Should deserialize struct with fallback enum: {:?}",
        result
    );
    let cfg = result.unwrap();
    assert_eq!(cfg.name, "test");
    assert_eq!(
        cfg.value_spec,
        EnvValueSpec::Text {
            value: Some("nested value".to_string())
        }
    );
}

#[test]
fn test_tagged_enum_fallback_in_vec() {
    let json = r#"[
        {"type": "path", "value": "/path/one"},
        {"value": "text value"},
        {"type": "text", "value": "explicit text"}
    ]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Vec<EnvValueSpec>>();
    assert!(
        result.is_ok(),
        "Should deserialize vec with mixed tags: {:?}",
        result
    );
    let vec = result.unwrap();
    assert_eq!(vec.len(), 3);
    assert_eq!(
        vec[0],
        EnvValueSpec::Path {
            value: "/path/one".to_string()
        }
    );
    assert_eq!(
        vec[1],
        EnvValueSpec::Text {
            value: Some("text value".to_string())
        }
    );
    assert_eq!(
        vec[2],
        EnvValueSpec::Text {
            value: Some("explicit text".to_string())
        }
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_tagged_enum_fallback_yaml() {
    // Test with YAML format
    let yaml = r#"
value: some yaml text
"#;
    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnvValueSpec>();
    assert!(result.is_ok(), "Should use fallback in YAML: {:?}", result);
    assert_eq!(
        result.unwrap(),
        EnvValueSpec::Text {
            value: Some("some yaml text".to_string())
        }
    );
}

// ============================================================================
// Untagged Enum with Fallback Tests
// ============================================================================

/// Test untagged enum with fallback unit variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum UntaggedWithFallback {
    Int(i64),
    Text(String),
    #[compote(fallback)]
    Unknown,
}

#[test]
fn test_untagged_enum_fallback_int() {
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedWithFallback>();
    assert!(result.is_ok(), "Should deserialize int: {:?}", result);
    assert_eq!(result.unwrap(), UntaggedWithFallback::Int(42));
}

#[test]
fn test_untagged_enum_fallback_text() {
    let json = r#""hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedWithFallback>();
    assert!(result.is_ok(), "Should deserialize text: {:?}", result);
    assert_eq!(
        result.unwrap(),
        UntaggedWithFallback::Text("hello".to_string())
    );
}

#[test]
fn test_untagged_enum_fallback_unknown() {
    // Arrays don't match Int or Text, so fallback to Unknown
    let json = r#"[1, 2, 3]"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedWithFallback>();
    assert!(
        result.is_ok(),
        "Should use fallback for array: {:?}",
        result
    );
    assert_eq!(result.unwrap(), UntaggedWithFallback::Unknown);
}

#[test]
fn test_untagged_enum_fallback_null() {
    // Null doesn't match Int or Text, so fallback to Unknown
    let json = r#"null"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedWithFallback>();
    assert!(result.is_ok(), "Should use fallback for null: {:?}", result);
    assert_eq!(result.unwrap(), UntaggedWithFallback::Unknown);
}

/// Test untagged enum with fallback tuple variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum UntaggedTupleFallback {
    Specific {
        name: String,
        value: i32,
    },
    #[compote(fallback)]
    Generic(String),
}

#[test]
fn test_untagged_enum_fallback_tuple_specific() {
    let json = r#"{"name": "test", "value": 42}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedTupleFallback>();
    assert!(
        result.is_ok(),
        "Should match specific variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        UntaggedTupleFallback::Specific {
            name: "test".to_string(),
            value: 42
        }
    );
}

#[test]
fn test_untagged_enum_fallback_tuple_generic() {
    // String input doesn't match Specific struct, so fallback to Generic
    let json = r#""fallback value""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedTupleFallback>();
    assert!(
        result.is_ok(),
        "Should use fallback tuple variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        UntaggedTupleFallback::Generic("fallback value".to_string())
    );
}

/// Test untagged enum with fallback struct variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum UntaggedStructFallback {
    Simple(String),
    Number(i64),
    #[compote(fallback)]
    Complex {
        data: Option<String>,
    },
}

#[test]
fn test_untagged_enum_fallback_struct_simple() {
    let json = r#""simple string""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedStructFallback>();
    assert!(result.is_ok(), "Should match simple variant: {:?}", result);
    assert_eq!(
        result.unwrap(),
        UntaggedStructFallback::Simple("simple string".to_string())
    );
}

#[test]
fn test_untagged_enum_fallback_struct_number() {
    let json = r#"123"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedStructFallback>();
    assert!(result.is_ok(), "Should match number variant: {:?}", result);
    assert_eq!(result.unwrap(), UntaggedStructFallback::Number(123));
}

#[test]
fn test_untagged_enum_fallback_struct_complex() {
    // Object input doesn't match Simple or Number, so fallback to Complex
    let json = r#"{"data": "complex data"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedStructFallback>();
    assert!(
        result.is_ok(),
        "Should use fallback struct variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        UntaggedStructFallback::Complex {
            data: Some("complex data".to_string())
        }
    );
}

#[test]
fn test_untagged_enum_fallback_struct_empty_object() {
    // Empty object falls back to Complex with None
    let json = r#"{}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedStructFallback>();
    assert!(
        result.is_ok(),
        "Should use fallback for empty object: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        UntaggedStructFallback::Complex { data: None }
    );
}

/// Test untagged enum without fallback still errors on no match
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum UntaggedNoFallback {
    Int(i64),
    Text(String),
}

#[test]
fn test_untagged_enum_no_fallback_error() {
    // Array doesn't match any variant and there's no fallback
    let json = r#"[1, 2, 3]"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UntaggedNoFallback>();
    assert!(
        result.is_err(),
        "Should error when no variant matches and no fallback"
    );
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("does not match any variant"),
        "Error should mention no match: {}",
        err_str
    );
}

// ============================================================================
// Untagged Enum with Variant Predicates Tests
// ============================================================================

/// Test untagged enum with exact string matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum CommandType {
    #[compote(variant = "help" | "h" | "?")]
    Help,
    #[compote(variant = "version" | "v")]
    Version,
    #[compote(fallback)]
    Custom(String),
}

#[test]
fn test_untagged_enum_variant_exact_string() {
    let json = r#""help""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CommandType>();
    assert!(result.is_ok(), "Should match exact string: {:?}", result);
    assert_eq!(result.unwrap(), CommandType::Help);
}

#[test]
fn test_untagged_enum_variant_exact_string_alias() {
    let json = r#""?""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CommandType>();
    assert!(
        result.is_ok(),
        "Should match exact string alias: {:?}",
        result
    );
    assert_eq!(result.unwrap(), CommandType::Help);
}

#[test]
fn test_untagged_enum_variant_exact_string_fallback() {
    let json = r#""custom-command""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CommandType>();
    assert!(
        result.is_ok(),
        "Should fallback for unmatched string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        CommandType::Custom("custom-command".to_string())
    );
}

/// Test untagged enum with truthy/falsy predicates
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum EnabledState {
    #[compote(variant = truthy)]
    Enabled,
    #[compote(variant = falsy)]
    Disabled,
    #[compote(fallback)]
    Unknown,
}

#[test]
fn test_untagged_enum_variant_truthy() {
    // Test boolean true
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match truthy: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Enabled);

    // Test string "yes"
    let json = r#""yes""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match truthy string: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Enabled);

    // Test integer 1
    let json = r#"1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match truthy integer: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Enabled);
}

#[test]
fn test_untagged_enum_variant_falsy() {
    // Test boolean false
    let json = r#"false"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match falsy: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Disabled);

    // Test string "no"
    let json = r#""no""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match falsy string: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Disabled);

    // Test integer 0
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EnabledState>();
    assert!(result.is_ok(), "Should match falsy integer: {:?}", result);
    assert_eq!(result.unwrap(), EnabledState::Disabled);
}

/// Test untagged enum with null variant
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum OptionalValue {
    #[compote(variant = null)]
    NotSet,
    #[compote(variant = any_string)]
    Text(String),
    #[compote(variant = any_int)]
    Number(i64),
}

#[test]
fn test_untagged_enum_variant_null() {
    let json = r#"null"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<OptionalValue>();
    assert!(result.is_ok(), "Should match null: {:?}", result);
    assert_eq!(result.unwrap(), OptionalValue::NotSet);
}

#[test]
fn test_untagged_enum_variant_any_string() {
    let json = r#""hello world""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<OptionalValue>();
    assert!(result.is_ok(), "Should match any_string: {:?}", result);
    assert_eq!(
        result.unwrap(),
        OptionalValue::Text("hello world".to_string())
    );
}

#[test]
fn test_untagged_enum_variant_any_int() {
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<OptionalValue>();
    assert!(result.is_ok(), "Should match any_int: {:?}", result);
    assert_eq!(result.unwrap(), OptionalValue::Number(42));
}

/// Test untagged enum with mixed predicates and type-based matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum MixedMatching {
    #[compote(variant = "special")]
    Special,
    // No predicate - uses type-based matching
    Config {
        host: String,
        port: i32,
    },
    #[compote(fallback)]
    Other(String),
}

#[test]
fn test_untagged_enum_mixed_predicate_match() {
    let json = r#""special""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MixedMatching>();
    assert!(result.is_ok(), "Should match predicate: {:?}", result);
    assert_eq!(result.unwrap(), MixedMatching::Special);
}

#[test]
fn test_untagged_enum_mixed_type_match() {
    let json = r#"{"host": "localhost", "port": 8080}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MixedMatching>();
    assert!(result.is_ok(), "Should match struct type: {:?}", result);
    assert_eq!(
        result.unwrap(),
        MixedMatching::Config {
            host: "localhost".to_string(),
            port: 8080
        }
    );
}

#[test]
fn test_untagged_enum_mixed_fallback() {
    let json = r#""other string""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MixedMatching>();
    assert!(result.is_ok(), "Should fallback: {:?}", result);
    assert_eq!(
        result.unwrap(),
        MixedMatching::Other("other string".to_string())
    );
}

/// Test priority: predicates should match before type-based matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(untagged)]
enum PriorityTest {
    #[compote(variant = "special")]
    SpecialString,
    // Type-based matching for String would also match "special", but predicate has priority
    GenericString(String),
}

#[test]
fn test_untagged_enum_predicate_priority() {
    // "special" should match the predicate variant, not the generic String
    let json = r#""special""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<PriorityTest>();
    assert!(
        result.is_ok(),
        "Should match predicate with priority: {:?}",
        result
    );
    assert_eq!(result.unwrap(), PriorityTest::SpecialString);
}

#[test]
fn test_untagged_enum_type_match_after_predicate() {
    // Other strings should match the generic String variant
    let json = r#""other""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<PriorityTest>();
    assert!(result.is_ok(), "Should match type-based: {:?}", result);
    assert_eq!(
        result.unwrap(),
        PriorityTest::GenericString("other".to_string())
    );
}

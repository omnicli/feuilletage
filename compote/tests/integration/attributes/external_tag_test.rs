// This entire test file requires JSON support since most tests use load_json
#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};

// ============================================================================
// Basic External Tag Enum Tests
// ============================================================================

/// Simple config for homebrew-like packages
#[derive(Debug, compote::Config, PartialEq)]
#[compote(array_as = "packages")]
struct HomebrewConfig {
    #[compote(allow_single)]
    packages: Vec<String>,
}

/// Simple config for python version
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "version")]
struct PythonConfig {
    version: Option<String>,
}

/// Basic external tag enum without fallback
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum BasicTool {
    #[compote(rename = "homebrew")]
    Homebrew(HomebrewConfig),
    #[compote(rename = "python")]
    Python(PythonConfig),
}

#[test]
fn test_basic_external_tag_homebrew() {
    let json = r#"{"homebrew": ["ripgrep", "fd"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(
        result.is_ok(),
        "Should deserialize homebrew variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        BasicTool::Homebrew(HomebrewConfig {
            packages: vec!["ripgrep".to_string(), "fd".to_string()]
        })
    );
}

#[test]
fn test_basic_external_tag_python() {
    let json = r#"{"python": "3.11"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(
        result.is_ok(),
        "Should deserialize python variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        BasicTool::Python(PythonConfig {
            version: Some("3.11".to_string())
        })
    );
}

#[test]
fn test_basic_external_tag_unknown_key_error() {
    let json = r#"{"unknown": "value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(
        result.is_err(),
        "Should error on unknown key without fallback"
    );

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("unknown variant") && err_str.contains("unknown"),
        "Error should mention unknown variant: {}",
        err_str
    );
}

#[test]
fn test_external_tag_multiple_keys_error() {
    let json = r#"{"homebrew": ["pkg"], "python": "3.11"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(result.is_err(), "Should error on multiple keys");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("single key-value pair") || err_str.contains("2 keys"),
        "Error should mention expected single key: {}",
        err_str
    );
}

// ============================================================================
// External Tag with Fallback Variant
// ============================================================================

/// Config for mise tool manager - captures the tool name
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "version")]
struct MiseConfig {
    #[compote(default = "")]
    name: String,

    version: Option<String>,
}

/// External tag enum with fallback for unknown tools
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ToolWithFallback {
    #[compote(rename = "homebrew")]
    Homebrew(HomebrewConfig),

    #[compote(rename = "python")]
    Python(PythonConfig),

    #[compote(fallback, from_tag = "name")]
    Mise(MiseConfig),
}

#[test]
fn test_external_tag_known_variant_with_fallback() {
    // Known variants should still work when fallback exists
    let json = r#"{"homebrew": ["ripgrep"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolWithFallback>();
    assert!(
        result.is_ok(),
        "Should deserialize known variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ToolWithFallback::Homebrew(HomebrewConfig {
            packages: vec!["ripgrep".to_string()]
        })
    );
}

#[test]
fn test_external_tag_fallback_unknown_key() {
    // Unknown key should route to fallback variant
    let json = r#"{"rust": "1.75"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolWithFallback>();
    assert!(
        result.is_ok(),
        "Should deserialize to fallback variant: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ToolWithFallback::Mise(MiseConfig {
            name: "rust".to_string(),
            version: Some("1.75".to_string()),
        })
    );
}

#[test]
fn test_external_tag_fallback_from_tag_injection() {
    // Test that from_tag field receives the key value
    let json = r#"{"terraform": "1.5"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolWithFallback>();
    assert!(result.is_ok(), "Should deserialize: {:?}", result);

    if let ToolWithFallback::Mise(mise) = result.unwrap() {
        assert_eq!(
            mise.name, "terraform",
            "from_tag field should receive the key"
        );
        assert_eq!(mise.version, Some("1.5".to_string()));
    } else {
        panic!("Expected Mise variant");
    }
}

#[test]
fn test_external_tag_fallback_multiple_unknown() {
    // Test various unknown keys
    let tools = vec![
        (r#"{"golang": "1.21"}"#, "golang"),
        (r#"{"nodejs": "20"}"#, "nodejs"),
        (r#"{"java": "17"}"#, "java"),
    ];

    for (json, expected_name) in tools {
        let mut config = Config::default();
        config.load_json(json, Context::new(Source::Programmatic, Level::User));

        let result = config.deserialize::<ToolWithFallback>();
        assert!(
            result.is_ok(),
            "Should deserialize {}: {:?}",
            expected_name,
            result
        );

        if let ToolWithFallback::Mise(mise) = result.unwrap() {
            assert_eq!(
                mise.name, expected_name,
                "from_tag should be {}",
                expected_name
            );
        } else {
            panic!("Expected Mise variant for {}", expected_name);
        }
    }
}

// ============================================================================
// Variant Aliases Tests
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ToolWithAliases {
    #[compote(rename = "homebrew", aliases = ["brew", "hb"])]
    Homebrew(HomebrewConfig),

    #[compote(rename = "python", alias = "py")]
    Python(PythonConfig),
}

#[test]
fn test_external_tag_primary_name() {
    let json = r#"{"homebrew": ["pkg"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ToolWithAliases>();
    assert!(result.is_ok());
    assert!(matches!(result.unwrap(), ToolWithAliases::Homebrew(_)));
}

#[test]
fn test_external_tag_alias_array_form() {
    // Test aliases = ["brew", "hb"] syntax
    let json = r#"{"brew": ["pkg"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ToolWithAliases>();
    assert!(result.is_ok(), "Should match alias 'brew': {:?}", result);
    assert!(matches!(result.unwrap(), ToolWithAliases::Homebrew(_)));

    let json2 = r#"{"hb": ["pkg"]}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<ToolWithAliases>();
    assert!(result2.is_ok(), "Should match alias 'hb': {:?}", result2);
    assert!(matches!(result2.unwrap(), ToolWithAliases::Homebrew(_)));
}

#[test]
fn test_external_tag_alias_singular_form() {
    // Test alias = "py" syntax (singular)
    let json = r#"{"py": "3.11"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ToolWithAliases>();
    assert!(result.is_ok(), "Should match alias 'py': {:?}", result);
    assert!(matches!(result.unwrap(), ToolWithAliases::Python(_)));
}

// ============================================================================
// Recursive Enums (Vec<Self>)
// ============================================================================

/// Config for compound operations
#[derive(Debug, compote::Config, PartialEq)]
#[compote(array_as = "items")]
struct CompoundConfig {
    #[compote(allow_single)]
    items: Vec<RecursiveTool>,
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum RecursiveTool {
    #[compote(rename = "homebrew")]
    Homebrew(HomebrewConfig),

    #[compote(rename = "python")]
    Python(PythonConfig),

    #[compote(rename = "and")]
    And(CompoundConfig),

    #[compote(rename = "or")]
    Or(CompoundConfig),

    #[compote(fallback, from_tag = "name")]
    Mise(MiseConfig),
}

#[test]
fn test_external_tag_recursive_simple() {
    let json = r#"{"and": [{"homebrew": ["pkg1"]}, {"python": "3.11"}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RecursiveTool>();
    assert!(result.is_ok(), "Should deserialize recursive: {:?}", result);

    if let RecursiveTool::And(compound) = result.unwrap() {
        assert_eq!(compound.items.len(), 2);
        assert!(matches!(&compound.items[0], RecursiveTool::Homebrew(_)));
        assert!(matches!(&compound.items[1], RecursiveTool::Python(_)));
    } else {
        panic!("Expected And variant");
    }
}

#[test]
fn test_external_tag_recursive_nested() {
    let json = r#"{"or": [{"and": [{"rust": "1.75"}, {"go": "1.21"}]}, {"homebrew": ["pkg"]}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<RecursiveTool>();
    assert!(
        result.is_ok(),
        "Should deserialize nested recursive: {:?}",
        result
    );

    if let RecursiveTool::Or(outer) = result.unwrap() {
        assert_eq!(outer.items.len(), 2);

        // First item should be And containing Mise variants
        if let RecursiveTool::And(inner) = &outer.items[0] {
            assert_eq!(inner.items.len(), 2);

            // Both should be Mise (fallback) since rust/go aren't explicit variants
            if let RecursiveTool::Mise(mise1) = &inner.items[0] {
                assert_eq!(mise1.name, "rust");
            } else {
                panic!("Expected Mise for 'rust'");
            }

            if let RecursiveTool::Mise(mise2) = &inner.items[1] {
                assert_eq!(mise2.name, "go");
            } else {
                panic!("Expected Mise for 'go'");
            }
        } else {
            panic!("Expected And variant");
        }

        // Second item should be Homebrew
        assert!(matches!(&outer.items[1], RecursiveTool::Homebrew(_)));
    } else {
        panic!("Expected Or variant");
    }
}

// ============================================================================
// Vec<ExternalTagEnum> Tests
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
struct ToolList {
    tools: Vec<ToolWithFallback>,
}

#[test]
fn test_external_tag_vec() {
    let json = r#"{
        "tools": [
            {"homebrew": ["ripgrep", "fd"]},
            {"python": "3.11"},
            {"rust": "1.75"},
            {"terraform": "1.5"}
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolList>();
    assert!(
        result.is_ok(),
        "Should deserialize vec of external_tag: {:?}",
        result
    );

    let list = result.unwrap();
    assert_eq!(list.tools.len(), 4);

    assert!(matches!(&list.tools[0], ToolWithFallback::Homebrew(_)));
    assert!(matches!(&list.tools[1], ToolWithFallback::Python(_)));

    if let ToolWithFallback::Mise(mise) = &list.tools[2] {
        assert_eq!(mise.name, "rust");
    } else {
        panic!("Expected Mise for rust");
    }

    if let ToolWithFallback::Mise(mise) = &list.tools[3] {
        assert_eq!(mise.name, "terraform");
    } else {
        panic!("Expected Mise for terraform");
    }
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_external_tag_serialize_basic() {
    let tool = BasicTool::Homebrew(HomebrewConfig {
        packages: vec!["ripgrep".to_string(), "fd".to_string()],
    });

    let json = compote::to_json_compact(&tool).unwrap();
    assert!(
        json.contains("homebrew"),
        "Should serialize with external tag: {}",
        json
    );
    assert!(
        json.contains("ripgrep"),
        "Should contain packages: {}",
        json
    );
}

#[test]
fn test_external_tag_serialize_fallback() {
    let tool = ToolWithFallback::Mise(MiseConfig {
        name: "rust".to_string(),
        version: Some("1.75".to_string()),
    });

    let json = compote::to_json_compact(&tool).unwrap();
    // Should use the from_tag value as the key
    assert!(
        json.contains("rust"),
        "Should use from_tag value as key: {}",
        json
    );
    assert!(json.contains("1.75"), "Should contain version: {}", json);
}

// ============================================================================
// YAML Format Tests
// ============================================================================

#[cfg(feature = "yaml")]
#[test]
fn test_external_tag_yaml() {
    let yaml = r#"
homebrew:
  - ripgrep
  - fd
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(result.is_ok(), "Should deserialize from YAML: {:?}", result);
    assert_eq!(
        result.unwrap(),
        BasicTool::Homebrew(HomebrewConfig {
            packages: vec!["ripgrep".to_string(), "fd".to_string()]
        })
    );
}

#[cfg(feature = "yaml")]
#[test]
fn test_external_tag_yaml_vec() {
    let yaml = r#"
tools:
  - homebrew:
      - ripgrep
  - python: "3.11"
  - rust: "1.75"
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolList>();
    assert!(result.is_ok(), "Should deserialize YAML vec: {:?}", result);

    let list = result.unwrap();
    assert_eq!(list.tools.len(), 3);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_external_tag_empty_map_error() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(result.is_err(), "Should error on empty map");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("0 keys") || err_str.contains("single key-value pair"),
        "Error should mention 0 keys: {}",
        err_str
    );
}

#[test]
fn test_external_tag_non_object_error() {
    let json = r#""just a string""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<BasicTool>();
    assert!(result.is_err(), "Should error on non-object");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    assert!(
        err_str.contains("TypeMismatch") || err_str.contains("object"),
        "Error should mention type mismatch: {}",
        err_str
    );
}

// ============================================================================
// Default Naming (snake_case)
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum DefaultNamingTool {
    HomebrewPackage(HomebrewConfig),
    PythonVersion(PythonConfig),
}

#[test]
fn test_external_tag_default_snake_case() {
    let json = r#"{"homebrew_package": ["pkg"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<DefaultNamingTool>();
    assert!(
        result.is_ok(),
        "Should use snake_case by default: {:?}",
        result
    );

    let json2 = r#"{"python_version": "3.11"}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<DefaultNamingTool>();
    assert!(
        result2.is_ok(),
        "Should use snake_case by default: {:?}",
        result2
    );
}

// ============================================================================
// Graceful Degradation Tests - Vec<ExternalTagEnum> with malformed entries
// ============================================================================

#[test]
fn test_vec_external_tag_graceful_degradation_skips_bad_entries() {
    // Vec<ExternalTagEnum> should skip malformed entries (multiple keys) and keep good ones
    let json = r#"{
        "tools": [
            {"homebrew": ["ripgrep"]},
            {"python": "3.11", "extra": "bad"},
            {"rust": "1.75"}
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolList>();
    // Should succeed - bad entries are skipped
    assert!(
        result.is_ok(),
        "Should gracefully skip bad entries: {:?}",
        result
    );

    let list = result.unwrap();
    // Should have 2 items (first and third), second was skipped due to multiple keys
    assert_eq!(
        list.tools.len(),
        2,
        "Should have 2 valid tools, bad one skipped"
    );

    // First is homebrew
    assert!(matches!(&list.tools[0], ToolWithFallback::Homebrew(_)));

    // Second is rust (fallback/mise) - third entry in original list
    if let ToolWithFallback::Mise(mise) = &list.tools[1] {
        assert_eq!(mise.name, "rust");
    } else {
        panic!("Expected Mise for rust");
    }
}

#[test]
fn test_vec_external_tag_graceful_degradation_records_error() {
    // Verify that errors are recorded in tracker when entries are skipped
    let json = r#"{
        "tools": [
            {"homebrew": ["ripgrep"]},
            {"python": "3.11", "extra": "bad"}
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolList>();

    // Should succeed
    assert!(result.is_ok(), "Should succeed: {:?}", result);

    // Config's error tracker should have recorded the error for the bad entry
    assert!(
        config.errors().has_errors(),
        "Tracker should have recorded the error"
    );
    let errors = config.errors().errors();
    assert_eq!(errors.len(), 1, "Should have exactly one error recorded");

    let error_str = format!("{}", errors[0]);
    assert!(
        error_str.contains("single key-value pair"),
        "Error should mention single key-value pair: {}",
        error_str
    );
}

#[test]
fn test_vec_external_tag_graceful_degradation_all_bad() {
    // If all entries are bad, result is empty vec (not an error)
    let json = r#"{
        "tools": [
            {"a": 1, "b": 2},
            {"c": 3, "d": 4}
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<ToolList>();

    // Should succeed with empty vec
    assert!(
        result.is_ok(),
        "Should succeed with empty vec: {:?}",
        result
    );
    let list = result.unwrap();
    assert_eq!(list.tools.len(), 0, "Should have no valid tools");

    // Config's error tracker should have recorded 2 errors
    assert!(config.errors().has_errors());
    assert_eq!(
        config.errors().errors().len(),
        2,
        "Should have 2 errors recorded"
    );
}

#[test]
fn test_vec_external_tag_graceful_degradation_user_friendly_message() {
    // Verify the error message is user-friendly (not internal/technical)
    let json = r#"{
        "tools": [
            {"key1": "val1", "key2": "val2"}
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let _result = config.deserialize::<ToolList>();

    let errors = config.errors().errors();
    assert!(!errors.is_empty(), "Should have errors");

    let error_str = format!("{}", errors[0]);
    // Should NOT contain internal terminology
    assert!(
        !error_str.contains("external_tag"),
        "Error should NOT mention 'external_tag' (internal): {}",
        error_str
    );
    // Should contain user-friendly message
    assert!(
        error_str.contains("single key-value pair"),
        "Error should mention 'single key-value pair': {}",
        error_str
    );
}

// ============================================================================
// Null Variant and Scalar Variant Tests
// ============================================================================

/// StringFilter enum demonstrating null_variant and scalar_variant
/// - null -> Any (matches everything)
/// - "*.rs" -> Glob("*.rs") (glob pattern)
/// - {contains: "foo"} -> Contains("foo") (standard external_tag)
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringFilter {
    #[compote(null_variant)]
    Any,

    #[compote(scalar_variant)]
    Glob(String),

    Contains(String),
    StartsWith(String),
    EndsWith(String),
    Regex(String),
    Exact(String),
}

#[test]
fn test_null_variant_unit() {
    // null input should select the null_variant
    let json = r#"null"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize null to Any: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::Any);
}

#[test]
fn test_scalar_variant_string() {
    // string input should select the scalar_variant
    let json = r#""*.rs""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize string to Glob: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::Glob("*.rs".to_string()));
}

#[test]
fn test_standard_external_tag_with_special_variants() {
    // map input should still work for standard variants
    let json = r#"{"contains": "foo"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize map to Contains: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::Contains("foo".to_string()));
}

#[test]
fn test_external_tag_starts_with() {
    let json = r#"{"starts_with": "bar"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize to StartsWith: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::StartsWith("bar".to_string()));
}

#[test]
fn test_external_tag_ends_with() {
    let json = r#"{"ends_with": ".txt"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize to EndsWith: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::EndsWith(".txt".to_string()));
}

#[test]
fn test_external_tag_regex() {
    let json = r#"{"regex": "^foo.*bar$"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_ok(), "Should deserialize to Regex: {:?}", result);
    assert_eq!(
        result.unwrap(),
        StringFilter::Regex("^foo.*bar$".to_string())
    );
}

#[test]
fn test_external_tag_exact() {
    let json = r#"{"exact": "hello world"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_ok(), "Should deserialize to Exact: {:?}", result);
    assert_eq!(
        result.unwrap(),
        StringFilter::Exact("hello world".to_string())
    );
}

// ============================================================================
// Serialization Tests for Special Variants
// ============================================================================

#[test]
fn test_null_variant_serialization() {
    let filter = StringFilter::Any;
    let json = compote::to_json_compact(&filter).unwrap();
    assert_eq!(json, "null", "null_variant should serialize to null");
}

#[test]
fn test_scalar_variant_serialization() {
    let filter = StringFilter::Glob("*.rs".to_string());
    let json = compote::to_json_compact(&filter).unwrap();
    assert_eq!(
        json, r#""*.rs""#,
        "scalar_variant should serialize as bare string"
    );
}

#[test]
fn test_standard_variant_serialization_with_special_variants() {
    let filter = StringFilter::Contains("foo".to_string());
    let json = compote::to_json_compact(&filter).unwrap();
    assert!(
        json.contains("contains"),
        "Should serialize with external tag: {}",
        json
    );
    assert!(json.contains("foo"), "Should contain value: {}", json);
}

// ============================================================================
// Roundtrip Tests
// ============================================================================

#[test]
fn test_null_variant_roundtrip() {
    let original = StringFilter::Any;
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_scalar_variant_roundtrip() {
    let original = StringFilter::Glob("**/*.rs".to_string());
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_standard_variant_roundtrip_with_special() {
    let original = StringFilter::Contains("test".to_string());
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

// ============================================================================
// Vec<StringFilter> Tests
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
struct FilterList {
    filters: Vec<StringFilter>,
}

#[test]
fn test_vec_with_mixed_special_variants() {
    let json = r#"{
        "filters": [
            null,
            "*.rs",
            {"contains": "test"},
            {"starts_with": "src/"},
            "**/*.md"
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FilterList>();
    assert!(
        result.is_ok(),
        "Should deserialize mixed list: {:?}",
        result
    );

    let list = result.unwrap();
    assert_eq!(list.filters.len(), 5);
    assert_eq!(list.filters[0], StringFilter::Any);
    assert_eq!(list.filters[1], StringFilter::Glob("*.rs".to_string()));
    assert_eq!(list.filters[2], StringFilter::Contains("test".to_string()));
    assert_eq!(
        list.filters[3],
        StringFilter::StartsWith("src/".to_string())
    );
    assert_eq!(list.filters[4], StringFilter::Glob("**/*.md".to_string()));
}

// ============================================================================
// YAML Tests for Special Variants
// ============================================================================

// Note: YAML null_variant tests are omitted because serde-saphyr has a quirk
// where it treats `null` and `~` as strings in certain contexts. The null_variant
// feature is thoroughly tested with JSON which correctly parses null values.
// See test_null_variant_unit() and test_null_variant_roundtrip() for coverage.

#[cfg(feature = "yaml")]
#[test]
fn test_special_variants_yaml_scalar() {
    let yaml = r#""*.rs""#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(
        result.is_ok(),
        "Should deserialize YAML scalar: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilter::Glob("*.rs".to_string()));
}

#[cfg(feature = "yaml")]
#[test]
fn test_special_variants_yaml_mixed_list() {
    // Test scalar_variant and standard variants with YAML
    // Note: null_variant is not tested here due to serde-saphyr parsing quirks with null
    let yaml = r#"
filters:
  - "*.rs"
  - contains: test
  - starts_with: src/
"#;

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<FilterList>();
    assert!(
        result.is_ok(),
        "Should deserialize YAML mixed list: {:?}",
        result
    );

    let list = result.unwrap();
    assert_eq!(list.filters.len(), 3);
    assert_eq!(list.filters[0], StringFilter::Glob("*.rs".to_string()));
    assert_eq!(list.filters[1], StringFilter::Contains("test".to_string()));
    assert_eq!(
        list.filters[2],
        StringFilter::StartsWith("src/".to_string())
    );
}

// ============================================================================
// Scalar Variant with Different Types
// ============================================================================

/// Test scalar_variant with integer type
#[derive(Debug, compote::Config, PartialEq)]
struct RangeConfig {
    min: i64,
    max: i64,
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum NumberOrRange {
    #[compote(scalar_variant)]
    Single(i64),

    Range(RangeConfig),
}

#[test]
fn test_scalar_variant_integer() {
    let json = r#"42"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NumberOrRange>();
    assert!(
        result.is_ok(),
        "Should deserialize int to Single: {:?}",
        result
    );
    assert_eq!(result.unwrap(), NumberOrRange::Single(42));
}

#[test]
fn test_scalar_variant_with_map_fallback() {
    let json = r#"{"range": {"min": 1, "max": 10}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<NumberOrRange>();
    assert!(
        result.is_ok(),
        "Should deserialize map to Range: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        NumberOrRange::Range(RangeConfig { min: 1, max: 10 })
    );
}

// ============================================================================
// Null Variant with Newtype (inner type with Default)
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
struct NamedConfig {
    name: String,
}

/// Test null_variant as newtype with Default inner type
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum OptionalValue {
    #[compote(null_variant)]
    Default(String), // String::default() = ""

    #[compote(scalar_variant)]
    Value(String),

    Named(NamedConfig),
}

#[test]
fn test_null_variant_newtype_uses_default() {
    let json = r#"null"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<OptionalValue>();
    assert!(
        result.is_ok(),
        "Should deserialize null to Default: {:?}",
        result
    );
    assert_eq!(result.unwrap(), OptionalValue::Default(String::new()));
}

#[test]
fn test_null_variant_newtype_serializes_as_null() {
    let value = OptionalValue::Default("ignored".to_string());
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(json, "null", "null_variant should always serialize to null");
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_array_input_without_array_variant_error() {
    // Array input with no array handling should error
    let json = r#"["a", "b", "c"]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilter>();
    assert!(result.is_err(), "Should error on array input");

    let err = result.unwrap_err();
    let err_str = format!("{:?}", err);
    // With null_variant and scalar_variant, error should mention expected types
    assert!(
        err_str.contains("object") || err_str.contains("scalar") || err_str.contains("null"),
        "Error should mention expected types: {}",
        err_str
    );
}

// ============================================================================
// Alias 'scalar' instead of 'scalar_variant'
// ============================================================================

/// Test that #[compote(scalar)] works as alias for #[compote(scalar_variant)]
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum AliasTest {
    #[compote(scalar)] // Using short form
    Direct(String),

    Wrapped(String),
}

#[test]
fn test_scalar_alias() {
    let json = r#""hello""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<AliasTest>();
    assert!(result.is_ok(), "scalar alias should work: {:?}", result);
    assert_eq!(result.unwrap(), AliasTest::Direct("hello".to_string()));
}

// ============================================================================
// Unified Variant Syntax Tests (variant = null, variant = any_scalar, etc.)
// ============================================================================

/// StringFilter using the new unified variant = null syntax
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringFilterUnifiedNull {
    #[compote(variant = null)] // New unified syntax
    Any,

    #[compote(scalar_variant)] // Legacy syntax for scalar
    Glob(String),

    Contains(String),
}

#[test]
fn test_unified_null_variant_syntax() {
    // null input should select the variant with variant = null
    let json = r#"null"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterUnifiedNull>();
    assert!(result.is_ok(), "variant = null should work: {:?}", result);
    assert_eq!(result.unwrap(), StringFilterUnifiedNull::Any);
}

#[test]
fn test_unified_null_variant_serialization() {
    let filter = StringFilterUnifiedNull::Any;
    let json = compote::to_json_compact(&filter).unwrap();
    assert_eq!(json, "null", "variant = null should serialize to null");
}

/// StringFilter using the new unified variant = any_scalar syntax
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringFilterUnifiedScalar {
    #[compote(null_variant)] // Legacy syntax for null
    Any,

    #[compote(variant = any_scalar)] // New unified syntax
    Glob(String),

    Contains(String),
}

#[test]
fn test_unified_scalar_variant_syntax() {
    // string input should select the variant with variant = any_scalar
    let json = r#""*.rs""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterUnifiedScalar>();
    assert!(
        result.is_ok(),
        "variant = any_scalar should work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        StringFilterUnifiedScalar::Glob("*.rs".to_string())
    );
}

#[test]
fn test_unified_scalar_variant_serialization() {
    let filter = StringFilterUnifiedScalar::Glob("*.rs".to_string());
    let json = compote::to_json_compact(&filter).unwrap();
    assert_eq!(
        json, r#""*.rs""#,
        "variant = any_scalar should serialize as bare scalar"
    );
}

/// StringFilter using both new unified syntax options
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringFilterFullyUnified {
    #[compote(variant = null)] // New unified syntax for null
    Any,

    #[compote(variant = any_scalar)] // New unified syntax for scalar
    Glob(String),

    Contains(String),
    StartsWith(String),
    EndsWith(String),
}

#[test]
fn test_fully_unified_null() {
    let json = r#"null"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(
        result.is_ok(),
        "variant = null in fully unified enum: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilterFullyUnified::Any);
}

#[test]
fn test_fully_unified_scalar() {
    let json = r#""*.rs""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(
        result.is_ok(),
        "variant = any_scalar in fully unified enum: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        StringFilterFullyUnified::Glob("*.rs".to_string())
    );
}

#[test]
fn test_fully_unified_map() {
    let json = r#"{"contains": "foo"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(
        result.is_ok(),
        "map variant in fully unified enum: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        StringFilterFullyUnified::Contains("foo".to_string())
    );
}

#[test]
fn test_fully_unified_roundtrip_null() {
    let original = StringFilterFullyUnified::Any;
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_fully_unified_roundtrip_scalar() {
    let original = StringFilterFullyUnified::Glob("**/*.md".to_string());
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_fully_unified_roundtrip_map() {
    let original = StringFilterFullyUnified::Contains("test".to_string());
    let json = compote::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringFilterFullyUnified>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

/// Test that variant = any_string matches only strings (more specific than any_scalar)
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringSpecificScalar {
    #[compote(variant = any_string)]
    Text(String),

    Named(String),
}

#[test]
fn test_any_string_matches_string() {
    let json = r#""hello world""#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringSpecificScalar>();
    assert!(
        result.is_ok(),
        "any_string should match string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        StringSpecificScalar::Text("hello world".to_string())
    );
}

#[test]
fn test_any_string_map_still_works() {
    let json = r#"{"named": "foo"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<StringSpecificScalar>();
    assert!(
        result.is_ok(),
        "map variant should still work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        StringSpecificScalar::Named("foo".to_string())
    );
}

/// Test that variant = any_int matches only integers
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum IntSpecificScalar {
    #[compote(variant = any_int)]
    Number(i64),

    Named(String),
}

#[test]
fn test_any_int_matches_int() {
    let json = r#"42"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<IntSpecificScalar>();
    assert!(result.is_ok(), "any_int should match integer: {:?}", result);
    assert_eq!(result.unwrap(), IntSpecificScalar::Number(42));
}

#[test]
fn test_any_int_matches_negative_int() {
    let json = r#"-123"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<IntSpecificScalar>();
    assert!(
        result.is_ok(),
        "any_int should match negative integer: {:?}",
        result
    );
    assert_eq!(result.unwrap(), IntSpecificScalar::Number(-123));
}

#[test]
fn test_any_int_map_still_works() {
    let json = r#"{"named": "foo"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<IntSpecificScalar>();
    assert!(
        result.is_ok(),
        "map variant should still work: {:?}",
        result
    );
    assert_eq!(result.unwrap(), IntSpecificScalar::Named("foo".to_string()));
}

/// Test Vec<T> with unified syntax enum
#[derive(Debug, compote::Config, PartialEq)]
struct UnifiedFilterList {
    filters: Vec<StringFilterFullyUnified>,
}

#[test]
fn test_vec_with_unified_syntax() {
    let json = r#"{
        "filters": [
            null,
            "*.rs",
            {"contains": "test"},
            {"starts_with": "src/"},
            "**/*.md"
        ]
    }"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<UnifiedFilterList>();
    assert!(
        result.is_ok(),
        "Should deserialize vec with unified syntax: {:?}",
        result
    );

    let list = result.unwrap();
    assert_eq!(list.filters.len(), 5);
    assert_eq!(list.filters[0], StringFilterFullyUnified::Any);
    assert_eq!(
        list.filters[1],
        StringFilterFullyUnified::Glob("*.rs".to_string())
    );
    assert_eq!(
        list.filters[2],
        StringFilterFullyUnified::Contains("test".to_string())
    );
    assert_eq!(
        list.filters[3],
        StringFilterFullyUnified::StartsWith("src/".to_string())
    );
    assert_eq!(
        list.filters[4],
        StringFilterFullyUnified::Glob("**/*.md".to_string())
    );
}

/// Test variant = null with newtype (inner type with Default)
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum UnifiedOptionalValue {
    #[compote(variant = null)]
    Default(String), // String::default() = ""

    #[compote(variant = any_scalar)]
    Value(String),

    Named(String),
}

#[test]
fn test_unified_null_newtype_uses_default() {
    let json = r#"null"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<UnifiedOptionalValue>();
    assert!(
        result.is_ok(),
        "Should deserialize null to Default: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        UnifiedOptionalValue::Default(String::new())
    );
}

#[test]
fn test_unified_null_newtype_serializes_as_null() {
    let value = UnifiedOptionalValue::Default("ignored".to_string());
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(
        json, "null",
        "variant = null should always serialize to null"
    );
}

// ============================================================================
// Exact Value Matching Tests
// ============================================================================

/// Test exact string matching in external_tag
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum Command {
    #[compote(variant = "help" | "h" | "?")]
    Help,
    #[compote(variant = "version" | "v")]
    Version,
    #[compote(variant = any_string)] // fallback for other strings
    Custom(String),
    Run(RunConfig), // map: {"run": {...}}
}

#[derive(Debug, compote::Config, PartialEq)]
struct RunConfig {
    command: String,
}

#[test]
fn test_exact_string_matching_help() {
    let json = r#""help""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Command>();
    assert!(
        result.is_ok(),
        "Should match exact string 'help': {:?}",
        result
    );
    assert_eq!(result.unwrap(), Command::Help);
}

#[test]
fn test_exact_string_matching_alias() {
    // Test "h" and "?" aliases for Help
    for input in &[r#""h""#, r#""?""#] {
        let mut config = Config::default();
        config.load_json(input, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<Command>();
        assert!(
            result.is_ok(),
            "Should match alias for 'help': {:?}",
            result
        );
        assert_eq!(result.unwrap(), Command::Help);
    }
}

#[test]
fn test_exact_string_matching_version() {
    let json = r#""version""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Command>();
    assert!(
        result.is_ok(),
        "Should match exact string 'version': {:?}",
        result
    );
    assert_eq!(result.unwrap(), Command::Version);
}

#[test]
fn test_exact_string_fallback_to_any_string() {
    // Unknown string should fall through to any_string variant
    let json = r#""something_else""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Command>();
    assert!(
        result.is_ok(),
        "Should match any_string wildcard: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        Command::Custom("something_else".to_string())
    );
}

#[test]
fn test_exact_string_map_still_works() {
    let json = r#"{"run": {"command": "echo hello"}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Command>();
    assert!(
        result.is_ok(),
        "Map variant should still work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        Command::Run(RunConfig {
            command: "echo hello".to_string()
        })
    );
}

#[test]
fn test_exact_string_serialization() {
    // Help should serialize to "help" (first match value)
    let cmd = Command::Help;
    let json = compote::to_json_compact(&cmd).unwrap();
    assert_eq!(
        json, r#""help""#,
        "Help should serialize to first match value"
    );

    // Version should serialize to "version"
    let cmd = Command::Version;
    let json = compote::to_json_compact(&cmd).unwrap();
    assert_eq!(
        json, r#""version""#,
        "Version should serialize to first match value"
    );
}

/// Test exact bool matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum Toggle {
    #[compote(variant = true)]
    On,
    #[compote(variant = false)]
    Off,
    Named(ToggleConfig), // map: {"named": "..."}
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "name")]
struct ToggleConfig {
    name: String,
}

#[test]
fn test_exact_bool_matching_true() {
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Toggle>();
    assert!(result.is_ok(), "Should match exact bool true: {:?}", result);
    assert_eq!(result.unwrap(), Toggle::On);
}

#[test]
fn test_exact_bool_matching_false() {
    let json = r#"false"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Toggle>();
    assert!(
        result.is_ok(),
        "Should match exact bool false: {:?}",
        result
    );
    assert_eq!(result.unwrap(), Toggle::Off);
}

#[test]
fn test_exact_bool_map_fallback() {
    let json = r#"{"named": "custom_toggle"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Toggle>();
    assert!(result.is_ok(), "Map variant should work: {:?}", result);
    assert_eq!(
        result.unwrap(),
        Toggle::Named(ToggleConfig {
            name: "custom_toggle".to_string()
        })
    );
}

#[test]
fn test_exact_bool_serialization() {
    let toggle = Toggle::On;
    let json = compote::to_json_compact(&toggle).unwrap();
    assert_eq!(json, "true", "On should serialize to true");

    let toggle = Toggle::Off;
    let json = compote::to_json_compact(&toggle).unwrap();
    assert_eq!(json, "false", "Off should serialize to false");
}

/// Test exact int matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum Priority {
    #[compote(variant = 1)]
    High,
    #[compote(variant = 2)]
    Medium,
    #[compote(variant = 3)]
    Low,
    #[compote(variant = any_int)] // other integers
    Custom(i64),
    Detailed(PriorityConfig), // map
}

#[derive(Debug, compote::Config, PartialEq)]
struct PriorityConfig {
    level: i64,
    reason: String,
}

#[test]
fn test_exact_int_matching() {
    for (json, expected) in &[
        (r#"1"#, Priority::High),
        (r#"2"#, Priority::Medium),
        (r#"3"#, Priority::Low),
    ] {
        let mut config = Config::default();
        config.load_json(json, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<Priority>();
        assert!(result.is_ok(), "Should match exact int: {:?}", result);
        assert_eq!(&result.unwrap(), expected);
    }
}

#[test]
fn test_exact_int_fallback_to_any_int() {
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Priority>();
    assert!(
        result.is_ok(),
        "Should fall through to any_int: {:?}",
        result
    );
    assert_eq!(result.unwrap(), Priority::Custom(42));
}

#[test]
fn test_exact_int_serialization() {
    let priority = Priority::High;
    let json = compote::to_json_compact(&priority).unwrap();
    assert_eq!(json, "1", "High should serialize to 1");
}

// ============================================================================
// Truthy/Falsy Predicate Tests
// ============================================================================

/// Test truthy/falsy predicates
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum Feature {
    #[compote(variant = truthy)]
    Enabled,
    #[compote(variant = falsy)]
    Disabled,
    Config(FeatureConfig), // map: {"config": {...}}
}

#[derive(Debug, compote::Config, PartialEq)]
struct FeatureConfig {
    name: String,
    #[compote(default = false)]
    enabled: bool,
}

#[test]
fn test_truthy_matches_bool_true() {
    let json = r#"true"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Feature>();
    assert!(result.is_ok(), "truthy should match true: {:?}", result);
    assert_eq!(result.unwrap(), Feature::Enabled);
}

#[test]
fn test_truthy_matches_string_true() {
    for input in &[r#""true""#, r#""yes""#, r#""y""#, r#""1""#] {
        let mut config = Config::default();
        config.load_json(input, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<Feature>();
        assert!(
            result.is_ok(),
            "truthy should match {}: {:?}",
            input,
            result
        );
        assert_eq!(result.unwrap(), Feature::Enabled);
    }
}

#[test]
fn test_truthy_matches_nonzero_int() {
    for input in &[r#"1"#, r#"42"#, r#"-1"#] {
        let mut config = Config::default();
        config.load_json(input, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<Feature>();
        assert!(
            result.is_ok(),
            "truthy should match non-zero int {}: {:?}",
            input,
            result
        );
        assert_eq!(result.unwrap(), Feature::Enabled);
    }
}

#[test]
fn test_falsy_matches_bool_false() {
    let json = r#"false"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Feature>();
    assert!(result.is_ok(), "falsy should match false: {:?}", result);
    assert_eq!(result.unwrap(), Feature::Disabled);
}

#[test]
fn test_falsy_matches_string_false() {
    for input in &[r#""false""#, r#""no""#, r#""n""#, r#""0""#] {
        let mut config = Config::default();
        config.load_json(input, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<Feature>();
        assert!(result.is_ok(), "falsy should match {}: {:?}", input, result);
        assert_eq!(result.unwrap(), Feature::Disabled);
    }
}

#[test]
fn test_falsy_matches_zero_int() {
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Feature>();
    assert!(result.is_ok(), "falsy should match 0: {:?}", result);
    assert_eq!(result.unwrap(), Feature::Disabled);
}

#[test]
fn test_truthy_falsy_map_still_works() {
    let json = r#"{"config": {"name": "dark_mode", "enabled": true}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Feature>();
    assert!(result.is_ok(), "Map variant should work: {:?}", result);
    assert_eq!(
        result.unwrap(),
        Feature::Config(FeatureConfig {
            name: "dark_mode".to_string(),
            enabled: true
        })
    );
}

#[test]
fn test_truthy_serializes_as_true() {
    let feature = Feature::Enabled;
    let json = compote::to_json_compact(&feature).unwrap();
    assert_eq!(json, "true", "truthy variant should serialize as true");
}

#[test]
fn test_falsy_serializes_as_false() {
    let feature = Feature::Disabled;
    let json = compote::to_json_compact(&feature).unwrap();
    assert_eq!(json, "false", "falsy variant should serialize as false");
}

// ============================================================================
// Float Matching Tests
// ============================================================================

/// Test float matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum Threshold {
    #[compote(variant = 0.0)]
    Zero,
    #[compote(variant = 1.0)]
    Full,
    #[compote(variant = 0.5)]
    Half,
    #[compote(variant = any_float)]
    Custom(f64),
    Config(ThresholdConfig),
}

#[derive(Debug, compote::Config, PartialEq)]
struct ThresholdConfig {
    value: f64,
    unit: String,
}

#[test]
fn test_exact_float_matching() {
    let json = r#"0.0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Threshold>();
    assert!(result.is_ok(), "Should match exact float 0.0: {:?}", result);
    assert_eq!(result.unwrap(), Threshold::Zero);
}

#[test]
fn test_exact_float_one() {
    let json = r#"1.0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Threshold>();
    assert!(result.is_ok(), "Should match exact float 1.0: {:?}", result);
    assert_eq!(result.unwrap(), Threshold::Full);
}

#[test]
fn test_exact_float_half() {
    let json = r#"0.5"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Threshold>();
    assert!(result.is_ok(), "Should match exact float 0.5: {:?}", result);
    assert_eq!(result.unwrap(), Threshold::Half);
}

#[test]
fn test_exact_float_fallback_to_any_float() {
    let json = r#"0.75"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Threshold>();
    assert!(
        result.is_ok(),
        "Should fall through to any_float: {:?}",
        result
    );
    assert_eq!(result.unwrap(), Threshold::Custom(0.75));
}

#[test]
fn test_exact_float_serialization() {
    let threshold = Threshold::Zero;
    let json = compote::to_json_compact(&threshold).unwrap();
    assert_eq!(json, "0.0", "Zero should serialize to 0.0");

    let threshold = Threshold::Full;
    let json = compote::to_json_compact(&threshold).unwrap();
    assert_eq!(json, "1.0", "Full should serialize to 1.0");
}

// ============================================================================
// Matching Order / Precedence Tests
// ============================================================================

/// Test that exact matches take precedence over wildcards
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum MatchOrder {
    #[compote(variant = "special")]
    Special,
    #[compote(variant = any_string)] // should not match "special"
    Other(String),
}

#[test]
fn test_exact_before_wildcard_string() {
    let json = r#""special""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MatchOrder>();
    assert!(
        result.is_ok(),
        "Should match exact 'special' not wildcard: {:?}",
        result
    );
    assert_eq!(result.unwrap(), MatchOrder::Special);
}

/// Test matching order with exact int and any_int
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum IntMatchOrder {
    #[compote(variant = 0)]
    Zero,
    #[compote(variant = any_int)]
    Other(i64),
}

#[test]
fn test_exact_before_wildcard_int() {
    let json = r#"0"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<IntMatchOrder>();
    assert!(
        result.is_ok(),
        "Should match exact 0 not wildcard: {:?}",
        result
    );
    assert_eq!(result.unwrap(), IntMatchOrder::Zero);

    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<IntMatchOrder>();
    assert!(result.is_ok(), "Should match wildcard for 42: {:?}", result);
    assert_eq!(result.unwrap(), IntMatchOrder::Other(42));
}

// ============================================================================
// Roundtrip Tests for New Features
// ============================================================================

#[test]
fn test_exact_string_roundtrip() {
    let original = Command::Help;
    let json = compote::to_json_compact(&original).unwrap();
    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Command>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_exact_bool_roundtrip() {
    let original = Toggle::On;
    let json = compote::to_json_compact(&original).unwrap();
    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Toggle>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_truthy_roundtrip() {
    let original = Feature::Enabled;
    let json = compote::to_json_compact(&original).unwrap();
    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Feature>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

#[test]
fn test_exact_float_roundtrip() {
    let original = Threshold::Half;
    let json = compote::to_json_compact(&original).unwrap();
    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<Threshold>();
    assert!(result.is_ok(), "Roundtrip failed: {:?}", result);
    assert_eq!(result.unwrap(), original);
}

// ============================================================================
// StringFilter with Exact "*" Test (from task description)
// ============================================================================

/// Test exact string "*" before wildcard (use case from task)
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StringFilterWithStar {
    #[compote(variant = null)]
    Any,
    #[compote(variant = "*")] // exact string "*"
    All,
    #[compote(variant = any_string)] // any other string
    Glob(String),
    Contains(String),
}

#[test]
fn test_exact_star_before_wildcard() {
    // "*" should match All, not Glob
    let json = r#""*""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StringFilterWithStar>();
    assert!(
        result.is_ok(),
        "Should match exact '*' not wildcard: {:?}",
        result
    );
    assert_eq!(result.unwrap(), StringFilterWithStar::All);
}

#[test]
fn test_glob_pattern_to_wildcard() {
    // "*.rs" should match Glob
    let json = r#""*.rs""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StringFilterWithStar>();
    assert!(result.is_ok(), "Should match Glob for '*.rs': {:?}", result);
    assert_eq!(
        result.unwrap(),
        StringFilterWithStar::Glob("*.rs".to_string())
    );
}

#[test]
fn test_null_before_star() {
    let json = r#"null"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StringFilterWithStar>();
    assert!(result.is_ok(), "Should match Any for null: {:?}", result);
    assert_eq!(result.unwrap(), StringFilterWithStar::Any);
}

#[test]
fn test_map_contains_still_works() {
    let json = r#"{"contains": "foo"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StringFilterWithStar>();
    assert!(result.is_ok(), "Should match Contains: {:?}", result);
    assert_eq!(
        result.unwrap(),
        StringFilterWithStar::Contains("foo".to_string())
    );
}

// ============================================================================
// Negative Number Tests
// ============================================================================

/// Test negative integer matching
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum NegativeNumber {
    #[compote(variant = -1)]
    MinusOne,
    #[compote(variant = 0)]
    Zero,
    #[compote(variant = 1)]
    One,
    #[compote(variant = any_int)]
    Other(i64),
}

#[test]
fn test_exact_negative_int() {
    let json = r#"-1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<NegativeNumber>();
    assert!(result.is_ok(), "Should match exact -1: {:?}", result);
    assert_eq!(result.unwrap(), NegativeNumber::MinusOne);
}

#[test]
fn test_negative_int_serialization() {
    let value = NegativeNumber::MinusOne;
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(json, "-1", "MinusOne should serialize to -1");
}

// ============================================================================
// Built-in Parameterized Predicate Tests
// ============================================================================

/// Test starts_with predicate
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum StartsWithTest {
    #[compote(variant = starts_with("run:"))]
    RunUnit,

    #[compote(variant = starts_with("exec:"))]
    RunNewtype(String),

    #[compote(variant = any_string)]
    Other(String),
}

#[test]
fn test_starts_with_unit_variant() {
    let json = r#""run:hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StartsWithTest>();
    assert!(result.is_ok(), "starts_with should match: {:?}", result);
    assert_eq!(result.unwrap(), StartsWithTest::RunUnit);
}

#[test]
fn test_starts_with_newtype_variant() {
    let json = r#""exec:ls -la""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StartsWithTest>();
    assert!(result.is_ok(), "starts_with should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        StartsWithTest::RunNewtype("exec:ls -la".to_string())
    );
}

#[test]
fn test_starts_with_fallback_to_any_string() {
    let json = r#""other command""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<StartsWithTest>();
    assert!(result.is_ok(), "any_string should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        StartsWithTest::Other("other command".to_string())
    );
}

/// Test ends_with predicate
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum EndsWithTest {
    #[compote(variant = ends_with(".rs"))]
    RustFile(String),

    #[compote(variant = ends_with(".py"))]
    PythonFile(String),

    #[compote(variant = any_string)]
    Other(String),
}

#[test]
fn test_ends_with_rust() {
    let json = r#""main.rs""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EndsWithTest>();
    assert!(result.is_ok(), "ends_with should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        EndsWithTest::RustFile("main.rs".to_string())
    );
}

#[test]
fn test_ends_with_python() {
    let json = r#""script.py""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<EndsWithTest>();
    assert!(result.is_ok(), "ends_with should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        EndsWithTest::PythonFile("script.py".to_string())
    );
}

/// Test contains predicate
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ContainsTest {
    #[compote(variant = contains("@"))]
    Email(String),

    #[compote(variant = contains("://"))]
    Url(String),

    #[compote(variant = any_string)]
    PlainText(String),
}

#[test]
fn test_contains_email() {
    let json = r#""user@example.com""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ContainsTest>();
    assert!(result.is_ok(), "contains should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ContainsTest::Email("user@example.com".to_string())
    );
}

#[test]
fn test_contains_url() {
    let json = r#""https://example.com""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ContainsTest>();
    assert!(result.is_ok(), "contains should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ContainsTest::Url("https://example.com".to_string())
    );
}

#[test]
fn test_contains_order() {
    // "user@example.com" contains both "@" and could be a URL, but "@" matches first
    let json = r#""foo@bar""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ContainsTest>();
    assert!(
        result.is_ok(),
        "contains should match @ first: {:?}",
        result
    );
    assert_eq!(result.unwrap(), ContainsTest::Email("foo@bar".to_string()));
}

/// Test range predicate with integers
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum RangeIntTest {
    #[compote(variant = range(1, 10))]
    SmallInt(i64),

    #[compote(variant = range(11, 100))]
    MediumInt(i64),

    #[compote(variant = any_int)]
    LargeInt(i64),
}

#[test]
fn test_range_small_int() {
    let json = r#"5"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeIntTest>();
    assert!(result.is_ok(), "range should match: {:?}", result);
    assert_eq!(result.unwrap(), RangeIntTest::SmallInt(5));
}

#[test]
fn test_range_medium_int() {
    let json = r#"50"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeIntTest>();
    assert!(result.is_ok(), "range should match: {:?}", result);
    assert_eq!(result.unwrap(), RangeIntTest::MediumInt(50));
}

#[test]
fn test_range_large_int() {
    let json = r#"500"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeIntTest>();
    assert!(result.is_ok(), "any_int should match: {:?}", result);
    assert_eq!(result.unwrap(), RangeIntTest::LargeInt(500));
}

#[test]
fn test_range_boundary_inclusive() {
    // Test boundary values (1 and 10 should both be included)
    let json = r#"1"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeIntTest>();
    assert!(
        result.is_ok(),
        "range should include boundary 1: {:?}",
        result
    );
    assert_eq!(result.unwrap(), RangeIntTest::SmallInt(1));

    let json2 = r#"10"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<RangeIntTest>();
    assert!(
        result2.is_ok(),
        "range should include boundary 10: {:?}",
        result2
    );
    assert_eq!(result2.unwrap(), RangeIntTest::SmallInt(10));
}

/// Test range predicate with floats
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum RangeFloatTest {
    #[compote(variant = range(0.0, 1.0))]
    Normalized(f64),

    #[compote(variant = any_float)]
    Other(f64),
}

#[test]
fn test_range_float_normalized() {
    let json = r#"0.5"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeFloatTest>();
    assert!(result.is_ok(), "range should match: {:?}", result);
    assert_eq!(result.unwrap(), RangeFloatTest::Normalized(0.5));
}

#[test]
fn test_range_float_outside() {
    let json = r#"1.5"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeFloatTest>();
    assert!(result.is_ok(), "any_float should match: {:?}", result);
    assert_eq!(result.unwrap(), RangeFloatTest::Other(1.5));
}

/// Test range predicate coerces int to float
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum RangeCoerceTest {
    #[compote(variant = range(1, 100))]
    Small(f64), // Variant takes f64, but input could be int

    #[compote(variant = any_int)]
    Other(i64),
}

#[test]
fn test_range_int_to_float_coercion() {
    let json = r#"50"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RangeCoerceTest>();
    assert!(
        result.is_ok(),
        "range should match and coerce to float: {:?}",
        result
    );
    assert_eq!(result.unwrap(), RangeCoerceTest::Small(50.0));
}

/// Test negative range values
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum NegativeRangeTest {
    #[compote(variant = range(-100, -1))]
    Negative(i64),

    #[compote(variant = range(0, 100))]
    Positive(i64),

    #[compote(variant = any_int)]
    Other(i64),
}

#[test]
fn test_negative_range() {
    let json = r#"-50"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<NegativeRangeTest>();
    assert!(result.is_ok(), "negative range should match: {:?}", result);
    assert_eq!(result.unwrap(), NegativeRangeTest::Negative(-50));
}

/// Test regex predicate
#[cfg(feature = "regex")]
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum RegexTest {
    #[compote(variant = regex(r"^v\d+\.\d+\.\d+$"))]
    SemVer(String),

    #[compote(variant = regex(r"^[a-z0-9_]+$"))]
    Identifier(String),

    #[compote(variant = any_string)]
    Other(String),
}

#[cfg(feature = "regex")]
#[test]
fn test_regex_semver() {
    let json = r#""v1.2.3""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RegexTest>();
    assert!(result.is_ok(), "regex should match semver: {:?}", result);
    assert_eq!(result.unwrap(), RegexTest::SemVer("v1.2.3".to_string()));
}

#[cfg(feature = "regex")]
#[test]
fn test_regex_identifier() {
    let json = r#""my_variable_123""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RegexTest>();
    assert!(
        result.is_ok(),
        "regex should match identifier: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        RegexTest::Identifier("my_variable_123".to_string())
    );
}

#[cfg(feature = "regex")]
#[test]
fn test_regex_no_match() {
    let json = r#""Hello World!""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<RegexTest>();
    assert!(
        result.is_ok(),
        "should fall through to any_string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        RegexTest::Other("Hello World!".to_string())
    );
}

// ============================================================================
// Custom Predicate Tests
// ============================================================================

/// Custom predicate function - checks if value is a special command
fn is_special_command<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> bool {
    matches!(value, compote::ContextValue::String(s, _) if s == "!special")
}

/// Custom predicate function - checks if value starts with !
fn is_important<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> bool {
    matches!(value, compote::ContextValue::String(s, _) if s.starts_with('!'))
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum CustomPredicateTest {
    #[compote(variant = predicate("is_special_command"))]
    Special,

    #[compote(variant = predicate("is_important"))]
    Important(String),

    #[compote(variant = any_string)]
    Normal(String),
}

#[test]
fn test_custom_predicate_unit() {
    let json = r#""!special""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CustomPredicateTest>();
    assert!(
        result.is_ok(),
        "custom predicate should match: {:?}",
        result
    );
    assert_eq!(result.unwrap(), CustomPredicateTest::Special);
}

#[test]
fn test_custom_predicate_newtype() {
    let json = r#""!important message""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CustomPredicateTest>();
    assert!(
        result.is_ok(),
        "custom predicate should match: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        CustomPredicateTest::Important("!important message".to_string())
    );
}

#[test]
fn test_custom_predicate_no_match() {
    let json = r#""normal text""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CustomPredicateTest>();
    assert!(
        result.is_ok(),
        "should fall through to any_string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        CustomPredicateTest::Normal("normal text".to_string())
    );
}

// ============================================================================
// Custom Extractor (parse) Tests
// ============================================================================

#[derive(Debug, PartialEq, serde::Serialize)]
struct CustomData {
    value: String,
    count: i64,
}

/// Custom extractor function - parses "key=value" format
fn parse_key_value<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> Option<CustomData> {
    if let compote::ContextValue::String(s, _) = value {
        let parts: Vec<&str> = s.splitn(2, '=').collect();
        if parts.len() == 2 {
            return Some(CustomData {
                value: parts[1].to_string(),
                count: parts[0].len() as i64,
            });
        }
    }
    None
}

/// Custom extractor function - parses object with "custom" field
fn parse_custom_object<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> Option<CustomData> {
    if let compote::ContextValue::Object(map, _) = value {
        if let Some(custom_val) = map.get("custom") {
            if let compote::ContextValue::String(s, _) = custom_val {
                return Some(CustomData {
                    value: s.to_string(),
                    count: s.len() as i64,
                });
            }
        }
    }
    None
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ParseExtractorTest {
    #[compote(variant = parse("parse_key_value"))]
    KeyValue(CustomData),

    #[compote(variant = parse("parse_custom_object"))]
    CustomObject(CustomData),

    #[compote(variant = any_string)]
    Plain(String),

    Named(String),
}

#[test]
fn test_parse_extractor_key_value() {
    let json = r#""name=John""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseExtractorTest>();
    assert!(result.is_ok(), "parse extractor should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ParseExtractorTest::KeyValue(CustomData {
            value: "John".to_string(),
            count: 4, // "name" has 4 chars
        })
    );
}

#[test]
fn test_parse_extractor_custom_object() {
    let json = r#"{"custom": "hello world"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseExtractorTest>();
    assert!(result.is_ok(), "parse extractor should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ParseExtractorTest::CustomObject(CustomData {
            value: "hello world".to_string(),
            count: 11,
        })
    );
}

#[test]
fn test_parse_extractor_no_match_falls_through() {
    let json = r#""plain text""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseExtractorTest>();
    assert!(
        result.is_ok(),
        "should fall through to any_string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseExtractorTest::Plain("plain text".to_string())
    );
}

#[test]
fn test_parse_extractor_map_variant_still_works() {
    let json = r#"{"named": "foo"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseExtractorTest>();
    assert!(
        result.is_ok(),
        "map variant should still work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseExtractorTest::Named("foo".to_string())
    );
}

// ============================================================================
// Serialization Tests for New Features
// ============================================================================

#[test]
fn test_starts_with_newtype_serialization() {
    let value = StartsWithTest::RunNewtype("exec:ls -la".to_string());
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(json, r#""exec:ls -la""#, "should serialize inner value");
}

#[test]
fn test_range_serialization() {
    let value = RangeIntTest::SmallInt(5);
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(json, "5", "should serialize inner value");
}

#[test]
fn test_custom_predicate_serialization() {
    let value = CustomPredicateTest::Important("!hello".to_string());
    let json = compote::to_json_compact(&value).unwrap();
    assert_eq!(json, r#""!hello""#, "should serialize inner value");
}

#[test]
fn test_parse_extractor_serialization() {
    let value = ParseExtractorTest::KeyValue(CustomData {
        value: "test".to_string(),
        count: 3,
    });
    let json = compote::to_json_compact(&value).unwrap();
    // Should serialize the CustomData struct
    assert!(
        json.contains("test"),
        "should serialize inner value: {}",
        json
    );
    assert!(json.contains("3"), "should serialize inner value: {}", json);
}

// ============================================================================
// Matching Order Tests for New Predicates
// ============================================================================

/// Test matching order: exact > truthy/falsy > builtin predicates > custom predicates > parse > wildcards
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum MatchOrderWithPredicates {
    #[compote(variant = "exact")]
    Exact,

    #[compote(variant = truthy)]
    Truthy,

    #[compote(variant = starts_with("prefix:"))]
    Prefix(String),

    #[compote(variant = predicate("is_important"))]
    Important(String),

    #[compote(variant = any_string)]
    Other(String),
}

#[test]
fn test_exact_before_starts_with() {
    // "exact" should match Exact, not any_string
    let json = r#""exact""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MatchOrderWithPredicates>();
    assert!(result.is_ok(), "exact should match: {:?}", result);
    assert_eq!(result.unwrap(), MatchOrderWithPredicates::Exact);
}

#[test]
fn test_truthy_before_starts_with() {
    // "true" should match Truthy, not starts_with
    let json = r#""true""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MatchOrderWithPredicates>();
    assert!(result.is_ok(), "truthy should match: {:?}", result);
    assert_eq!(result.unwrap(), MatchOrderWithPredicates::Truthy);
}

#[test]
fn test_starts_with_before_custom_predicate() {
    // "prefix:!hello" starts with "prefix:" so matches Prefix, not Important
    let json = r#""prefix:!hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MatchOrderWithPredicates>();
    assert!(
        result.is_ok(),
        "starts_with should match first: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        MatchOrderWithPredicates::Prefix("prefix:!hello".to_string())
    );
}

#[test]
fn test_custom_predicate_before_wildcard() {
    // "!hello" starts with ! so matches Important (custom predicate), not Other
    let json = r#""!hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<MatchOrderWithPredicates>();
    assert!(
        result.is_ok(),
        "custom predicate should match: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        MatchOrderWithPredicates::Important("!hello".to_string())
    );
}

// ============================================================================
// Complex Combined Example
// ============================================================================

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ComplexCommand {
    #[compote(variant = null)]
    Empty,

    #[compote(variant = "help" | "h" | "?")]
    Help,

    #[compote(variant = truthy)]
    Enabled,

    #[compote(variant = starts_with("run:"))]
    Run(String),

    #[compote(variant = ends_with(".sh"))]
    Script(String),

    #[compote(variant = range(1, 100))]
    Priority(i64),

    #[compote(variant = predicate("is_important"))]
    Important(String),

    #[compote(variant = any_string)]
    Custom(String),

    Config(FeatureConfig),
}

#[test]
fn test_complex_command_null() {
    let json = r#"null"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "null should match: {:?}", result);
    assert_eq!(result.unwrap(), ComplexCommand::Empty);
}

#[test]
fn test_complex_command_help() {
    for input in &[r#""help""#, r#""h""#, r#""?""#] {
        let mut config = Config::default();
        config.load_json(input, Context::new(Source::Programmatic, Level::User));
        let result = config.deserialize::<ComplexCommand>();
        assert!(result.is_ok(), "help should match {}: {:?}", input, result);
        assert_eq!(result.unwrap(), ComplexCommand::Help);
    }
}

#[test]
fn test_complex_command_truthy() {
    // Note: Integer 42 would match truthy (non-zero), so use string "yes" for truthy test
    let json = r#""yes""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "truthy should match: {:?}", result);
    assert_eq!(result.unwrap(), ComplexCommand::Enabled);
}

#[test]
fn test_complex_command_truthy_int() {
    // Non-zero integers are truthy, so they match Enabled before Priority
    // This demonstrates that truthy predicates have priority over range predicates
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "truthy should match int: {:?}", result);
    assert_eq!(result.unwrap(), ComplexCommand::Enabled);
}

#[test]
fn test_complex_command_run() {
    let json = r#""run:echo hello""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "run should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ComplexCommand::Run("run:echo hello".to_string())
    );
}

#[test]
fn test_complex_command_script() {
    let json = r#""setup.sh""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "script should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ComplexCommand::Script("setup.sh".to_string())
    );
}

/// Test enum without truthy to demonstrate range working
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum PriorityCommand {
    #[compote(variant = range(1, 100))]
    Priority(i64),

    #[compote(variant = any_int)]
    Other(i64),
}

#[test]
fn test_priority_command_range() {
    // Without truthy, range should match
    let json = r#"42"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<PriorityCommand>();
    assert!(result.is_ok(), "range should match: {:?}", result);
    assert_eq!(result.unwrap(), PriorityCommand::Priority(42));
}

#[test]
fn test_priority_command_outside_range() {
    let json = r#"200"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<PriorityCommand>();
    assert!(result.is_ok(), "any_int should match: {:?}", result);
    assert_eq!(result.unwrap(), PriorityCommand::Other(200));
}

#[test]
fn test_complex_command_important() {
    let json = r#""!urgent""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "important should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ComplexCommand::Important("!urgent".to_string())
    );
}

#[test]
fn test_complex_command_custom() {
    let json = r#""something else""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "custom should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ComplexCommand::Custom("something else".to_string())
    );
}

#[test]
fn test_complex_command_map() {
    let json = r#"{"config": {"name": "test", "enabled": true}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ComplexCommand>();
    assert!(result.is_ok(), "map should match: {:?}", result);
    assert_eq!(
        result.unwrap(),
        ComplexCommand::Config(FeatureConfig {
            name: "test".to_string(),
            enabled: true,
        })
    );
}

// ============================================================================
// Parse Extractor Tests with Multi-Field Tuple Variants
// ============================================================================

/// Custom extractor function - parses "key=value" into a tuple (key, value)
fn parse_key_value_pair<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> Option<(String, String)> {
    if let compote::ContextValue::String(s, _) = value {
        let parts: Vec<&str> = s.splitn(2, '=').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

/// Custom extractor function - parses "name:value:priority" into a 3-tuple
fn parse_triple<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> Option<(String, String, i64)> {
    if let compote::ContextValue::String(s, _) = value {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        if parts.len() == 3 {
            if let Ok(priority) = parts[2].parse::<i64>() {
                return Some((parts[0].to_string(), parts[1].to_string(), priority));
            }
        }
    }
    None
}

/// Custom extractor function - parses "a|b|c|d" into a 4-tuple
fn parse_quad<S: compote::CustomSource, L: compote::CustomLevel>(
    value: &compote::ContextValue<S, L>,
) -> Option<(String, String, String, String)> {
    if let compote::ContextValue::String(s, _) = value {
        let parts: Vec<&str> = s.splitn(4, '|').collect();
        if parts.len() == 4 {
            return Some((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].to_string(),
                parts[3].to_string(),
            ));
        }
    }
    None
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag)]
enum ParseMultiFieldTest {
    /// Four-field tuple variant - function returns Option<(T1, T2, T3, T4)>
    #[compote(variant = parse("parse_quad"))]
    FourFields(String, String, String, String),

    /// Three-field tuple variant - function returns Option<(T1, T2, T3)>
    #[compote(variant = parse("parse_triple"))]
    ThreeFields(String, String, i64),

    /// Two-field tuple variant - function returns Option<(T1, T2)>
    #[compote(variant = parse("parse_key_value_pair"))]
    TwoFields(String, String),

    /// Single-field variant (existing behavior) - must come AFTER multi-field variants
    /// because parse_key_value also matches "key=value" format
    #[compote(variant = parse("parse_key_value"))]
    SingleField(CustomData),

    #[compote(variant = any_string)]
    Fallback(String),
}

#[test]
fn test_parse_two_field_tuple() {
    let json = r#""key=value""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "parse with 2-field tuple should match: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::TwoFields("key".to_string(), "value".to_string())
    );
}

#[test]
fn test_parse_two_field_tuple_with_equals_in_value() {
    let json = r#""setting=a=b=c""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "parse with value containing = should work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::TwoFields("setting".to_string(), "a=b=c".to_string())
    );
}

#[test]
fn test_parse_three_field_tuple() {
    let json = r#""task:do_work:5""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "parse with 3-field tuple should match: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::ThreeFields("task".to_string(), "do_work".to_string(), 5)
    );
}

#[test]
fn test_parse_three_field_tuple_negative_int() {
    let json = r#""alert:warning:-10""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "parse with negative int should work: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::ThreeFields("alert".to_string(), "warning".to_string(), -10)
    );
}

#[test]
fn test_parse_four_field_tuple() {
    let json = r#""a|b|c|d""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "parse with 4-field tuple should match: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::FourFields(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        )
    );
}

#[test]
fn test_parse_multi_field_no_match_falls_through() {
    // This doesn't match any parse pattern, so falls through to Fallback
    let json = r#""just_plain_text""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "should fall through to any_string: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::Fallback("just_plain_text".to_string())
    );
}

#[test]
fn test_parse_multi_field_invalid_format_falls_through() {
    // Has colons but not enough parts for parse_triple, falls through
    let json = r#""only:two""#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ParseMultiFieldTest>();
    assert!(
        result.is_ok(),
        "invalid format should fall through: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        ParseMultiFieldTest::Fallback("only:two".to_string())
    );
}

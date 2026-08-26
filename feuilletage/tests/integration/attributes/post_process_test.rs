//! Tests for the post_process struct-level attribute.
//!
//! The `post_process` attribute allows calling a function after all fields
//! are deserialized, enabling post-processing of the parsed struct.

#![cfg(feature = "json")]

use feuilletage::{
    Config, Context, ContextValue, CustomLevel, CustomSource, Error, ErrorTracker, Level, Source,
};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Basic post_process tests
// ============================================================================

/// Post-process function that modifies a field
fn uppercase_name<S: CustomSource, L: CustomLevel>(
    config: &mut BasicPostProcess,
    _source: &ContextValue<S, L>,
    _error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    config.name = config.name.to_uppercase();
    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "uppercase_name")]
struct BasicPostProcess {
    name: String,
}

#[test]
fn test_post_process_modifies_field() {
    let json = r#"{"name": "hello"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: BasicPostProcess = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "HELLO");
}

// ============================================================================
// Post_process with path@version splitting
// ============================================================================

/// Post-process that splits path@version into separate fields
fn finalize_go_install<S: CustomSource, L: CustomLevel>(
    config: &mut UpConfigGoInstall,
    source: &ContextValue<S, L>,
    error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    // Split path@version if present
    if let Some((path, ver)) = config.path.rsplit_once('@') {
        if config.version.is_some() {
            error_tracker.record_invalid_value("version in both path and version field");
            return Err(Error::InvalidValue {
                path: error_tracker.current_path(),
                message: "conflicting version".to_string(),
            });
        }
        config.version = Some(ver.to_string());
        config.path = path.to_string();
    }

    // Conditional default: exact = version.is_some(), only if not explicitly set
    let explicitly_set = match source {
        ContextValue::Object(map, _) => map.contains_key("exact"),
        _ => false,
    };
    if !explicitly_set && config.version.is_some() {
        config.exact = true;
    }

    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "finalize_go_install")]
struct UpConfigGoInstall {
    path: String,
    #[feuilletage(default)]
    version: Option<String>,
    #[feuilletage(default)]
    exact: bool,
}

#[test]
fn test_post_process_splits_path_version() {
    let json = r#"{"path": "github.com/example/tool@v1.2.3"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UpConfigGoInstall = config.deserialize().expect("Should deserialize");
    assert_eq!(result.path, "github.com/example/tool");
    assert_eq!(result.version, Some("v1.2.3".to_string()));
    assert!(result.exact); // Auto-set because version was provided
}

#[test]
fn test_post_process_no_version_in_path() {
    let json = r#"{"path": "github.com/example/tool"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UpConfigGoInstall = config.deserialize().expect("Should deserialize");
    assert_eq!(result.path, "github.com/example/tool");
    assert_eq!(result.version, None);
    assert!(!result.exact); // Not set because no version
}

#[test]
fn test_post_process_explicit_exact_respected() {
    // When exact is explicitly set to false, post_process should not override it
    let json = r#"{"path": "github.com/example/tool@v1.0.0", "exact": false}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: UpConfigGoInstall = config.deserialize().expect("Should deserialize");
    assert_eq!(result.path, "github.com/example/tool");
    assert_eq!(result.version, Some("v1.0.0".to_string()));
    assert!(!result.exact); // Explicitly set to false, not overridden
}

#[test]
fn test_post_process_conflict_version_error() {
    // When version is in both path and field, should error
    let json = r#"{"path": "github.com/example/tool@v1.0.0", "version": "v2.0.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<UpConfigGoInstall, _> = config.deserialize();
    assert!(result.is_err());
}

// ============================================================================
// Post_process that returns error
// ============================================================================

fn validate_positive<S: CustomSource, L: CustomLevel>(
    config: &mut ValidateConfig,
    _source: &ContextValue<S, L>,
    error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    if config.value < 0 {
        return Err(Error::InvalidValue {
            path: error_tracker.current_path(),
            message: "value must be positive".to_string(),
        });
    }
    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "validate_positive")]
struct ValidateConfig {
    value: i32,
}

#[test]
fn test_post_process_returns_error() {
    let json = r#"{"value": -5}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: Result<ValidateConfig, _> = config.deserialize();
    assert!(result.is_err());
}

#[test]
fn test_post_process_passes_validation() {
    let json = r#"{"value": 10}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ValidateConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.value, 10);
}

// ============================================================================
// Post_process with error_tracker recording
// ============================================================================

fn warn_deprecated_field<S: CustomSource, L: CustomLevel>(
    config: &mut WarningConfig,
    source: &ContextValue<S, L>,
    error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    // Check if deprecated field was used
    let used_old_name = match source {
        ContextValue::Object(map, _) => map.contains_key("old_name"),
        _ => false,
    };

    if used_old_name {
        error_tracker.record_warning("old_name is deprecated. Use 'name' instead");
        // Copy old_name to name if name wasn't set
        if config.name.is_empty() {
            config.name = config.old_name.clone().unwrap_or_default();
        }
    }

    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "warn_deprecated_field")]
struct WarningConfig {
    #[feuilletage(default)]
    name: String,
    #[feuilletage(default)]
    old_name: Option<String>,
}

#[test]
fn test_post_process_records_warning() {
    let json = r#"{"old_name": "legacy_value"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: WarningConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "legacy_value"); // Copied from old_name
    assert_eq!(result.old_name, Some("legacy_value".to_string()));
}

// ============================================================================
// Post_process combined with other attributes
// ============================================================================

fn normalize_paths<S: CustomSource, L: CustomLevel>(
    config: &mut CombinedAttrsConfig,
    _source: &ContextValue<S, L>,
    _error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    // Ensure path ends with /
    if !config.base_path.ends_with('/') {
        config.base_path.push('/');
    }
    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "normalize_paths")]
struct CombinedAttrsConfig {
    #[feuilletage(default = "/tmp")]
    base_path: String,
    #[feuilletage(default = "100")]
    timeout: i32,
}

#[test]
fn test_post_process_with_defaults() {
    let json = r#"{}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: CombinedAttrsConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.base_path, "/tmp/"); // Post-processed to add /
    assert_eq!(result.timeout, 100);
}

#[test]
fn test_post_process_with_explicit_values() {
    let json = r#"{"base_path": "/home/user", "timeout": 200}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: CombinedAttrsConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.base_path, "/home/user/"); // Post-processed to add /
    assert_eq!(result.timeout, 200);
}

// ============================================================================
// Post_process with nested structs
// ============================================================================

fn set_defaults_from_nested<S: CustomSource, L: CustomLevel>(
    config: &mut NestedConfig,
    _source: &ContextValue<S, L>,
    _error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    // Use inner.name as outer name if not set
    if config.name.is_empty() {
        config.name = config.inner.label.clone();
    }
    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct InnerConfig {
    label: String,
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "set_defaults_from_nested")]
struct NestedConfig {
    #[feuilletage(default)]
    name: String,
    inner: InnerConfig,
}

#[test]
fn test_post_process_with_nested_struct() {
    let json = r#"{"inner": {"label": "nested_label"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: NestedConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "nested_label"); // Set from inner.label
    assert_eq!(result.inner.label, "nested_label");
}

#[test]
fn test_post_process_nested_with_explicit_name() {
    let json = r#"{"name": "explicit", "inner": {"label": "nested_label"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: NestedConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.name, "explicit"); // Explicit value used
    assert_eq!(result.inner.label, "nested_label");
}

// ============================================================================
// Post_process with Vec fields
// ============================================================================

fn dedupe_items<S: CustomSource, L: CustomLevel>(
    config: &mut VecConfig,
    _source: &ContextValue<S, L>,
    _error_tracker: &mut ErrorTracker,
) -> Result<(), Error> {
    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    config.items.retain(|item| seen.insert(item.clone()));
    Ok(())
}

#[derive(Debug, DeriveConfig, PartialEq)]
#[feuilletage(post_process = "dedupe_items")]
struct VecConfig {
    items: Vec<String>,
}

#[test]
fn test_post_process_dedupes_vec() {
    let json = r#"{"items": ["a", "b", "a", "c", "b", "d"]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: VecConfig = config.deserialize().expect("Should deserialize");
    assert_eq!(result.items, vec!["a", "b", "c", "d"]);
}

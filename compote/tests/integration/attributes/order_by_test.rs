//! Tests for the order_by and order_by_fn attributes with allow_map.
//!
//! These attributes control sorting after map-to-vec conversion:
//! - `order_by = "field"` - sort by field name (ascending)
//! - `order_by_fn = "fn"` - sort using custom comparison function

#![cfg(feature = "json")]

use compote::{Config, Context, Level, Source};
use compote_macros::Config as DeriveConfig;

// ============================================================================
// order_by with explicit allow_map(key = ...)
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq, Clone)]
struct Tool {
    name: String,
    #[compote(default)]
    version: Option<String>,
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct ExplicitOrderByConfig {
    #[compote(allow_map(key = "name", scalar_as = "version", order_by = "name"))]
    tools: Vec<Tool>,
}

#[test]
fn test_order_by_explicit_allow_map() {
    // Map notation input - keys are NOT in alphabetical order
    let json = r#"{"tools": {"zulu": "1.0", "alpha": "2.0", "mike": "3.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExplicitOrderByConfig = config.deserialize().expect("Should deserialize");

    // Should be sorted alphabetically by name
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].name, "alpha");
    assert_eq!(result.tools[1].name, "mike");
    assert_eq!(result.tools[2].name, "zulu");
}

#[test]
fn test_order_by_array_input_not_sorted() {
    // Array input - should NOT be sorted (order_by only applies to map notation)
    let json = r#"{"tools": [{"name": "zulu"}, {"name": "alpha"}, {"name": "mike"}]}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: ExplicitOrderByConfig = config.deserialize().expect("Should deserialize");

    // Array input preserves original order
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].name, "zulu");
    assert_eq!(result.tools[1].name, "alpha");
    assert_eq!(result.tools[2].name, "mike");
}

// ============================================================================
// order_by with flag form allow_map(order_by = ...)
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq, Clone)]
#[compote(allow_map(key = name, scalar_as = version))]
struct ToolWithTrait {
    name: String,
    #[compote(default)]
    version: Option<String>,
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct FlagFormOrderByConfig {
    // Flag form allow_map with order_by - uses inner type's AllowMapKeys trait
    #[compote(allow_map(order_by = "name"))]
    tools: Vec<ToolWithTrait>,
}

#[test]
fn test_order_by_flag_form_allow_map() {
    // Map notation input
    let json = r#"{"tools": {"zulu": "1.0", "alpha": "2.0", "mike": "3.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: FlagFormOrderByConfig = config.deserialize().expect("Should deserialize");

    // Should be sorted alphabetically by name
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].name, "alpha");
    assert_eq!(result.tools[1].name, "mike");
    assert_eq!(result.tools[2].name, "zulu");
}

// ============================================================================
// order_by_fn with custom sort function
// ============================================================================

fn sort_by_version_desc(a: &Tool, b: &Tool) -> std::cmp::Ordering {
    // Sort by version descending (None values last)
    match (&b.version, &a.version) {
        (Some(v1), Some(v2)) => v1.cmp(v2),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct OrderByFnConfig {
    #[compote(allow_map(
        key = "name",
        scalar_as = "version",
        order_by_fn = "sort_by_version_desc"
    ))]
    tools: Vec<Tool>,
}

#[test]
fn test_order_by_fn_custom_sort() {
    let json = r#"{"tools": {"alpha": "1.0", "beta": "3.0", "gamma": "2.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: OrderByFnConfig = config.deserialize().expect("Should deserialize");

    // Should be sorted by version descending: 3.0, 2.0, 1.0
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].version, Some("3.0".to_string())); // beta
    assert_eq!(result.tools[1].version, Some("2.0".to_string())); // gamma
    assert_eq!(result.tools[2].version, Some("1.0".to_string())); // alpha
}

// ============================================================================
// order_by with transparent struct
// ============================================================================

#[derive(Debug, DeriveConfig, PartialEq)]
#[compote(transparent)]
struct TransparentOrderByConfig {
    #[compote(allow_single, allow_map(order_by = "name"))]
    tools: Vec<ToolWithTrait>,
}

#[test]
fn test_order_by_transparent_map_input() {
    // Map notation directly (transparent unwraps)
    let json = r#"{"zulu": "1.0", "alpha": "2.0", "mike": "3.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransparentOrderByConfig = config.deserialize().expect("Should deserialize");

    // Should be sorted alphabetically by name
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].name, "alpha");
    assert_eq!(result.tools[1].name, "mike");
    assert_eq!(result.tools[2].name, "zulu");
}

#[test]
fn test_order_by_transparent_array_input() {
    // Array input - should NOT be sorted
    let json = r#"[{"name": "zulu"}, {"name": "alpha"}]"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransparentOrderByConfig = config.deserialize().expect("Should deserialize");

    // Array input preserves original order
    assert_eq!(result.tools.len(), 2);
    assert_eq!(result.tools[0].name, "zulu");
    assert_eq!(result.tools[1].name, "alpha");
}

#[test]
fn test_order_by_transparent_single_input() {
    // Single value (allow_single) - no sorting needed
    let json = r#"{"name": "solo", "version": "1.0"}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: TransparentOrderByConfig = config.deserialize().expect("Should deserialize");

    assert_eq!(result.tools.len(), 1);
    assert_eq!(result.tools[0].name, "solo");
}

// ============================================================================
// order_by_fn with flag form
// ============================================================================

fn sort_tools_by_name_desc(a: &ToolWithTrait, b: &ToolWithTrait) -> std::cmp::Ordering {
    b.name.cmp(&a.name)
}

#[derive(Debug, DeriveConfig, PartialEq)]
struct FlagFormOrderByFnConfig {
    #[compote(allow_map(order_by_fn = "sort_tools_by_name_desc"))]
    tools: Vec<ToolWithTrait>,
}

#[test]
fn test_order_by_fn_flag_form() {
    let json = r#"{"tools": {"alpha": "1.0", "zulu": "2.0", "mike": "3.0"}}"#;

    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result: FlagFormOrderByFnConfig = config.deserialize().expect("Should deserialize");

    // Should be sorted by name descending: zulu, mike, alpha
    assert_eq!(result.tools.len(), 3);
    assert_eq!(result.tools[0].name, "zulu");
    assert_eq!(result.tools[1].name, "mike");
    assert_eq!(result.tools[2].name, "alpha");
}

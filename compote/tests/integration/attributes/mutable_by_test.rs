//! Integration test for mutable_by enforcement during multi-file config loading.
//!
//! This test loads 4 config files in this order:
//! 1. System (first)
//! 2. User
//! 3. Workdir/Local
//! 4. System (second - tests that later system files still respect constraints)
//!
//! The test verifies that mutable_by constraints are properly enforced and
//! warnings are recorded for skipped values.
//!
//! Also includes tests for immutability constraints during merge operations.

use compote::{Config, Context, ContextValue, Format, Level, MutabilityConstraint, Source};
use compote_macros::Config as DeriveConfig;
use indexmap::IndexMap;

/// Test struct with various mutable_by constraint combinations.
///
/// This covers all meaningful combinations:
/// - No constraint (any level can set)
/// - Single level constraints
/// - Two-level constraints
/// - All three levels (equivalent to no constraint but explicit)
#[derive(DeriveConfig, Debug, PartialEq)]
struct TestConfig {
    // ========================================================================
    // No constraint - any level can modify
    // ========================================================================
    #[compote(default = "default_any")]
    any_level: String,

    // ========================================================================
    // Single level constraints
    // ========================================================================
    /// Only system level can set this
    #[compote(mutable_by = ["system"], default = "default_system_only")]
    system_only: String,

    /// Only user level can set this
    #[compote(mutable_by = ["user"], default = "default_user_only")]
    user_only: String,

    /// Only local/workdir level can set this
    #[compote(mutable_by = ["local"], default = "default_local_only")]
    local_only: String,

    // ========================================================================
    // Two-level constraints
    // ========================================================================
    /// System or user can set, but NOT local/workdir
    #[compote(mutable_by = ["system", "user"], default = "default_system_user")]
    system_or_user: String,

    /// System or local can set, but NOT user
    #[compote(mutable_by = ["system", "local"], default = "default_system_local")]
    system_or_local: String,

    /// User or local can set, but NOT system
    #[compote(mutable_by = ["user", "local"], default = "default_user_local")]
    user_or_local: String,

    // ========================================================================
    // Numeric field to test non-string types
    // ========================================================================
    /// Numeric field with user-only constraint
    #[compote(mutable_by = ["user"], default = "0")]
    user_only_number: i32,

    // ========================================================================
    // Optional field to test Option<T> behavior
    // ========================================================================
    /// Optional field with local-only constraint
    #[compote(mutable_by = ["local"])]
    local_only_optional: Option<String>,

    // ========================================================================
    // Nested object to test object merging with constraints
    // ========================================================================
    /// Nested config - the parent field has a constraint
    #[compote(mutable_by = ["user", "local"], default)]
    nested: NestedConfig,
}

#[derive(DeriveConfig, Debug, PartialEq, Default)]
struct NestedConfig {
    #[compote(default = "default_nested_value")]
    value: String,

    #[compote(default = "0")]
    count: i32,
}

#[derive(DeriveConfig, Debug, PartialEq)]
#[compote(mutable_by = ["user"])]
struct ContainerMutableConfig {
    #[compote(default = "default_inherited")]
    inherited: String,

    #[compote(rename = "renamed", default = "default_renamed")]
    inherited_renamed: String,

    #[compote(mutable_by = ["system"], default = "default_override")]
    overridden: String,
}

#[derive(DeriveConfig, Debug, Default, PartialEq)]
struct NestedContainerParent {
    #[compote(default, nested)]
    operations: NestedContainerOperations,
}

#[derive(DeriveConfig, Debug, PartialEq)]
struct NestedContainerRoot {
    #[compote(default, nested)]
    parent: NestedContainerParent,
}

#[derive(DeriveConfig, Debug, Default, PartialEq)]
#[compote(mutable_by = ["system", "user"])]
struct NestedContainerOperations {
    #[compote(default = "Vec::new()")]
    allowed: Vec<String>,
}

#[derive(DeriveConfig, Debug, Default, PartialEq)]
#[compote(mutable_by = ["user"])]
struct NestedOverrideConfig {
    #[compote(default = "user-default")]
    inherited: String,

    #[compote(mutable_by = ["local"], default = "local-default")]
    overridden: String,
}

#[derive(DeriveConfig, Debug, PartialEq)]
struct RenamedNestedParent {
    #[compote(rename = "policy", default, nested)]
    nested_config: NestedOverrideConfig,

    #[compote(default = "unrestricted-default")]
    unrestricted: String,
}

#[derive(DeriveConfig, Debug, PartialEq)]
struct FlattenedNestedParent {
    #[compote(flatten, nested)]
    nested_config: NestedOverrideConfig,
}

#[derive(DeriveConfig, Debug, Default, PartialEq)]
#[compote(scalar_as = "unrestricted")]
struct ShapeChangingNestedConfig {
    #[compote(mutable_by = ["user"], default = "protected-a-default")]
    protected_a: String,

    #[compote(mutable_by = ["user"], default = "protected-b-default")]
    protected_b: String,

    #[compote(default = "unrestricted-default")]
    unrestricted: String,
}

#[derive(DeriveConfig, Debug, Default, PartialEq)]
struct ShapeChangingParent {
    #[compote(default, nested)]
    shape: ShapeChangingNestedConfig,
}

// ============================================================================
// Test config content for each level
// ============================================================================

/// System config (loaded first) - tries to set everything
const SYSTEM_CONFIG_1: &str = r#"
any_level: "system1_any"
system_only: "system1_system_only"
user_only: "system1_user_only_SHOULD_BE_SKIPPED"
local_only: "system1_local_only_SHOULD_BE_SKIPPED"
system_or_user: "system1_system_or_user"
system_or_local: "system1_system_or_local"
user_or_local: "system1_user_or_local_SHOULD_BE_SKIPPED"
user_only_number: 100
local_only_optional: "system1_optional_SHOULD_BE_SKIPPED"
nested:
  value: "system1_nested_SHOULD_BE_SKIPPED"
  count: 10
"#;

/// User config (loaded second) - overrides some values
const USER_CONFIG: &str = r#"
any_level: "user_any"
system_only: "user_system_only_SHOULD_BE_SKIPPED"
user_only: "user_user_only"
local_only: "user_local_only_SHOULD_BE_SKIPPED"
system_or_user: "user_system_or_user"
system_or_local: "user_system_or_local_SHOULD_BE_SKIPPED"
user_or_local: "user_user_or_local"
user_only_number: 200
local_only_optional: "user_optional_SHOULD_BE_SKIPPED"
nested:
  value: "user_nested"
  count: 20
"#;

/// Workdir/Local config (loaded third) - overrides more values
const LOCAL_CONFIG: &str = r#"
any_level: "local_any"
system_only: "local_system_only_SHOULD_BE_SKIPPED"
user_only: "local_user_only_SHOULD_BE_SKIPPED"
local_only: "local_local_only"
system_or_user: "local_system_or_user_SHOULD_BE_SKIPPED"
system_or_local: "local_system_or_local"
user_or_local: "local_user_or_local"
user_only_number: 300
local_only_optional: "local_optional"
nested:
  value: "local_nested"
  count: 30
"#;

/// System config (loaded fourth/last) - tests that even late system configs respect constraints
const SYSTEM_CONFIG_2: &str = r#"
any_level: "system2_any"
system_only: "system2_system_only"
user_only: "system2_user_only_SHOULD_BE_SKIPPED"
local_only: "system2_local_only_SHOULD_BE_SKIPPED"
system_or_user: "system2_system_or_user"
system_or_local: "system2_system_or_local"
user_or_local: "system2_user_or_local_SHOULD_BE_SKIPPED"
user_only_number: 400
local_only_optional: "system2_optional_SHOULD_BE_SKIPPED"
nested:
  value: "system2_nested_SHOULD_BE_SKIPPED"
  count: 40
"#;

#[test]
fn test_mutable_by_comprehensive() {
    // Load all 4 configs in order: system, user, local, system
    let mut loader = compote::loader()
        .load_str(SYSTEM_CONFIG_1, Format::Yaml, Level::System)
        .expect("Failed to load system config 1")
        .load_str(USER_CONFIG, Format::Yaml, Level::User)
        .expect("Failed to load user config")
        .load_str(LOCAL_CONFIG, Format::Yaml, Level::Local)
        .expect("Failed to load local config")
        .load_str(SYSTEM_CONFIG_2, Format::Yaml, Level::System)
        .expect("Failed to load system config 2");

    // Deserialize with mutability enforcement
    let config: TestConfig = loader.deserialize().expect("Failed to deserialize config");

    // ========================================================================
    // Verify final values
    // ========================================================================

    // any_level: No constraint, so the highest-priority level wins (local)
    assert_eq!(
        config.any_level, "local_any",
        "any_level should be from local (highest-priority level)"
    );

    // system_only: Only system allowed, system2 is last system → system2 wins
    assert_eq!(
        config.system_only, "system2_system_only",
        "system_only should be from system2 (last system config)"
    );

    // user_only: Only user allowed, user config is the only one that can set it
    assert_eq!(
        config.user_only, "user_user_only",
        "user_only should be from user config (only allowed source)"
    );

    // local_only: Only local allowed, local config is the only one that can set it
    assert_eq!(
        config.local_only, "local_local_only",
        "local_only should be from local config (only allowed source)"
    );

    // system_or_user: System and user allowed, but NOT local; user has higher priority
    assert_eq!(
        config.system_or_user, "user_system_or_user",
        "system_or_user should be from user (highest-priority allowed level)"
    );

    // system_or_local: System and local allowed, but NOT user; local has higher priority
    assert_eq!(
        config.system_or_local, "local_system_or_local",
        "system_or_local should be from local (highest-priority allowed level)"
    );

    // user_or_local: User and local allowed, but NOT system
    // Order: (system1 skipped) → user → local → (system2 skipped)
    // local is last allowed, so local wins
    assert_eq!(
        config.user_or_local, "local_user_or_local",
        "user_or_local should be from local config (last allowed source)"
    );

    // user_only_number: Only user allowed
    // All system and local attempts should be skipped
    assert_eq!(
        config.user_only_number, 200,
        "user_only_number should be 200 from user config"
    );

    // local_only_optional: Only local allowed
    assert_eq!(
        config.local_only_optional,
        Some("local_optional".to_string()),
        "local_only_optional should be from local config"
    );

    // nested: Only user or local allowed
    // Order: (system1 skipped) → user → local → (system2 skipped)
    // local is last allowed, so local wins
    assert_eq!(
        config.nested.value, "local_nested",
        "nested.value should be from local config"
    );
    assert_eq!(
        config.nested.count, 30,
        "nested.count should be 30 from local config"
    );

    // ========================================================================
    // Verify warnings were recorded for all skipped values
    // ========================================================================

    let warnings = loader.errors().warnings();

    // Expected warnings (values that were skipped):
    // From system1:
    //   - user_only (system not allowed)
    //   - local_only (system not allowed)
    //   - user_or_local (system not allowed)
    //   - local_only_optional (system not allowed)
    //   - nested (system not allowed)
    // From user:
    //   - system_only (user not allowed)
    //   - local_only (user not allowed)
    //   - system_or_local (user not allowed)
    //   - local_only_optional (user not allowed)
    // From local:
    //   - system_only (local not allowed)
    //   - user_only (local not allowed)
    //   - system_or_user (local not allowed)
    //   - user_only_number (local not allowed)
    // From system2:
    //   - user_only (system not allowed)
    //   - local_only (system not allowed)
    //   - user_or_local (system not allowed)
    //   - local_only_optional (system not allowed)
    //   - nested (system not allowed)

    // Count warnings per field
    let _warning_messages: Vec<String> = warnings.iter().map(|w| w.to_string()).collect();

    println!("=== Recorded Warnings ({} total) ===", warnings.len());
    for (i, warning) in warnings.iter().enumerate() {
        println!("  {}: {}", i + 1, warning);
    }

    // Helper to count warnings for a field
    let count_field_warnings =
        |field: &str| -> usize { warnings.iter().filter(|w| w.path.contains(field)).count() };

    // Verify specific warnings exist

    // user_only should have warnings from system1, local, system2 (3 total)
    assert!(
        count_field_warnings("user_only") >= 3,
        "user_only should have at least 3 warnings (from system1, local, system2)"
    );

    // local_only should have warnings from system1, user, system2 (3 total)
    assert!(
        count_field_warnings("local_only") >= 3,
        "local_only should have at least 3 warnings (from system1, user, system2)"
    );

    // system_only should have warnings from user, local (2 total)
    assert!(
        count_field_warnings("system_only") >= 2,
        "system_only should have at least 2 warnings (from user, local)"
    );

    // user_or_local should have warnings from system1, system2 (2 total)
    assert!(
        count_field_warnings("user_or_local") >= 2,
        "user_or_local should have at least 2 warnings (from system1, system2)"
    );

    // system_or_user should have warning from local (1 total)
    assert!(
        count_field_warnings("system_or_user") >= 1,
        "system_or_user should have at least 1 warning (from local)"
    );

    // system_or_local should have warning from user (1 total)
    assert!(
        count_field_warnings("system_or_local") >= 1,
        "system_or_local should have at least 1 warning (from user)"
    );

    // Verify warning message format
    let has_proper_format = warnings
        .iter()
        .any(|w| w.message.contains("level ignored") && w.message.contains("allowed by:"));
    assert!(
        has_proper_format,
        "Warnings should have proper format: 'value from X level ignored (allowed by: [Y, Z])'"
    );

    // Total expected warnings: at least 15 (some fields are skipped multiple times)
    assert!(
        warnings.len() >= 15,
        "Expected at least 15 warnings, got {}",
        warnings.len()
    );

    println!("\n=== Test Passed ===");
    println!("Final config values verified correctly");
    println!("All {} expected warnings recorded", warnings.len());
}

#[test]
fn test_container_mutable_by_defaults_and_field_override() {
    let mut loader = compote::loader()
        .load_str(
            r#"
inherited: "system_inherited"
renamed: "system_renamed"
overridden: "system_override"
"#,
            Format::Yaml,
            Level::System,
        )
        .expect("failed to load system config")
        .load_str(
            r#"
inherited: "user_inherited"
renamed: "user_renamed"
overridden: "user_override"
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config");

    let config: ContainerMutableConfig = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.inherited, "user_inherited");
    assert_eq!(config.inherited_renamed, "user_renamed");
    assert_eq!(config.overridden, "system_override");

    let warnings = loader.errors().warnings();
    assert_eq!(warnings.len(), 3);
    assert!(warnings.iter().any(|warning| warning.path == "inherited"));
    assert!(warnings.iter().any(|warning| warning.path == "renamed"));
    assert!(warnings.iter().any(|warning| warning.path == "overridden"));
}

#[test]
fn test_nested_container_mutable_by_preserves_prior_allowed_value() {
    let mut loader = compote::loader()
        .load_str(
            r#"
operations:
  allowed: ["user-command"]
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config")
        .load_str(
            r#"
operations:
  allowed: ["local-command"]
"#,
            Format::Yaml,
            Level::Local,
        )
        .expect("failed to load local config");

    let config: NestedContainerParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.operations.allowed, vec!["user-command"]);
    assert!(loader
        .errors()
        .warnings()
        .iter()
        .any(|warning| warning.path == "operations.allowed"));
}

#[test]
fn test_nested_mutability_composes_through_each_opted_in_parent() {
    let mut loader = compote::loader()
        .load_str(
            r#"
parent:
  operations:
    allowed: ["user-command"]
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config")
        .load_str(
            r#"
parent:
  operations:
    allowed: ["local-command"]
"#,
            Format::Yaml,
            Level::Local,
        )
        .expect("failed to load local config");

    let config: NestedContainerRoot = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.parent.operations.allowed, vec!["user-command"]);
    assert!(loader
        .errors()
        .warnings()
        .iter()
        .any(|warning| warning.path == "parent.operations.allowed"));
}

#[test]
fn test_nested_mutability_composes_renames_overrides_and_siblings() {
    let mut loader = compote::loader()
        .load_str(
            r#"
policy:
  inherited: "from-user"
  overridden: "user-cannot-set-this"
unrestricted: "from-user"
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config")
        .load_str(
            r#"
policy:
  inherited: "local-cannot-set-this"
  overridden: "from-local"
unrestricted: "from-local"
"#,
            Format::Yaml,
            Level::Local,
        )
        .expect("failed to load local config");

    let config: RenamedNestedParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.nested_config.inherited, "from-user");
    assert_eq!(config.nested_config.overridden, "from-local");
    assert_eq!(config.unrestricted, "from-local");

    let warning_paths: Vec<_> = loader
        .errors()
        .warnings()
        .iter()
        .map(|warning| warning.path.as_str())
        .collect();
    assert!(warning_paths.contains(&"policy.overridden"));
    assert!(warning_paths.contains(&"policy.inherited"));
}

#[test]
fn test_nested_mutability_composes_flattened_paths() {
    let mut loader = compote::loader()
        .load_str(
            r#"
inherited: "from-user"
overridden: "user-cannot-set-this"
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config")
        .load_str(
            r#"
inherited: "local-cannot-set-this"
overridden: "from-local"
"#,
            Format::Yaml,
            Level::Local,
        )
        .expect("failed to load local config");

    let config: FlattenedNestedParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.nested_config.inherited, "from-user");
    assert_eq!(config.nested_config.overridden, "from-local");
    assert!(loader
        .errors()
        .warnings()
        .iter()
        .any(|warning| warning.path == "inherited"));
}

#[test]
fn test_nested_mutability_blocks_object_to_scalar_shape_change() {
    let mut loader = compote::loader()
        .load_str(
            r#"
shape:
  protected_a: "from-user-a"
  protected_b: "from-user-b"
  unrestricted: "from-user-open"
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load user config")
        .load_str("shape: from-local-scalar", Format::Yaml, Level::Local)
        .expect("failed to load local config");

    let config: ShapeChangingParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.shape.protected_a, "from-user-a");
    assert_eq!(config.shape.protected_b, "from-user-b");
    assert_eq!(config.shape.unrestricted, "from-user-open");

    let mut warning_paths: Vec<_> = loader
        .errors()
        .warnings()
        .iter()
        .map(|warning| warning.path.as_str())
        .collect();
    warning_paths.sort_unstable();
    assert_eq!(warning_paths, ["shape.protected_a", "shape.protected_b"]);
}

#[test]
fn test_nested_mutability_filters_scalar_to_object_shape_change() {
    let mut loader = compote::loader()
        .load_str("shape: from-local-scalar", Format::Yaml, Level::Local)
        .expect("failed to load initial local config")
        .load_str(
            r#"
shape:
  protected_a: "local-cannot-set-a"
  protected_b: "local-cannot-set-b"
  unrestricted: "from-local-object"
"#,
            Format::Yaml,
            Level::Local,
        )
        .expect("failed to load replacement local config");

    let config: ShapeChangingParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.shape.protected_a, "protected-a-default");
    assert_eq!(config.shape.protected_b, "protected-b-default");
    assert_eq!(config.shape.unrestricted, "from-local-object");

    let mut warning_paths: Vec<_> = loader
        .errors()
        .warnings()
        .iter()
        .map(|warning| warning.path.as_str())
        .collect();
    warning_paths.sort_unstable();
    assert_eq!(warning_paths, ["shape.protected_a", "shape.protected_b"]);
}

#[test]
fn test_nested_mutability_allows_authorized_shape_change_without_warning() {
    let mut loader = compote::loader()
        .load_str(
            r#"
shape:
  protected_a: "first-a"
  protected_b: "first-b"
  unrestricted: "first-open"
"#,
            Format::Yaml,
            Level::User,
        )
        .expect("failed to load initial user config")
        .load_str("shape: replacement", Format::Yaml, Level::User)
        .expect("failed to load replacement user config");

    let config: ShapeChangingParent = loader.deserialize().expect("failed to deserialize");

    assert_eq!(config.shape.protected_a, "protected-a-default");
    assert_eq!(config.shape.protected_b, "protected-b-default");
    assert_eq!(config.shape.unrestricted, "replacement");
    assert!(loader.errors().warnings().is_empty());
}

/// Test that deserialize_unconstrained skips merge-level filtering but not macro validation.
///
/// Note: `deserialize_unconstrained` bypasses MERGE-level mutability enforcement
/// (no values are filtered out, no warnings recorded). However, the macro-generated
/// deserialization code still validates mutable_by constraints, which will fail.
///
/// This test verifies the current behavior. For a struct without mutable_by
/// constraints, deserialize_unconstrained would work normally.
#[test]
fn test_deserialize_unconstrained_skips_merge_filtering() {
    // Create a config with only unconstrained fields
    let config_str = r#"
any_level: "test_value"
"#;

    let mut loader = compote::loader()
        .load_str(config_str, Format::Yaml, Level::System)
        .expect("Failed to load system config");

    // deserialize_unconstrained should work for unconstrained fields
    let config: TestConfig = loader
        .deserialize_unconstrained()
        .expect("Failed to deserialize config");

    // The unconstrained field should have the value
    assert_eq!(config.any_level, "test_value");

    // Other fields should have their defaults
    assert_eq!(config.user_only, "default_user_only");
    assert_eq!(config.local_only, "default_local_only");

    // No merge-level warnings should be recorded since we skipped that step
    // (the warnings come from merge, not from macro validation)
    let warnings = loader.errors().warnings();
    assert!(
        warnings.is_empty(),
        "deserialize_unconstrained should skip merge-level filtering (no warnings), got {} warnings",
        warnings.len()
    );
}

/// Test that empty configs don't cause issues
#[test]
fn test_mutable_by_with_partial_configs() {
    // Only set a few values in each config
    let system_partial = r#"
any_level: "sys_partial"
system_only: "sys_system_only"
"#;

    let user_partial = r#"
user_only: "user_partial"
"#;

    let mut loader = compote::loader()
        .load_str(system_partial, Format::Yaml, Level::System)
        .expect("Failed to load system config")
        .load_str(user_partial, Format::Yaml, Level::User)
        .expect("Failed to load user config");

    let config: TestConfig = loader.deserialize().expect("Failed to deserialize");

    // Verify values that were set
    assert_eq!(config.any_level, "sys_partial");
    assert_eq!(config.system_only, "sys_system_only");
    assert_eq!(config.user_only, "user_partial");

    // Verify defaults for unset fields
    assert_eq!(config.local_only, "default_local_only");
    assert_eq!(config.user_or_local, "default_user_local");
}

/// Test that warnings include correct level information
#[test]
fn test_warning_level_information() {
    let system_config = r#"
user_only: "from_system"
"#;

    let mut loader = compote::loader()
        .load_str(system_config, Format::Yaml, Level::System)
        .expect("Failed to load system config");

    let _config: TestConfig = loader.deserialize().expect("Failed to deserialize");

    let warnings = loader.errors().warnings();
    assert_eq!(warnings.len(), 1, "Expected exactly 1 warning");

    let warning = &warnings[0];
    assert!(
        warning.message.contains("system"),
        "Warning should mention 'system' level: {}",
        warning.message
    );
    assert!(
        warning.message.contains("user"),
        "Warning should mention 'user' as allowed level: {}",
        warning.message
    );
}

// ============================================================================
// Test interaction between mutable_by warnings and error tracking
// ============================================================================

/// Test that both merge warnings (mutable_by) and deserialization errors are accessible
///
/// This test verifies that the error tracking system works correctly when
/// mutable_by constraints generate warnings during merge.
#[test]
fn test_mutable_by_warnings_accessible_after_deserialize() {
    // Config that will trigger mutable_by warning (system trying to set user_only field)
    let system_config = r#"
any_level: "from_system"
user_only: "system_trying_to_set_user_only"
"#;

    let mut loader = compote::loader()
        .load_str(system_config, Format::Yaml, Level::System)
        .expect("Failed to load config");

    let _config: TestConfig = loader
        .deserialize()
        .expect("Deserialization should succeed");

    // Check for mutable_by warning
    let warnings = loader.errors().warnings();
    println!("=== mutable_by Warnings ({}) ===", warnings.len());
    for (i, warning) in warnings.iter().enumerate() {
        println!("  {}: {}", i + 1, warning);
    }

    assert!(
        warnings.iter().any(|w| w.path.contains("user_only")),
        "Expected mutable_by warning for 'user_only' field"
    );

    // Deserialization errors (if any) should also be accessible
    let errors = loader.errors().errors();
    println!("=== Deserialization Errors ({}) ===", errors.len());

    // In this case, no deserialization errors since all types match
    // The key point is that BOTH warnings() and errors() are accessible after deserialize()
}

// ============================================================================
// Immutability tests (from integration_test.rs)
// ============================================================================

/// Test that immutable fields cannot be overridden during merge
#[test]
fn test_merge_with_immutability() {
    let mut config = Config::default();

    // Load initial config with immutable field
    let mut obj = IndexMap::new();
    obj.insert(
        "name".to_string(),
        ContextValue::string(
            "original",
            Context::new(Source::Programmatic, Level::System)
                .with_mutability_constraint(MutabilityConstraint::Immutable),
        ),
    );
    obj.insert(
        "count".to_string(),
        ContextValue::int(10, Context::new(Source::Programmatic, Level::System)),
    );

    let immutable_config =
        ContextValue::object(obj, Context::new(Source::Programmatic, Level::System));

    config.merge(immutable_config);

    // Try to override
    let json2 = r#"{"name": "changed", "count": 20}"#;
    config.load_json(json2, Context::new(Source::Programmatic, Level::User));

    // Name should still be original (immutable), but count should be updated
    if let compote::ContextValue::Object(map, _) = config.root() {
        if let Some(name) = map.get("name") {
            if let compote::ContextValue::String(s, _) = name {
                assert_eq!(s, "original", "Immutable field should not change");
            }
        }
        if let Some(count) = map.get("count") {
            if let compote::ContextValue::Int(i, _) = count {
                assert_eq!(*i, 20, "Mutable field should change");
            }
        }
    }

    // Should have recorded an immutability error
    assert!(config.has_errors());
}

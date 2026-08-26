//! Mutable By Pattern: Scope filtering / Level restrictions
//!
//! Common use cases:
//! - Security-sensitive configs (only system can set)
//! - Per-project settings (only workdir can override)
//! - Admin settings (only specific levels allowed)
//! - Multi-tenant configs with permission levels
//!
//! Compote Solution: `#[compote(mutable_by = ["level1", "level2"])]`
//!
//! Example:
//! ```yaml
//! # System level only - cannot be overridden by user/local configs
//! root_access: true
//!
//! # System or user level - local configs cannot override
//! auto_update: false
//! ```

use compote::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Config with level restrictions
#[derive(Debug, compote::Config, PartialEq)]
struct SecurityConfig {
    /// Can only be set by system level
    #[compote(mutable_by = ["system"], default = "false")]
    root_access: bool,

    /// Can be set by system or user level (not workdir)
    #[compote(mutable_by = ["system", "user"], default = "true")]
    auto_update: bool,

    /// No restrictions - any level can set
    #[compote(default = "default_name")]
    name: String,
}

/// Helper to deserialize with specific level
fn deserialize_at_level<T: FromContextValue>(json: &str, level: Level) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, level));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Mutable By Examples ===\n");

    // System level can set everything
    println!("--- System Level (full access) ---");
    let json = r#"{"root_access": true, "auto_update": false}"#;
    let config: SecurityConfig =
        deserialize_at_level(json, Level::System).expect("System level should be allowed");
    println!("root_access: {}", config.root_access);
    println!("auto_update: {}", config.auto_update);
    assert!(config.root_access);
    assert!(!config.auto_update);

    // User level denied for root_access
    println!("\n--- User Level (denied root_access) ---");
    let json = r#"{"root_access": true}"#;
    let result = deserialize_at_level::<SecurityConfig>(json, Level::User);
    match &result {
        Ok(_) => println!("Unexpected success"),
        Err(e) => println!(
            "Error (expected): {}",
            format!("{:?}", e)
                .split("can only be set by levels")
                .next()
                .unwrap_or("")
        ),
    }
    assert!(
        result.is_err(),
        "User level should be denied for root_access"
    );

    // Local level denied for root_access
    println!("\n--- Local Level (denied root_access) ---");
    let json = r#"{"root_access": true}"#;
    let result = deserialize_at_level::<SecurityConfig>(json, Level::Local);
    assert!(
        result.is_err(),
        "Local level should be denied for root_access"
    );
    println!("Local denied as expected");

    // User level allowed for auto_update
    println!("\n--- User Level (allowed auto_update) ---");
    let json = r#"{"auto_update": false}"#;
    let config: SecurityConfig = deserialize_at_level(json, Level::User)
        .expect("User level should be allowed for auto_update");
    println!("auto_update: {}", config.auto_update);
    assert!(!config.auto_update);

    // Local level denied for auto_update
    println!("\n--- Local Level (denied auto_update) ---");
    let json = r#"{"auto_update": false}"#;
    let result = deserialize_at_level::<SecurityConfig>(json, Level::Local);
    assert!(
        result.is_err(),
        "Local level should be denied for auto_update"
    );
    println!("Local denied as expected");

    // Unrestricted field any level
    println!("\n--- Unrestricted Field (any level) ---");
    for level in [Level::System, Level::User, Level::Local] {
        let json = r#"{"name": "custom"}"#;
        let config: SecurityConfig = deserialize_at_level(json, level.clone())
            .unwrap_or_else(|_| panic!("{:?} should be allowed for name", level));
        println!("{:?}: name={}", level, config.name);
        assert_eq!(config.name, "custom");
    }

    // Default values when not provided
    println!("\n--- Default Values ---");
    let json = r#"{"name": "test"}"#;
    let config: SecurityConfig =
        deserialize_at_level(json, Level::Local).expect("Should use defaults");
    println!("root_access: {} (default)", config.root_access);
    println!("auto_update: {} (default)", config.auto_update);
    assert!(!config.root_access);
    assert!(config.auto_update);

    // Vec fields with mutable_by
    println!("\n--- Vec Fields with Mutable By ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct PathConfig {
        #[compote(allow_single, mutable_by = ["system", "user"])]
        trusted_paths: Vec<String>,

        #[compote(allow_single)]
        project_paths: Vec<String>,
    }

    let json = r#"{"trusted_paths": ["/usr/bin", "/opt/bin"], "project_paths": ["./bin"]}"#;
    let config: PathConfig =
        deserialize_at_level(json, Level::User).expect("User should be allowed");
    println!("trusted_paths: {:?}", config.trusted_paths);
    println!("project_paths: {:?}", config.project_paths);
    assert_eq!(config.trusted_paths.len(), 2);
    assert_eq!(config.project_paths.len(), 1);

    let json = r#"{"trusted_paths": ["/tmp/evil"]}"#;
    let result = deserialize_at_level::<PathConfig>(json, Level::Local);
    assert!(
        result.is_err(),
        "Local should not be allowed to set trusted_paths"
    );
    println!("Local denied for trusted_paths as expected");

    // Real-world: Up Command Config
    println!("\n--- Real-World: Up Command Config ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct UpCommandConfig {
        #[compote(mutable_by = ["system", "user"], default = "true")]
        auto_bootstrap: bool,

        #[compote(mutable_by = ["system"], default = "false")]
        trust_all_repos: bool,

        #[compote(duration, default = "300")]
        timeout: i64,

        #[compote(default)]
        custom_steps: Vec<String>,
    }

    let json = r#"{
        "auto_bootstrap": true,
        "trust_all_repos": true,
        "timeout": "10m",
        "custom_steps": ["custom.sh"]
    }"#;
    let config: UpCommandConfig =
        deserialize_at_level(json, Level::System).expect("System should be allowed for all fields");
    println!("System level:");
    println!("  auto_bootstrap: {}", config.auto_bootstrap);
    println!("  trust_all_repos: {}", config.trust_all_repos);
    println!("  timeout: {}s", config.timeout);
    println!("  custom_steps: {:?}", config.custom_steps);
    assert!(config.auto_bootstrap);
    assert!(config.trust_all_repos);
    assert_eq!(config.timeout, 600);
    assert_eq!(config.custom_steps, vec!["custom.sh".to_string()]);

    // User level - partial access
    println!("\nUser level (partial access):");
    let json = r#"{"auto_bootstrap": false, "timeout": "5m"}"#;
    let config: UpCommandConfig = deserialize_at_level(json, Level::User)
        .expect("User should be allowed for auto_bootstrap and timeout");
    println!("  auto_bootstrap: {}", config.auto_bootstrap);
    println!("  trust_all_repos: {} (default)", config.trust_all_repos);
    println!("  timeout: {}s", config.timeout);
    assert!(!config.auto_bootstrap);
    assert!(!config.trust_all_repos); // default
    assert_eq!(config.timeout, 300);

    // User denied trust_all_repos
    let json = r#"{"trust_all_repos": true}"#;
    let result = deserialize_at_level::<UpCommandConfig>(json, Level::User);
    assert!(
        result.is_err(),
        "User should not be allowed to set trust_all_repos"
    );
    println!("  trust_all_repos: denied for user level");

    // Local level - only unrestricted
    println!("\nLocal level (only unrestricted):");
    let json = r#"{"timeout": "2m", "custom_steps": ["local-step.sh"]}"#;
    let config: UpCommandConfig = deserialize_at_level(json, Level::Local)
        .expect("Local should be allowed for unrestricted fields");
    println!("  auto_bootstrap: {} (default)", config.auto_bootstrap);
    println!("  trust_all_repos: {} (default)", config.trust_all_repos);
    println!("  timeout: {}s", config.timeout);
    println!("  custom_steps: {:?}", config.custom_steps);
    assert!(config.auto_bootstrap); // default
    assert!(!config.trust_all_repos); // default
    assert_eq!(config.timeout, 120);
    assert_eq!(config.custom_steps, vec!["local-step.sh".to_string()]);

    // Merging respects mutable_by
    println!("\n--- Merging Respects Mutable By ---");
    let mut config = ConfigContainer::default();
    config.load_json(
        r#"{"root_access": true, "auto_update": true, "name": "system"}"#,
        Context::new(Source::Programmatic, Level::System),
    );
    config.load_json(
        r#"{"auto_update": false, "name": "user"}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    let result: SecurityConfig = config.deserialize().expect("Should deserialize");
    println!("After merge:");
    println!("  root_access: {} (from system)", result.root_access);
    println!("  auto_update: {} (changed by user)", result.auto_update);
    println!("  name: {} (changed by user)", result.name);
    assert!(
        result.root_access,
        "root_access should remain from system level"
    );
    assert!(!result.auto_update, "auto_update should be changed by user");
    assert_eq!(result.name, "user", "name should be changed by user");

    println!("\n=== All mutable_by examples passed! ===");
}

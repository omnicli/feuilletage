//! Path Resolution Pattern: Relative path transforms
//!
//! Common use cases:
//! - Config file paths relative to the config file location
//! - Template paths for static site generators
//! - Include paths for build systems
//! - Asset paths for web applications
//!
//! Feuilletage Solution: `#[feuilletage(relative_path)]` or `#[feuilletage(transform = "relative_path")]`
//!
//! Example:
//! ```yaml
//! # In /project/config.yaml:
//! template_path: "templates/main.txt"  # Resolved to /project/templates/main.txt
//! config_path: "/absolute/path"        # Unchanged (already absolute)
//! ```

use feuilletage::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};
use std::path::PathBuf;

/// Config with path fields
#[derive(Debug, feuilletage::Config, PartialEq)]
struct PathConfig {
    #[feuilletage(relative_path)]
    template_path: String,

    // Regular path (not relative)
    config_path: String,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Path Resolution Examples ===\n");

    // Programmatic source - no transformation
    println!("--- Programmatic Source (no transformation) ---");
    let json = r#"{
        "template_path": "templates/main.txt",
        "config_path": "/absolute/path"
    }"#;
    let config: PathConfig = deserialize_json(json).expect("Should deserialize paths");
    println!("template_path: {} (unchanged)", config.template_path);
    println!("config_path: {}", config.config_path);
    assert_eq!(config.template_path, "templates/main.txt");
    assert_eq!(config.config_path, "/absolute/path");

    // Absolute path stays absolute
    println!("\n--- Absolute Path Stays Absolute ---");
    let json = r#"{
        "template_path": "/usr/share/templates/main.txt",
        "config_path": "/etc/config"
    }"#;
    let config: PathConfig = deserialize_json(json).expect("Should deserialize absolute paths");
    println!("template_path: {}", config.template_path);
    println!("config_path: {}", config.config_path);
    assert!(config.template_path.starts_with('/'));
    assert!(config.config_path.starts_with('/'));

    // File source - relative path resolution
    println!("\n--- File Source (relative path resolution) ---");
    let mut config = ConfigContainer::default();
    let json = r#"{
        "template_path": "templates/main.txt",
        "config_path": "/absolute/path"
    }"#;
    let source = Source::File(PathBuf::from("/home/user/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: PathConfig = config
        .deserialize()
        .expect("Should deserialize with file source");
    println!("Source file: /home/user/project/.config.yaml");
    println!("template_path: {} (resolved)", cfg.template_path);
    println!("config_path: {} (unchanged)", cfg.config_path);
    assert_eq!(cfg.template_path, "/home/user/project/templates/main.txt");
    assert_eq!(cfg.config_path, "/absolute/path");

    // Absolute path not transformed
    println!("\n--- Absolute Path Not Transformed ---");
    let mut config = ConfigContainer::default();
    let json = r#"{
        "template_path": "/absolute/templates/main.txt",
        "config_path": "/etc/config"
    }"#;
    let source = Source::File(PathBuf::from("/home/user/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: PathConfig = config.deserialize().expect("Should deserialize");
    println!("template_path: {} (already absolute)", cfg.template_path);
    assert_eq!(cfg.template_path, "/absolute/templates/main.txt");

    // PathBuf fields
    println!("\n--- PathBuf Fields ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct PathBufConfig {
        #[feuilletage(relative_path)]
        script_path: PathBuf,

        #[feuilletage(absolute_path)]
        log_path: PathBuf,
    }

    let json = r#"{
        "script_path": "scripts/build.sh",
        "log_path": "/var/log/app.log"
    }"#;

    let mut config = ConfigContainer::default();
    let source = Source::File(PathBuf::from("/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: PathBufConfig = config
        .deserialize()
        .expect("Should deserialize PathBuf fields");
    println!("script_path: {:?}", cfg.script_path);
    println!("log_path: {:?}", cfg.log_path);
    assert_eq!(cfg.script_path, PathBuf::from("/project/scripts/build.sh"));
    assert_eq!(cfg.log_path, PathBuf::from("/var/log/app.log"));

    // Absolute path validation
    println!("\n--- Absolute Path Validation ---");
    let json = r#"{
        "script_path": "scripts/build.sh",
        "log_path": "relative/path.log"
    }"#;
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<PathBufConfig>();
    match &result {
        Ok(_) => println!("Unexpected success"),
        Err(_) => println!("Error (expected): validation failed for non-absolute path"),
    }
    assert!(
        result.is_err(),
        "Should fail for non-absolute path with absolute_path attribute"
    );

    // Optional path fields
    println!("\n--- Optional Path Fields ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct OptionalPathConfig {
        #[feuilletage(relative_path)]
        main_template: Option<String>,

        #[feuilletage(relative_path)]
        fallback_template: Option<String>,
    }

    let mut config = ConfigContainer::default();
    let json = r#"{"main_template": "templates/main.txt"}"#;
    let source = Source::File(PathBuf::from("/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: OptionalPathConfig = config.deserialize().expect("Should deserialize");
    println!("main_template: {:?}", cfg.main_template);
    println!("fallback_template: {:?}", cfg.fallback_template);
    assert_eq!(
        cfg.main_template,
        Some("/project/templates/main.txt".to_string())
    );
    assert_eq!(cfg.fallback_template, None);

    // Vec path fields
    println!("\n--- Vec Path Fields ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct VecPathConfig {
        #[feuilletage(allow_single, transform_each = "relative_path")]
        include_paths: Vec<String>,
    }

    let mut config = ConfigContainer::default();
    let json = r#"{"include_paths": ["includes/a.txt", "includes/b.txt"]}"#;
    let source = Source::File(PathBuf::from("/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: VecPathConfig = config.deserialize().expect("Should deserialize vec paths");
    println!("include_paths: {:?}", cfg.include_paths);
    assert_eq!(cfg.include_paths.len(), 2);
    assert_eq!(cfg.include_paths[0], "/project/includes/a.txt");
    assert_eq!(cfg.include_paths[1], "/project/includes/b.txt");

    // Single value
    let mut config = ConfigContainer::default();
    let json = r#"{"include_paths": "includes/single.txt"}"#;
    let source = Source::File(PathBuf::from("/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: VecPathConfig = config
        .deserialize()
        .expect("Should deserialize single path");
    println!("include_paths (single): {:?}", cfg.include_paths);
    assert_eq!(
        cfg.include_paths,
        vec!["/project/includes/single.txt".to_string()]
    );

    // Real-world: Project Config
    println!("\n--- Real-World: Project Config ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ProjectConfig {
        name: String,

        #[feuilletage(relative_path, default = "Makefile")]
        makefile: String,

        #[feuilletage(relative_path, default = "templates")]
        template_dir: String,

        #[feuilletage(relative_path)]
        assets_dir: Option<String>,

        #[feuilletage(allow_single, transform_each = "relative_path")]
        extra_configs: Vec<String>,
    }

    let mut config = ConfigContainer::default();
    let json = r#"{"name": "my-project"}"#;
    let source = Source::File(PathBuf::from("/home/user/my-project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: ProjectConfig = config
        .deserialize()
        .expect("Should deserialize project config");
    println!("name: {}", cfg.name);
    println!("makefile: {} (default, not transformed)", cfg.makefile);
    println!(
        "template_dir: {} (default, not transformed)",
        cfg.template_dir
    );
    println!("assets_dir: {:?}", cfg.assets_dir);
    assert_eq!(cfg.name, "my-project");
    // Note: Default values are NOT transformed by relative_path
    assert_eq!(cfg.makefile, "Makefile");
    assert_eq!(cfg.template_dir, "templates");
    assert!(cfg.assets_dir.is_none());

    // Full config
    let mut config = ConfigContainer::default();
    let json = r#"{
        "name": "my-project",
        "makefile": "build/Makefile",
        "template_dir": "src/templates",
        "assets_dir": "public/assets",
        "extra_configs": ["config/dev.yaml", "config/prod.yaml"]
    }"#;
    let source = Source::File(PathBuf::from("/project/.config.yaml"));
    config.load_json(json, Context::new(source, Level::User));

    let cfg: ProjectConfig = config
        .deserialize()
        .expect("Should deserialize full project config");
    println!("\nFull config:");
    println!("name: {}", cfg.name);
    println!("makefile: {}", cfg.makefile);
    println!("template_dir: {}", cfg.template_dir);
    println!("assets_dir: {:?}", cfg.assets_dir);
    println!("extra_configs: {:?}", cfg.extra_configs);
    assert_eq!(cfg.name, "my-project");
    assert_eq!(cfg.makefile, "/project/build/Makefile");
    assert_eq!(cfg.template_dir, "/project/src/templates");
    assert_eq!(cfg.assets_dir, Some("/project/public/assets".to_string()));
    assert_eq!(
        cfg.extra_configs,
        vec![
            "/project/config/dev.yaml".to_string(),
            "/project/config/prod.yaml".to_string(),
        ]
    );

    println!("\n=== All path resolution examples passed! ===");
}

//! Layered Configuration Pattern: Multi-source config with priority levels
//!
//! Common use cases:
//! - System defaults + user preferences + project overrides
//! - Enterprise policies + team settings + local development
//! - Package defaults + workspace config + environment overrides
//!
//! Compote Solution: Level enum (System < User < Local)
//!
//! Example:
//! ```yaml
//! # /etc/myapp/config.yaml (System level - lowest priority)
//! database:
//!   host: localhost
//!   port: 5432
//!
//! # ~/.config/myapp/config.yaml (User level - medium priority)
//! database:
//!   host: db.example.com  # Overrides system
//!
//! # ./config.local.yaml (Local level - highest priority)
//! database:
//!   host: localhost       # Development override
//!   name: myapp_dev       # New field merged in
//! ```

use compote::{Config, Context, Level, Source};
use std::path::PathBuf;

fn main() {
    println!("=== Layered Configuration Example ===\n");
    println!("Demonstrates loading configuration from multiple sources");
    println!("with different priority levels.\n");

    let mut config = Config::default();

    // 1. System-level configuration (lowest priority)
    println!("--- System Level (lowest priority) ---");
    let system_config = r#"
    {
        "app_name": "MyApp",
        "database": {
            "host": "localhost",
            "port": 5432,
            "pool_size": 10
        },
        "logging": {
            "level": "info",
            "format": "text"
        },
        "features": {
            "telemetry": true,
            "debug_mode": false
        }
    }
    "#;

    let system_context = Context::new(
        Source::File(PathBuf::from("/etc/myapp/config.json")),
        Level::System,
    );
    config.load_json(system_config, system_context);
    println!("  Source: /etc/myapp/config.json");
    println!("  Loaded: app_name, database, logging, features (base defaults)");

    // 2. User-level configuration (medium priority)
    println!("\n--- User Level (medium priority) ---");
    let user_config = r#"
    {
        "database": {
            "host": "db.example.com",
            "pool_size": 20
        },
        "logging": {
            "level": "debug"
        },
        "user_preferences": {
            "theme": "dark",
            "notifications": true
        }
    }
    "#;

    let user_context = Context::new(
        Source::File(PathBuf::from("~/.config/myapp/config.json")),
        Level::User,
    );
    config.load_json(user_config, user_context);
    println!("  Source: ~/.config/myapp/config.json");
    println!("  Updated: database.host -> 'db.example.com'");
    println!("  Updated: database.pool_size -> 20");
    println!("  Updated: logging.level -> 'debug'");
    println!("  Added: user_preferences section");

    // 3. Local/project-level configuration (highest priority)
    println!("\n--- Local Level (highest priority) ---");
    let local_config = r#"
    {
        "database": {
            "host": "localhost",
            "name": "myapp_dev"
        },
        "logging": {
            "level": "trace",
            "output": "file"
        },
        "features": {
            "debug_mode": true
        }
    }
    "#;

    let local_context = Context::new(
        Source::File(PathBuf::from("./config.local.json")),
        Level::Local,
    );
    config.load_json(local_config, local_context);
    println!("  Source: ./config.local.json");
    println!("  Updated: database.host -> 'localhost' (dev override)");
    println!("  Added: database.name -> 'myapp_dev'");
    println!("  Updated: logging.level -> 'trace'");
    println!("  Added: logging.output -> 'file'");
    println!("  Updated: features.debug_mode -> true");

    // 4. Display final merged configuration
    println!("\n--- Final Merged Configuration ---");
    println!("{:#?}", config.root());

    // 5. Check for any errors
    if config.has_errors() {
        println!("\nWarnings/Errors during configuration loading:");
        for error in config.get_errors() {
            println!("  - {}", error);
        }
    } else {
        println!("\nAll configurations merged successfully!");
    }

    // Summary
    println!("\n--- Summary ---");
    println!("Configuration was loaded from three levels:");
    println!("  System -> /etc/myapp/config.json (base defaults)");
    println!("  User   -> ~/.config/myapp/config.json (user preferences)");
    println!("  Local  -> ./config.local.json (development overrides)");
    println!("\nMerge behavior:");
    println!("  - Objects merge recursively (fields from all levels combined)");
    println!("  - Primitives use last-write-wins (higher priority overrides)");
    println!("  - New fields from any level are added to the result");
    println!("\n=== Layered configuration example completed! ===");
}

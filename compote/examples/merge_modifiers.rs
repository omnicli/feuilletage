//! Merge Modifiers Pattern: Custom merge behavior with special suffixes
//!
//! Common use cases:
//! - Preserving lower-priority values (don't override defaults)
//! - Appending items to arrays (plugins, middleware, paths)
//! - Prepending items to arrays (priority ordering)
//! - Full replacement of objects (discard inherited fields)
//!
//! Compote Solution: Key suffixes `__tokeep`, `__toappend`, `__toprepend`, `__toreplace`
//!
//! Example:
//! ```yaml
//! # __tokeep - Only set if not already present
//! port__tokeep: 3000      # Won't override existing port value
//!
//! # __toappend - Add to end of array
//! tags__toappend: "go"    # Adds "go" to existing tags array
//!
//! # __toprepend - Add to beginning of array
//! middleware__toprepend: "cors"  # Adds "cors" at start
//!
//! # __toreplace - Full replacement (discards existing fields)
//! server__toreplace:
//!   host: "0.0.0.0"       # Replaces entire server object
//! ```

use compote::{Config, Context, Level, Source};

fn main() {
    let mut config = Config::default();
    let context = Context::new(Source::Programmatic, Level::User);

    println!("=== Merge Modifiers Examples ===\n");

    // 1. __tokeep - Only set if not present
    println!("--- __tokeep Modifier ---");
    println!("Set a value only if the key doesn't already exist.\n");

    config.load_json(r#"{"port": 8080}"#, context.clone());
    println!("  Step 1: Load port=8080");

    config.load_json(r#"{"port__tokeep": 3000}"#, context.clone());
    println!("  Step 2: Load port__tokeep=3000");
    println!("  Result: port remains 8080 (not overridden)");
    println!("  Use case: Setting defaults that shouldn't override user values\n");

    // 2. __toappend - Append to array
    println!("--- __toappend Modifier ---");
    println!("Append items to the end of an existing array.\n");

    config.load_json(r#"{"tags": ["python", "rust"]}"#, context.clone());
    println!("  Step 1: Load tags=[\"python\", \"rust\"]");

    config.load_json(r#"{"tags__toappend": "go"}"#, context.clone());
    println!("  Step 2: Load tags__toappend=\"go\"");
    println!("  Result: tags=[\"python\", \"rust\", \"go\"]");
    println!("  Use case: Adding plugins, middleware, search paths\n");

    // 3. __toprepend - Prepend to array
    println!("--- __toprepend Modifier ---");
    println!("Prepend items to the beginning of an existing array.\n");

    let mut config2 = Config::default();
    config2.load_json(r#"{"middleware": ["auth", "logging"]}"#, context.clone());
    println!("  Step 1: Load middleware=[\"auth\", \"logging\"]");

    config2.load_json(r#"{"middleware__toprepend": "cors"}"#, context.clone());
    println!("  Step 2: Load middleware__toprepend=\"cors\"");
    println!("  Result: middleware=[\"cors\", \"auth\", \"logging\"]");
    println!("  Use case: Priority middleware, early path resolution\n");

    // 4. __toreplace - Full replacement
    println!("--- __toreplace Modifier ---");
    println!("Replace entire object (discards inherited fields).\n");

    let mut config3 = Config::default();
    config3.load_json(
        r#"{
        "server": {
            "host": "localhost",
            "port": 8080,
            "debug": true
        }
    }"#,
        context.clone(),
    );
    println!("  Step 1: Load server={{host, port, debug}}");

    config3.load_json(
        r#"{
        "server__toreplace": {
            "host": "0.0.0.0",
            "port": 80
        }
    }"#,
        context.clone(),
    );
    println!("  Step 2: Load server__toreplace={{host, port}}");
    println!("  Result: server only has {{host, port}} - debug field removed");
    println!("  Use case: Production overrides, clean slate configs\n");

    // 5. Combining multiple modifiers
    println!("--- Combining Modifiers ---");
    println!("Multiple modifiers can be used together in one config.\n");

    let mut config4 = Config::default();
    config4.load_json(
        r#"{
        "paths": ["/usr/bin"],
        "timeout": 30,
        "plugins": ["core"]
    }"#,
        context.clone(),
    );
    println!("  Base config: paths=[\"/usr/bin\"], timeout=30, plugins=[\"core\"]");

    config4.load_json(
        r#"{
        "paths__toprepend": "/opt/local/bin",
        "timeout__tokeep": 60,
        "plugins__toappend": "extras"
    }"#,
        context.clone(),
    );
    println!("  Overlay: paths__toprepend, timeout__tokeep, plugins__toappend");
    println!("  Result: paths=[\"/opt/local/bin\", \"/usr/bin\"], timeout=30, plugins=[\"core\", \"extras\"]");
    println!("  Note: timeout stayed 30 because __tokeep doesn't override\n");

    // Check for errors
    if config.has_errors() || config2.has_errors() || config3.has_errors() || config4.has_errors() {
        eprintln!("Errors occurred during configuration");
        for c in [&config, &config2, &config3, &config4] {
            for error in c.get_errors() {
                eprintln!("  - {}", error);
            }
        }
    } else {
        println!("=== All merge modifier examples completed successfully! ===");
    }
}

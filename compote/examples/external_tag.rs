//! External Tag Enum Example
//!
//! This example demonstrates the `external_tag` enum feature, which allows
//! the map key to determine which enum variant to deserialize.
//!
//! This is useful for "factory" patterns where configuration like:
//! ```yaml
//! tools:
//!   - homebrew:
//!       - ripgrep
//!   - python: "3.11"
//!   - rust: "1.75"      # Unknown key -> fallback variant
//! ```
//!
//! The key (`homebrew`, `python`, `rust`) determines the variant type.
//!
//! Run with: `cargo run -p compote --example external_tag --features json`

use compote::{Config, Context, Level, Source};

// =============================================================================
// Basic External Tag Enum
// =============================================================================

/// A tool configuration where the YAML/JSON key determines the variant.
///
/// With `external_tag`, input like `{"homebrew": [...]}` automatically
/// deserializes to the `Homebrew` variant.
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag, rename_all = "kebab-case")]
enum Tool {
    /// Homebrew package manager - also accepts "brew" as alias
    #[compote(alias = "brew")]
    Homebrew(HomebrewConfig),

    /// Python version manager
    Python(PythonConfig),

    /// Node.js version manager - also accepts "node" as alias
    #[compote(alias = "node")]
    Nodejs(NodejsConfig),
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(array_as = "packages")] // Allows: {"homebrew": ["pkg"]} instead of {"homebrew": {"packages": ["pkg"]}}
struct HomebrewConfig {
    /// List of packages to install
    #[compote(allow_single)]
    packages: Vec<String>,
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "version")] // Allows: {"python": "3.11"} instead of {"python": {"version": "3.11"}}
struct PythonConfig {
    /// Python version
    #[compote(default)]
    version: Option<String>,
}

#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "version")]
struct NodejsConfig {
    /// Node.js version
    #[compote(default)]
    version: Option<String>,
}

// =============================================================================
// External Tag with Fallback
// =============================================================================

/// A tool configuration with a fallback for unknown keys.
///
/// Unknown keys like "rust", "terraform", "erlang" are caught by the
/// `Mise` fallback variant, which receives the key name via `from_tag = "name"`.
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag, rename_all = "kebab-case")]
enum ToolWithFallback {
    #[compote(alias = "brew")]
    Homebrew(HomebrewConfig),

    Python(PythonConfig),

    /// Fallback for any unknown tool - the key becomes the tool name
    #[compote(fallback, from_tag = "name")]
    Mise(MiseConfig),
}

/// Generic mise tool configuration.
///
/// The `from_tag` variant-level attribute on the enum specifies which field
/// receives the external tag (map key) when this is used as a fallback variant.
#[derive(Debug, compote::Config, PartialEq)]
#[compote(scalar_as = "version")] // Allows: {"rust": "1.75"} with name injected from tag
struct MiseConfig {
    /// The tool name, injected from the external tag via `from_tag = "name"` on the variant.
    /// Needs a default since injection happens after initial deserialization.
    #[compote(default = "")]
    name: String,

    /// Version string (optional)
    #[compote(default)]
    version: Option<String>,
}

// =============================================================================
// Recursive External Tag Enum
// =============================================================================

/// A recursive tool configuration supporting AND/OR combinators.
///
/// This demonstrates that `Vec<Self>` works correctly with external_tag.
#[derive(Debug, compote::Config, PartialEq)]
#[compote(external_tag, rename_all = "kebab-case")]
enum RecursiveTool {
    /// All tools in the list must succeed
    And(Vec<RecursiveTool>),

    /// At least one tool must succeed
    Or(Vec<RecursiveTool>),

    #[compote(alias = "brew")]
    Homebrew(HomebrewConfig),

    Python(PythonConfig),

    #[compote(fallback, from_tag = "name")]
    Mise(MiseConfig),
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    println!("=== External Tag Enum Examples ===\n");

    // -------------------------------------------------------------------------
    // Example 1: Basic external tag
    // -------------------------------------------------------------------------
    println!("1. Basic External Tag");
    println!("   Input: {{\"homebrew\": [\"ripgrep\", \"fd\"]}}");

    let json = r#"{"homebrew": ["ripgrep", "fd"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tool: Tool = config.deserialize().unwrap();

    println!("   Result: {:?}\n", tool);
    assert_eq!(
        tool,
        Tool::Homebrew(HomebrewConfig {
            packages: vec!["ripgrep".to_string(), "fd".to_string()]
        })
    );

    // -------------------------------------------------------------------------
    // Example 2: Using an alias
    // -------------------------------------------------------------------------
    println!("2. Alias Support");
    println!("   Input: {{\"brew\": [\"jq\"]}}  (alias for homebrew)");

    let json = r#"{"brew": ["jq"]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tool: Tool = config.deserialize().unwrap();

    println!("   Result: {:?}\n", tool);
    assert!(matches!(tool, Tool::Homebrew(_)));

    // -------------------------------------------------------------------------
    // Example 3: Fallback for unknown keys
    // -------------------------------------------------------------------------
    println!("3. Fallback Variant");
    println!("   Input: {{\"rust\": \"1.75\"}}  (unknown key -> Mise fallback)");

    let json = r#"{"rust": "1.75"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tool: ToolWithFallback = config.deserialize().unwrap();

    println!("   Result: {:?}", tool);
    if let ToolWithFallback::Mise(mise) = &tool {
        println!("   -> name from tag: \"{}\"", mise.name);
        println!("   -> version: {:?}\n", mise.version);
    }
    assert_eq!(
        tool,
        ToolWithFallback::Mise(MiseConfig {
            name: "rust".to_string(),
            version: Some("1.75".to_string()),
        })
    );

    // -------------------------------------------------------------------------
    // Example 4: Vec of external tag enums
    // -------------------------------------------------------------------------
    println!("4. Vec<ExternalTagEnum>");
    println!(
        "   Input: [{{\"homebrew\": [...]}}, {{\"python\": \"3.11\"}}, {{\"terraform\": \"1.5\"}}]"
    );

    let json = r#"[
        {"homebrew": ["ripgrep"]},
        {"python": "3.11"},
        {"terraform": "1.5"}
    ]"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tools: Vec<ToolWithFallback> = config.deserialize().unwrap();

    println!("   Result: {} tools parsed", tools.len());
    for (i, tool) in tools.iter().enumerate() {
        println!("   [{}] {:?}", i, tool);
    }
    println!();

    // -------------------------------------------------------------------------
    // Example 5: Recursive enum with AND/OR
    // -------------------------------------------------------------------------
    println!("5. Recursive Enum (AND/OR combinators)");
    println!("   Input: {{\"and\": [{{\"python\": \"3.11\"}}, {{\"rust\": \"1.75\"}}]}}");

    let json = r#"{"and": [{"python": "3.11"}, {"rust": "1.75"}]}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tool: RecursiveTool = config.deserialize().unwrap();

    println!("   Result: {:?}\n", tool);
    if let RecursiveTool::And(inner) = &tool {
        assert_eq!(inner.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Example 6: Deeply nested recursive
    // -------------------------------------------------------------------------
    println!("6. Deeply Nested Recursive");
    println!("   Input: {{\"or\": [{{\"and\": [...]}}, {{\"homebrew\": [...]}}]}}");

    let json = r#"{
        "or": [
            {"and": [{"python": "3.11"}, {"go": "1.21"}]},
            {"homebrew": ["ripgrep"]}
        ]
    }"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let tool: RecursiveTool = config.deserialize().unwrap();

    println!("   Result: {:?}\n", tool);

    // -------------------------------------------------------------------------
    // Example 7: Serialization round-trip
    // -------------------------------------------------------------------------
    println!("7. Serialization Round-Trip");

    let original = ToolWithFallback::Homebrew(HomebrewConfig {
        packages: vec!["ripgrep".to_string()],
    });
    let json = compote::to_json_compact(&original).unwrap();
    println!("   Serialized: {}", json);

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let restored: ToolWithFallback = config.deserialize().unwrap();

    println!("   Restored:   {:?}", restored);
    assert_eq!(original, restored);

    println!("\n=== All examples passed! ===");
}

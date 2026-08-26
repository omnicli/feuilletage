//! Unit tests for context module (Level, Context, MutabilityConstraint).
//!
//! Extracted from feuilletage/src/context.rs

use feuilletage::{Context, Format, Level, MutabilityConstraint, Source};
use std::path::Path;

#[test]
fn test_level_names() {
    // Test that built-in levels have the expected names
    let system = Level::System;
    let user = Level::User;
    let local = Level::Local;

    assert_eq!(system.name(), "system");
    assert_eq!(user.name(), "user");
    assert_eq!(local.name(), "local");
}

#[test]
fn test_level_priorities() {
    assert_eq!(Level::System.priority(), 0);
    assert_eq!(Level::User.priority(), 100);
    assert_eq!(Level::Local.priority(), 200);
    assert!(Level::System < Level::User);
    assert!(Level::User < Level::Local);
}

#[test]
fn test_mutability_constraint() {
    // Test Mutable - allows all levels
    let mutable = MutabilityConstraint::Mutable;
    assert!(mutable.allows(&Level::System));
    assert!(mutable.allows(&Level::User));
    assert!(mutable.allows(&Level::Local));

    // Test Immutable - allows no levels
    let immutable = MutabilityConstraint::Immutable;
    assert!(!immutable.allows(&Level::System));
    assert!(!immutable.allows(&Level::User));
    assert!(!immutable.allows(&Level::Local));

    // Test MutableByName - allows only named levels
    let system_only = MutabilityConstraint::mutable_by(&["system"]);
    assert!(system_only.allows(&Level::System));
    assert!(!system_only.allows(&Level::User));
    assert!(!system_only.allows(&Level::Local));

    let system_user = MutabilityConstraint::mutable_by(&["system", "user"]);
    assert!(system_user.allows(&Level::System));
    assert!(system_user.allows(&Level::User));
    assert!(!system_user.allows(&Level::Local));
}

#[test]
fn test_format_detection() {
    // new_from_file auto-detects format from file extension
    let ctx_json: Context =
        Context::new_from_file(Path::new("config.json").to_path_buf(), Level::User);
    assert_eq!(ctx_json.format, Format::Json);

    let ctx_yaml: Context =
        Context::new_from_file(Path::new("config.yaml").to_path_buf(), Level::User);
    assert_eq!(ctx_yaml.format, Format::Yaml);

    let ctx_toml: Context =
        Context::new_from_file(Path::new("config.toml").to_path_buf(), Level::User);
    assert_eq!(ctx_toml.format, Format::Toml);
}

#[test]
fn test_new_does_not_autodetect_format() {
    // Context::new() does NOT auto-detect format - it defaults to Unknown
    // Use new_from_file() or with_format() if you need format detection
    let ctx: Context = Context::new(
        Source::File(Path::new("config.json").to_path_buf()),
        Level::User,
    );
    assert_eq!(ctx.format, Format::Unknown);
}

#[test]
fn test_default_format() {
    let format = Format::default_format();

    #[cfg(feature = "yaml")]
    assert_eq!(format, Format::Yaml);
    #[cfg(all(not(feature = "yaml"), feature = "toml"))]
    assert_eq!(format, Format::Toml);
    #[cfg(all(not(feature = "yaml"), not(feature = "toml"), feature = "json"))]
    assert_eq!(format, Format::Json);
    #[cfg(not(any(feature = "yaml", feature = "toml", feature = "json")))]
    assert_eq!(format, Format::Unknown);
}

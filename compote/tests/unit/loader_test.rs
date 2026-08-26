//! Unit tests for loader module (format loading and ConfigLoaderBuilder).
//!
//! Extracted from compote/src/loader.rs

#[cfg(feature = "json")]
use compote::loader::load_json;
#[cfg(feature = "toml")]
use compote::loader::load_toml;
#[cfg(feature = "yaml")]
use compote::loader::load_yaml;
use compote::loader::loader;
#[cfg(any(feature = "json", feature = "toml", feature = "yaml"))]
use compote::{Context, ContextValue};
use compote::{Error, Format, Level, Source};

#[cfg(feature = "json")]
#[test]
fn test_load_json() {
    let json = r#"{"name": "test", "count": 42, "enabled": true}"#;
    let context: Context = Context::new(Source::Programmatic, Level::User);

    let result = load_json(json, context).unwrap();

    if let ContextValue::Object(map, _) = result {
        assert_eq!(map.len(), 3);
        assert!(matches!(
            map.get("name").unwrap(),
            ContextValue::String(s, _) if s == "test"
        ));
        assert!(matches!(
            map.get("count").unwrap(),
            ContextValue::Int(42, _)
        ));
        assert!(matches!(
            map.get("enabled").unwrap(),
            ContextValue::Bool(true, _)
        ));
    } else {
        panic!("Expected object");
    }
}

#[cfg(feature = "yaml")]
#[test]
fn test_load_yaml() {
    let yaml = r#"
name: test
count: 42
enabled: true
"#;
    let context: Context = Context::new(Source::Programmatic, Level::User);

    let result = load_yaml(yaml, context).unwrap();

    if let ContextValue::Object(map, _) = result {
        assert_eq!(map.len(), 3);
        assert!(matches!(
            map.get("name").unwrap(),
            ContextValue::String(s, _) if s == "test"
        ));
        assert!(matches!(
            map.get("count").unwrap(),
            ContextValue::Int(42, _)
        ));
        assert!(matches!(
            map.get("enabled").unwrap(),
            ContextValue::Bool(true, _)
        ));
    } else {
        panic!("Expected object");
    }
}

#[cfg(feature = "toml")]
#[test]
fn test_load_toml() {
    let toml = r#"
name = "test"
count = 42
enabled = true
"#;
    let context: Context = Context::new(Source::Programmatic, Level::User);

    let result = load_toml(toml, context).unwrap();

    if let ContextValue::Object(map, _) = result {
        assert_eq!(map.len(), 3);
        assert!(matches!(
            map.get("name").unwrap(),
            ContextValue::String(s, _) if s == "test"
        ));
        assert!(matches!(
            map.get("count").unwrap(),
            ContextValue::Int(42, _)
        ));
        assert!(matches!(
            map.get("enabled").unwrap(),
            ContextValue::Bool(true, _)
        ));
    } else {
        panic!("Expected object");
    }
}

// ============================================================================
// ConfigLoaderBuilder tests
// ============================================================================

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_load_str() {
    let config = loader()
        .load_str(
            r#"{"name": "system", "count": 10}"#,
            Format::Json,
            Level::System,
        )
        .unwrap()
        .load_str(r#"{"count": 20}"#, Format::Json, Level::User)
        .unwrap()
        .build()
        .unwrap();

    // Verify the merge happened (User overrides System)
    let name = config.get("name").unwrap();
    let count = config.get("count").unwrap();

    assert!(matches!(name, ContextValue::String(s, _) if s == "system"));
    assert!(matches!(count, ContextValue::Int(20, _)));
}

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_build() {
    // Test building Config from multiple sources
    let config = loader()
        .load_str(
            r#"{"name": "system", "count": 10}"#,
            Format::Json,
            Level::System,
        )
        .unwrap()
        .load_str(r#"{"count": 20}"#, Format::Json, Level::User)
        .unwrap()
        .build()
        .unwrap();

    let name = config.get("name").unwrap();
    let count = config.get("count").unwrap();

    assert!(matches!(name, ContextValue::String(s, _) if s == "system"));
    assert!(matches!(count, ContextValue::Int(20, _)));
}

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_orders_sources_by_level_priority() {
    let config = loader()
        .load_str(r#"{"value": "local"}"#, Format::Json, Level::Local)
        .unwrap()
        .load_str(r#"{"value": "system"}"#, Format::Json, Level::System)
        .unwrap()
        .load_str(r#"{"value": "user"}"#, Format::Json, Level::User)
        .unwrap()
        .build()
        .unwrap();

    assert!(matches!(
        config.get("value"),
        Some(ContextValue::String(value, _)) if value == "local"
    ));
}

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_retains_insertion_order_for_equal_priorities() {
    let config = loader()
        .load_str(r#"{"value": "first"}"#, Format::Json, Level::User)
        .unwrap()
        .load_str(r#"{"value": "second"}"#, Format::Json, Level::User)
        .unwrap()
        .build()
        .unwrap();

    assert!(matches!(
        config.get("value"),
        Some(ContextValue::String(value, _)) if value == "second"
    ));
}

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_uses_custom_level_priorities() {
    #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    enum CustomPriorityLevel {
        High,
        #[default]
        Low,
    }

    impl compote::CustomLevel for CustomPriorityLevel {
        fn name(&self) -> &str {
            match self {
                Self::High => "high",
                Self::Low => "low",
            }
        }

        fn priority(&self) -> u32 {
            match self {
                Self::High => 20,
                Self::Low => 10,
            }
        }
    }

    let config = compote::ConfigLoaderBuilder::<Source, CustomPriorityLevel>::new()
        .load_str(
            r#"{"value": "high"}"#,
            Format::Json,
            CustomPriorityLevel::High,
        )
        .unwrap()
        .load_str(
            r#"{"value": "low"}"#,
            Format::Json,
            CustomPriorityLevel::Low,
        )
        .unwrap()
        .build()
        .unwrap();

    assert!(matches!(
        config.get("value"),
        Some(ContextValue::String(value, _)) if value == "high"
    ));
}

#[cfg(feature = "json")]
#[test]
fn test_loader_function() {
    use compote::loader::loader;
    // Test the convenience function - just verify it can be called
    let _builder = loader();
    // Can't check internal state, but if it builds and doesn't panic, it works
}

#[cfg(feature = "json")]
#[test]
fn test_loader_builder_multiple_formats() {
    let config = loader()
        .load_str(r#"{"host": "localhost"}"#, Format::Json, Level::System)
        .unwrap()
        .build()
        .unwrap();

    let host = config.get("host").unwrap();
    assert!(matches!(host, ContextValue::String(s, _) if s == "localhost"));
}

#[cfg(all(feature = "json", feature = "yaml"))]
#[test]
fn test_loader_builder_json_and_yaml() {
    let config = loader()
        .load_str(r#"{"name": "json-value"}"#, Format::Json, Level::System)
        .unwrap()
        .load_str("port: 8080\n", Format::Yaml, Level::User)
        .unwrap()
        .build()
        .unwrap();

    let name = config.get("name").unwrap();
    let port = config.get("port").unwrap();

    assert!(matches!(name, ContextValue::String(s, _) if s == "json-value"));
    assert!(matches!(port, ContextValue::Int(8080, _)));
}

#[cfg(feature = "yaml")]
#[test]
fn test_loader_builder_load_file_sets_detected_format_context() {
    use tempfile::NamedTempFile;

    fn context_format<S: compote::CustomSource, L: compote::CustomLevel>(
        context: &Context<S, L>,
    ) -> Format {
        context.format.clone()
    }

    #[derive(Debug, compote::Config)]
    struct FileConfig {
        name: String,
        #[compote(from_context_fn = "context_format")]
        format: Format,
    }

    let file = NamedTempFile::with_suffix(".yaml").unwrap();
    std::fs::write(file.path(), "name: detected-yaml\n").unwrap();

    let mut loader = loader().load_file(file.path(), Level::User);
    let config: FileConfig = loader.deserialize().unwrap();

    assert_eq!(config.name, "detected-yaml");
    assert_eq!(config.format, Format::Yaml);
}

#[cfg(feature = "yaml")]
#[test]
fn test_loader_builder_load_file_with_format_preserves_file_context() {
    use tempfile::NamedTempFile;

    #[derive(Debug, compote::Config)]
    struct FileConfig {
        #[compote(default = "fallback")]
        name: String,
        #[compote(from_context = "source.file_path")]
        source_path: Option<std::path::PathBuf>,
        #[compote(from_context = "level.name")]
        level: String,
    }

    let file = NamedTempFile::with_suffix(".conf").unwrap();
    std::fs::write(file.path(), "name: explicit-yaml\n").unwrap();

    let mut loader = loader().load_file_with_format(file.path(), Format::Yaml, Level::Local);
    assert_eq!(loader.loaded_files(), &[file.path().to_path_buf()]);

    let config: FileConfig = loader.deserialize().unwrap();
    assert_eq!(config.name, "explicit-yaml");
    assert_eq!(config.source_path.as_deref(), Some(file.path()));
    assert_eq!(config.level, "local");
}

#[cfg(feature = "yaml")]
#[test]
fn test_loader_builder_explicit_format_parse_error_uses_file_source() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::with_suffix(".conf").unwrap();
    std::fs::write(file.path(), "name: [unterminated\n").unwrap();

    let loader = loader().load_file_with_format(file.path(), Format::Yaml, Level::User);
    assert!(loader.loaded_files().is_empty());
    assert_eq!(loader.errors().errors().len(), 1);
    assert!(matches!(
        &loader.errors().errors()[0],
        Error::ParseError { source, .. } if source == &file.path().display().to_string()
    ));
}

#[test]
fn test_loader_builder_load_file_rejects_unknown_extension() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::with_suffix(".conf").unwrap();
    std::fs::write(file.path(), "name: value\n").unwrap();

    let loader = loader().load_file(file.path(), Level::User);
    assert!(loader.loaded_files().is_empty());
    assert!(matches!(
        loader.errors().errors(),
        [Error::FormatNotSupported { format, .. }] if format == "conf"
    ));
}

#[cfg(feature = "yaml")]
#[test]
fn test_direct_load_file_with_format_accepts_unknown_extension() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::with_suffix(".conf").unwrap();
    std::fs::write(file.path(), "name: explicit-yaml\n").unwrap();

    let value: ContextValue =
        compote::loader::load_file_with_format(file.path(), Format::Yaml, Level::User)
            .unwrap()
            .unwrap();

    assert_eq!(value.context().format, Format::Yaml);
    assert_eq!(value.context().source.file_path(), Some(file.path()));
}

#[cfg(all(feature = "json", feature = "yaml", feature = "toml"))]
#[test]
fn test_load_file_auto_uses_json_toml_yaml_order() {
    use tempfile::NamedTempFile;

    let json = NamedTempFile::new().unwrap();
    std::fs::write(json.path(), r#"{"name":"json"}"#).unwrap();
    let json_value: ContextValue = compote::loader::load_file_auto(json.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(json_value.context().format, Format::Json);

    let toml = NamedTempFile::new().unwrap();
    std::fs::write(toml.path(), "name = \"toml\"\n").unwrap();
    let toml_value: ContextValue = compote::loader::load_file_auto(toml.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(toml_value.context().format, Format::Toml);

    let yaml = NamedTempFile::new().unwrap();
    std::fs::write(yaml.path(), "name: yaml\n").unwrap();
    let yaml_value: ContextValue = compote::loader::load_file_auto(yaml.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(yaml_value.context().format, Format::Yaml);
}

#[cfg(all(feature = "json", feature = "yaml", feature = "toml"))]
#[test]
fn test_loader_builder_auto_success_discards_speculative_errors() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "name = \"toml\"\n").unwrap();

    let loader = loader().load_file_auto(file.path(), Level::User);
    assert_eq!(loader.loaded_files(), [file.path()]);
    assert!(loader.errors().errors().is_empty());
}

#[cfg(all(feature = "json", feature = "yaml", feature = "toml"))]
#[test]
fn test_load_file_auto_aggregates_parser_failures_in_attempt_order() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "[").unwrap();

    let error =
        compote::loader::load_file_auto::<Source, Level>(file.path(), Level::User).unwrap_err();
    let Error::ParseError { source, message } = error else {
        panic!("expected aggregate parse error");
    };
    assert_eq!(source, file.path().display().to_string());
    let json = message.find("JSON:").unwrap();
    let toml = message.find("TOML:").unwrap();
    let yaml = message.find("YAML:").unwrap();
    assert!(json < toml && toml < yaml);
}

#[cfg(all(feature = "json", not(feature = "toml"), not(feature = "yaml")))]
#[test]
fn test_load_file_auto_with_only_json_enabled() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), r#"{"name":"json"}"#).unwrap();
    let value: ContextValue = compote::loader::load_file_auto(file.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(value.context().format, Format::Json);
}

#[cfg(all(feature = "toml", not(feature = "json"), not(feature = "yaml")))]
#[test]
fn test_load_file_auto_with_only_toml_enabled() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "name = \"toml\"\n").unwrap();
    let value: ContextValue = compote::loader::load_file_auto(file.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(value.context().format, Format::Toml);
}

#[cfg(all(feature = "yaml", not(feature = "json"), not(feature = "toml")))]
#[test]
fn test_load_file_auto_with_only_yaml_enabled() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "name: yaml\n").unwrap();
    let value: ContextValue = compote::loader::load_file_auto(file.path(), Level::User)
        .unwrap()
        .unwrap();
    assert_eq!(value.context().format, Format::Yaml);
}

#[cfg(not(any(feature = "json", feature = "toml", feature = "yaml")))]
#[test]
fn test_load_file_auto_without_format_features_reports_unsupported() {
    use tempfile::NamedTempFile;

    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "name: value\n").unwrap();
    let error =
        compote::loader::load_file_auto::<Source, Level>(file.path(), Level::User).unwrap_err();
    assert!(matches!(
        error,
        Error::FormatNotSupported { format, .. } if format == "auto"
    ));
}

#[cfg(all(feature = "json", feature = "yaml"))]
#[test]
fn test_builder_build_preserves_last_loaded_format() {
    let config = loader()
        .load_str(r#"{"first": true}"#, Format::Json, Level::System)
        .unwrap()
        .load_str("second: true\n", Format::Yaml, Level::User)
        .unwrap()
        .build()
        .unwrap();

    assert_eq!(config.loaded_format(), Format::Yaml);
    let raw = config.serialize_raw().unwrap();
    assert!(raw.contains("second: true"));
}

#[cfg(all(feature = "json", feature = "yaml", feature = "toml"))]
#[test]
fn test_loader_builder_default_format_is_output_only() {
    use tempfile::NamedTempFile;

    let extensionless = NamedTempFile::new().unwrap();
    std::fs::write(extensionless.path(), "name = \"default-toml\"\n").unwrap();

    let loader = loader()
        .default_format(Format::Toml)
        .load_file(extensionless.path(), Level::System);

    assert!(loader.loaded_files().is_empty());
    assert!(matches!(
        loader.errors().errors(),
        [Error::FormatNotSupported { format, .. }] if format.is_empty()
    ));

    let config = loader.build().unwrap();
    assert_eq!(config.default_format(), Format::Toml);
    assert_eq!(config.loaded_format(), Format::Unknown);
    assert!(config.get("name").is_none());
}

#[test]
fn test_loader_builder_records_invalid_default_format() {
    let config = loader().default_format(Format::Unknown).build().unwrap();

    assert!(matches!(
        config.errors().errors(),
        [Error::FormatNotSupported { format, .. }] if format == "unknown"
    ));
}

#[test]
#[cfg(not(feature = "toml"))]
fn test_loader_builder_records_disabled_default_format() {
    let config = loader().default_format(Format::Toml).build().unwrap();

    assert!(matches!(
        config.errors().errors(),
        [Error::FormatNotSupported { format, .. }] if format == "toml"
    ));
}

#[test]
fn test_builder_rejects_unknown_string_format() {
    let result = loader().load_str("name: value", Format::Unknown, Level::User);
    assert!(matches!(result, Err(Error::FormatNotSupported { format, .. }) if format == "unknown"));
}

#[cfg(feature = "yaml")]
mod warning_path_tests {
    use super::*;
    use compote::de::MutabilityInfo;
    use compote::{ErrorTracker, FromContextValue, MutabilityHashMap};

    struct WarningConfig;

    impl FromContextValue for WarningConfig {
        fn from_context_value(
            _value: &ContextValue,
            tracker: &mut ErrorTracker,
        ) -> Result<Self, Error> {
            tracker.push_field("server");
            tracker.push_field("port");
            tracker.record_warning("deprecated setting");
            tracker.pop();
            tracker.pop();
            Ok(Self)
        }
    }

    impl MutabilityInfo for WarningConfig {
        fn mutability_constraints() -> MutabilityHashMap<String, &'static [&'static str]> {
            MutabilityHashMap::new()
        }
    }

    #[test]
    fn test_deserialize_copies_warning_path() {
        let mut loader = loader().load_str("{}", Format::Yaml, Level::User).unwrap();

        let _: WarningConfig = loader.deserialize().unwrap();
        assert_eq!(loader.errors().warnings()[0].path, "server.port");
    }

    #[test]
    fn test_deserialize_unconstrained_copies_warning_path() {
        let mut loader = loader().load_str("{}", Format::Yaml, Level::User).unwrap();

        let _: WarningConfig = loader.deserialize_unconstrained().unwrap();
        assert_eq!(loader.errors().warnings()[0].path, "server.port");
    }
}

// ============================================================================
// Mutability enforcement tests
// ============================================================================

#[cfg(feature = "json")]
mod mutability_tests {
    use super::*;
    use compote::de::MutabilityInfo;
    use compote::error::Error;
    use compote::MutabilityHashMap;

    /// A test config struct with mutable_by constraints
    #[derive(Debug, PartialEq)]
    struct TestConfig {
        app_name: String,
        user_preference: String,
    }

    /// Manual implementation of FromContextValue for testing
    impl compote::FromContextValue for TestConfig {
        fn from_context_value(
            value: &ContextValue,
            tracker: &mut compote::error::ErrorTracker,
        ) -> Result<Self, compote::error::Error> {
            if let ContextValue::Object(map, _) = value {
                let app_name = map
                    .get("app_name")
                    .and_then(|v| {
                        if let ContextValue::String(s, _) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                let user_preference = map
                    .get("user_preference")
                    .and_then(|v| {
                        if let ContextValue::String(s, _) = v {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();

                Ok(TestConfig {
                    app_name,
                    user_preference,
                })
            } else {
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "object".to_string(),
                    actual: format!("{:?}", value),
                })
            }
        }
    }

    /// Manual implementation of MutabilityInfo for testing
    /// user_preference can only be set by user or local levels
    impl MutabilityInfo for TestConfig {
        fn mutability_constraints() -> MutabilityHashMap<String, &'static [&'static str]> {
            static USER_LOCAL: &[&str] = &["user", "local"];
            let mut map = MutabilityHashMap::new();
            map.insert("user_preference".to_string(), USER_LOCAL);
            map
        }
    }

    #[test]
    fn test_mutability_constraint_skips_system_level() {
        // System level sets both app_name and user_preference
        // User level sets only user_preference
        // Expected: app_name from system, user_preference from user (system skipped)
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "MyApp", "user_preference": "system_default"}"#,
                Format::Json,
                Level::System,
            )
            .unwrap()
            .load_str(
                r#"{"user_preference": "user_choice"}"#,
                Format::Json,
                Level::User,
            )
            .unwrap()
            .deserialize()
            .unwrap();

        assert_eq!(config.app_name, "MyApp"); // From system
        assert_eq!(config.user_preference, "user_choice"); // From user, system was skipped
    }

    #[test]
    fn test_mutability_constraint_allows_user_level() {
        // User level is allowed by mutable_by constraint
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "App", "user_preference": "user_val"}"#,
                Format::Json,
                Level::User,
            )
            .unwrap()
            .deserialize()
            .unwrap();

        assert_eq!(config.user_preference, "user_val");
    }

    #[test]
    fn test_mutability_constraint_allows_local_level() {
        // Local level is allowed by mutable_by constraint
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "App", "user_preference": "local_val"}"#,
                Format::Json,
                Level::Local,
            )
            .unwrap()
            .deserialize()
            .unwrap();

        assert_eq!(config.user_preference, "local_val");
    }

    #[test]
    fn test_mutability_constraint_system_only_sets_unconstrained_fields() {
        // System level sets user_preference but it should be skipped
        // Since there's no default, the field will be empty
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "SystemApp", "user_preference": "system_should_be_skipped"}"#,
                Format::Json,
                Level::System,
            )
            .unwrap()
            .deserialize()
            .unwrap();

        assert_eq!(config.app_name, "SystemApp");
        // user_preference should be empty because system level was skipped
        assert_eq!(config.user_preference, "");
    }

    #[test]
    fn test_deserialize_unconstrained_ignores_mutability() {
        // Using deserialize_unconstrained should NOT enforce mutable_by
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "MyApp", "user_preference": "from_system"}"#,
                Format::Json,
                Level::System,
            )
            .unwrap()
            .deserialize_unconstrained()
            .unwrap();

        // Even system level should be allowed for user_preference
        assert_eq!(config.user_preference, "from_system");
    }

    #[test]
    fn test_deserialize_orders_sources_by_level_priority() {
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "user", "user_preference": "user"}"#,
                Format::Json,
                Level::User,
            )
            .unwrap()
            .load_str(
                r#"{"app_name": "system", "user_preference": "system"}"#,
                Format::Json,
                Level::System,
            )
            .unwrap()
            .deserialize()
            .unwrap();

        assert_eq!(config.app_name, "user");
        assert_eq!(config.user_preference, "user");
    }

    #[test]
    fn test_deserialize_unconstrained_orders_sources_by_level_priority() {
        let config: TestConfig = loader()
            .load_str(
                r#"{"app_name": "user", "user_preference": "user"}"#,
                Format::Json,
                Level::User,
            )
            .unwrap()
            .load_str(
                r#"{"app_name": "system", "user_preference": "system"}"#,
                Format::Json,
                Level::System,
            )
            .unwrap()
            .deserialize_unconstrained()
            .unwrap();

        assert_eq!(config.app_name, "user");
        assert_eq!(config.user_preference, "user");
    }
}

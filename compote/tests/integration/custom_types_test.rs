//! Tests for custom Source and Level types with derive macros.
//!
//! This module tests that `#[derive(Config)]` works correctly with:
//! - Custom Source + default Level
//! - Default Source + custom Level
//! - Custom Source + custom Level

use std::path::{Path, PathBuf};

use compote::{
    Config as DeriveConfig, Context, ContextValue, CustomLevel, CustomSource, ErrorTracker,
    FromContextValue,
};

// ============================================================================
// Custom Source Type
// ============================================================================

/// A custom source type for testing.
#[derive(Clone, Debug, Default, PartialEq)]
enum TestSource {
    #[default]
    Default,
    File(PathBuf),
    Environment,
    Programmatic,
    /// Custom variant: a package source with name and version
    Package {
        name: String,
        version: String,
    },
    /// Custom variant: a remote configuration URL
    Remote(String),
}

impl compote::CustomSource for TestSource {
    fn display_name(&self) -> String {
        match self {
            TestSource::Default => "default".to_string(),
            TestSource::File(path) => path.display().to_string(),
            TestSource::Environment => "environment".to_string(),
            TestSource::Programmatic => "programmatic".to_string(),
            TestSource::Package { name, version } => format!("package:{}@{}", name, version),
            TestSource::Remote(url) => format!("remote:{}", url),
        }
    }

    fn file_path(&self) -> Option<&Path> {
        match self {
            TestSource::File(path) => Some(path.as_path()),
            _ => None,
        }
    }

    fn from_file(path: PathBuf) -> Self {
        TestSource::File(path)
    }

    fn programmatic() -> Self {
        TestSource::Programmatic
    }

    fn environment() -> Self {
        TestSource::Environment
    }
}

// ============================================================================
// Custom Level Type
// ============================================================================

/// A custom level type for testing with different priority semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
enum TestLevel {
    /// Base configuration (lowest priority)
    #[default]
    Base,
    /// Team-wide overrides
    Team,
    /// Project-specific settings
    Project,
    /// User preferences (highest priority)
    User,
}

impl compote::CustomLevel for TestLevel {
    fn name(&self) -> &str {
        match self {
            TestLevel::Base => "base",
            TestLevel::Team => "team",
            TestLevel::Project => "project",
            TestLevel::User => "user",
        }
    }
}

// ============================================================================
// Test Structs using derive macro
// ============================================================================

/// Simple config struct for testing
#[derive(DeriveConfig, Debug, PartialEq)]
struct SimpleConfig {
    #[compote(default = "default_name")]
    name: String,
    #[compote(default = 42)]
    count: i32,
}

/// Config struct with nested types
#[derive(DeriveConfig, Debug, PartialEq)]
struct NestedConfig {
    #[compote(default = "app")]
    app_name: String,
    database: DatabaseConfig,
}

#[derive(DeriveConfig, Debug, PartialEq, Default)]
struct DatabaseConfig {
    #[compote(default = "localhost")]
    host: String,
    #[compote(default = 5432)]
    port: u16,
}

/// Config struct with Option and Vec
#[derive(DeriveConfig, Debug, PartialEq)]
struct CollectionConfig {
    #[compote(default = "test")]
    name: String,
    description: Option<String>,
    #[compote(default = [])]
    tags: Vec<String>,
}

/// Config struct with validation
#[derive(DeriveConfig, Debug, PartialEq)]
struct ValidatedConfig {
    #[compote(default = "user", regex = "^[a-z]+$")]
    username: String,
    #[compote(default = 18, range(min = 0, max = 150))]
    age: u8,
}

// ============================================================================
// Helper functions for creating ContextValues with custom types
// ============================================================================

// --- Custom Source + Default Level ---

fn cv_string_cs(
    s: impl Into<String>,
    source: TestSource,
) -> ContextValue<TestSource, compote::Level> {
    ContextValue::string(s.into(), Context::new(source, compote::Level::default()))
}

fn cv_int_cs(i: i64, source: TestSource) -> ContextValue<TestSource, compote::Level> {
    ContextValue::int(i, Context::new(source, compote::Level::default()))
}

fn cv_array_cs(
    items: Vec<ContextValue<TestSource, compote::Level>>,
    source: TestSource,
) -> ContextValue<TestSource, compote::Level> {
    ContextValue::array(items, Context::new(source, compote::Level::default()))
}

fn cv_object_cs(
    map: indexmap::IndexMap<String, ContextValue<TestSource, compote::Level>>,
    source: TestSource,
) -> ContextValue<TestSource, compote::Level> {
    ContextValue::object(map, Context::new(source, compote::Level::default()))
}

// --- Default Source + Custom Level ---

fn cv_string_cl(
    s: impl Into<String>,
    level: TestLevel,
) -> ContextValue<compote::Source, TestLevel> {
    ContextValue::string(s.into(), Context::new(compote::Source::default(), level))
}

fn cv_int_cl(i: i64, level: TestLevel) -> ContextValue<compote::Source, TestLevel> {
    ContextValue::int(i, Context::new(compote::Source::default(), level))
}

fn cv_array_cl(
    items: Vec<ContextValue<compote::Source, TestLevel>>,
    level: TestLevel,
) -> ContextValue<compote::Source, TestLevel> {
    ContextValue::array(items, Context::new(compote::Source::default(), level))
}

fn cv_object_cl(
    map: indexmap::IndexMap<String, ContextValue<compote::Source, TestLevel>>,
    level: TestLevel,
) -> ContextValue<compote::Source, TestLevel> {
    ContextValue::object(map, Context::new(compote::Source::default(), level))
}

// --- Custom Source + Custom Level ---

fn cv_string_both(
    s: impl Into<String>,
    source: TestSource,
    level: TestLevel,
) -> ContextValue<TestSource, TestLevel> {
    ContextValue::string(s.into(), Context::new(source, level))
}

fn cv_int_both(
    i: i64,
    source: TestSource,
    level: TestLevel,
) -> ContextValue<TestSource, TestLevel> {
    ContextValue::int(i, Context::new(source, level))
}

fn cv_array_both(
    items: Vec<ContextValue<TestSource, TestLevel>>,
    source: TestSource,
    level: TestLevel,
) -> ContextValue<TestSource, TestLevel> {
    ContextValue::array(items, Context::new(source, level))
}

fn cv_object_both(
    map: indexmap::IndexMap<String, ContextValue<TestSource, TestLevel>>,
    source: TestSource,
    level: TestLevel,
) -> ContextValue<TestSource, TestLevel> {
    ContextValue::object(map, Context::new(source, level))
}

// ============================================================================
// Tests: Custom Source + Default Level
// ============================================================================

mod custom_source_default_level {
    use super::*;
    use indexmap::IndexMap;

    type CV = ContextValue<TestSource, compote::Level>;

    fn make_object(pairs: Vec<(&str, CV)>, source: TestSource) -> CV {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        cv_object_cs(map, source)
    }

    #[test]
    fn simple_config_with_default_source() {
        let source = TestSource::Default;
        let value = make_object(
            vec![
                ("name", cv_string_cs("custom_name", source.clone())),
                ("count", cv_int_cs(100, source.clone())),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "custom_name");
        assert_eq!(config.count, 100);
    }

    #[test]
    fn simple_config_with_package_source() {
        let source = TestSource::Package {
            name: "my-package".to_string(),
            version: "1.0.0".to_string(),
        };
        let value = make_object(
            vec![
                ("name", cv_string_cs("pkg_config", source.clone())),
                ("count", cv_int_cs(200, source.clone())),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "pkg_config");
        assert_eq!(config.count, 200);
    }

    #[test]
    fn simple_config_with_remote_source() {
        let source = TestSource::Remote("https://config.example.com".to_string());
        let value = make_object(
            vec![
                ("name", cv_string_cs("remote_config", source.clone())),
                ("count", cv_int_cs(77, source.clone())),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "remote_config");
        assert_eq!(config.count, 77);
    }

    #[test]
    fn nested_config_with_custom_source() {
        let source = TestSource::File(PathBuf::from("/etc/myapp/config.yaml"));
        let db_obj = make_object(
            vec![
                ("host", cv_string_cs("db.example.com", source.clone())),
                ("port", cv_int_cs(3306, source.clone())),
            ],
            source.clone(),
        );
        let value = make_object(
            vec![
                ("app_name", cv_string_cs("MyApp", source.clone())),
                ("database", db_obj),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: NestedConfig = NestedConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.app_name, "MyApp");
        assert_eq!(config.database.host, "db.example.com");
        assert_eq!(config.database.port, 3306);
    }

    #[test]
    fn collection_config_with_custom_source() {
        let source = TestSource::Environment;
        let tags_array = cv_array_cs(
            vec![
                cv_string_cs("tag1", source.clone()),
                cv_string_cs("tag2", source.clone()),
            ],
            source.clone(),
        );
        let value = make_object(
            vec![
                ("name", cv_string_cs("test", source.clone())),
                ("description", cv_string_cs("A test config", source.clone())),
                ("tags", tags_array),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: CollectionConfig =
            CollectionConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "test");
        assert_eq!(config.description, Some("A test config".to_string()));
        assert_eq!(config.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn validated_config_with_custom_source() {
        let source = TestSource::Programmatic;
        let value = make_object(
            vec![
                ("username", cv_string_cs("alice", source.clone())),
                ("age", cv_int_cs(25, source.clone())),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let config: ValidatedConfig =
            ValidatedConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.username, "alice");
        assert_eq!(config.age, 25);
    }

    #[test]
    fn validated_config_fails_validation_with_custom_source() {
        let source = TestSource::Remote("https://bad-config.example.com".to_string());
        let value = make_object(
            vec![
                (
                    "username",
                    cv_string_cs("INVALID_UPPERCASE", source.clone()),
                ),
                ("age", cv_int_cs(25, source.clone())),
            ],
            source,
        );
        let mut tracker = ErrorTracker::new();

        let result = ValidatedConfig::from_context_value(&value, &mut tracker);

        assert!(result.is_err() || !tracker.errors().is_empty());
    }
}

// ============================================================================
// Tests: Default Source + Custom Level
// ============================================================================

mod default_source_custom_level {
    use super::*;
    use indexmap::IndexMap;

    type CV = ContextValue<compote::Source, TestLevel>;

    fn make_object(pairs: Vec<(&str, CV)>, level: TestLevel) -> CV {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        cv_object_cl(map, level)
    }

    #[test]
    fn simple_config_with_base_level() {
        let level = TestLevel::Base;
        let value = make_object(
            vec![
                ("name", cv_string_cl("base_config", level.clone())),
                ("count", cv_int_cl(10, level.clone())),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "base_config");
        assert_eq!(config.count, 10);
    }

    #[test]
    fn simple_config_with_team_level() {
        let level = TestLevel::Team;
        let value = make_object(
            vec![
                ("name", cv_string_cl("team_config", level.clone())),
                ("count", cv_int_cl(88, level.clone())),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "team_config");
        assert_eq!(config.count, 88);
    }

    #[test]
    fn simple_config_with_project_level() {
        let level = TestLevel::Project;
        let value = make_object(
            vec![
                ("name", cv_string_cl("project_config", level.clone())),
                ("count", cv_int_cl(500, level.clone())),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "project_config");
        assert_eq!(config.count, 500);
    }

    #[test]
    fn simple_config_with_user_level() {
        let level = TestLevel::User;
        let value = make_object(
            vec![
                ("name", cv_string_cl("user_config", level.clone())),
                ("count", cv_int_cl(99, level.clone())),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "user_config");
        assert_eq!(config.count, 99);
    }

    #[test]
    fn nested_config_with_custom_level() {
        let level = TestLevel::Project;
        let db_obj = make_object(
            vec![
                ("host", cv_string_cl("project-db.local", level.clone())),
                ("port", cv_int_cl(5433, level.clone())),
            ],
            level.clone(),
        );
        let value = make_object(
            vec![
                ("app_name", cv_string_cl("ProjectApp", level.clone())),
                ("database", db_obj),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: NestedConfig = NestedConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.app_name, "ProjectApp");
        assert_eq!(config.database.host, "project-db.local");
        assert_eq!(config.database.port, 5433);
    }

    #[test]
    fn collection_config_with_custom_level() {
        let level = TestLevel::Team;
        let tags_array = cv_array_cl(vec![cv_string_cl("team-tag", level.clone())], level.clone());
        let value = make_object(
            vec![
                ("name", cv_string_cl("team_service", level.clone())),
                ("tags", tags_array),
            ],
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: CollectionConfig =
            CollectionConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "team_service");
        assert_eq!(config.tags, vec!["team-tag"]);
    }

    #[test]
    fn level_name_is_accessible() {
        let level = TestLevel::Project;
        assert_eq!(CustomLevel::name(&level), "project");

        let level = TestLevel::User;
        assert_eq!(CustomLevel::name(&level), "user");

        let level = TestLevel::Base;
        assert_eq!(CustomLevel::name(&level), "base");

        let level = TestLevel::Team;
        assert_eq!(CustomLevel::name(&level), "team");
    }
}

// ============================================================================
// Tests: Custom Source + Custom Level
// ============================================================================

mod custom_source_custom_level {
    use super::*;
    use indexmap::IndexMap;

    type CV = ContextValue<TestSource, TestLevel>;

    fn make_object(pairs: Vec<(&str, CV)>, source: TestSource, level: TestLevel) -> CV {
        let mut map = IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v);
        }
        cv_object_both(map, source, level)
    }

    #[test]
    fn simple_config_with_both_custom() {
        let source = TestSource::Package {
            name: "core".to_string(),
            version: "2.0.0".to_string(),
        };
        let level = TestLevel::Base;

        let value = make_object(
            vec![
                (
                    "name",
                    cv_string_both("core_config", source.clone(), level.clone()),
                ),
                ("count", cv_int_both(999, source.clone(), level.clone())),
            ],
            source,
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "core_config");
        assert_eq!(config.count, 999);
    }

    #[test]
    fn simple_config_remote_source_user_level() {
        let source = TestSource::Remote("https://user-prefs.example.com".to_string());
        let level = TestLevel::User;

        let value = make_object(
            vec![
                (
                    "name",
                    cv_string_both("user_remote_config", source.clone(), level.clone()),
                ),
                ("count", cv_int_both(55, source.clone(), level.clone())),
            ],
            source,
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: SimpleConfig = SimpleConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "user_remote_config");
        assert_eq!(config.count, 55);
    }

    #[test]
    fn nested_config_with_both_custom() {
        let source = TestSource::File(PathBuf::from("/project/.config/app.toml"));
        let level = TestLevel::Project;

        let db_obj = make_object(
            vec![
                (
                    "host",
                    cv_string_both("project-db", source.clone(), level.clone()),
                ),
                ("port", cv_int_both(5434, source.clone(), level.clone())),
            ],
            source.clone(),
            level.clone(),
        );

        let value = make_object(
            vec![
                (
                    "app_name",
                    cv_string_both("ProjectApp", source.clone(), level.clone()),
                ),
                ("database", db_obj),
            ],
            source,
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: NestedConfig = NestedConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.app_name, "ProjectApp");
        assert_eq!(config.database.host, "project-db");
        assert_eq!(config.database.port, 5434);
    }

    #[test]
    fn collection_config_with_both_custom() {
        let source = TestSource::Environment;
        let level = TestLevel::Team;

        let tags_array = cv_array_both(
            vec![
                cv_string_both("env-tag1", source.clone(), level.clone()),
                cv_string_both("env-tag2", source.clone(), level.clone()),
                cv_string_both("env-tag3", source.clone(), level.clone()),
            ],
            source.clone(),
            level.clone(),
        );

        let value = make_object(
            vec![
                (
                    "name",
                    cv_string_both("env_team_config", source.clone(), level.clone()),
                ),
                (
                    "description",
                    cv_string_both("Environment config for team", source.clone(), level.clone()),
                ),
                ("tags", tags_array),
            ],
            source,
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: CollectionConfig =
            CollectionConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "env_team_config");
        assert_eq!(
            config.description,
            Some("Environment config for team".to_string())
        );
        assert_eq!(config.tags, vec!["env-tag1", "env-tag2", "env-tag3"]);
    }

    #[test]
    fn validated_config_with_both_custom() {
        let source = TestSource::Package {
            name: "validated-pkg".to_string(),
            version: "1.0.0".to_string(),
        };
        let level = TestLevel::User;

        let value = make_object(
            vec![
                (
                    "username",
                    cv_string_both("validuser", source.clone(), level.clone()),
                ),
                ("age", cv_int_both(30, source.clone(), level.clone())),
            ],
            source,
            level,
        );
        let mut tracker = ErrorTracker::new();

        let config: ValidatedConfig =
            ValidatedConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.username, "validuser");
        assert_eq!(config.age, 30);
    }

    #[test]
    fn context_preserves_custom_source_and_level() {
        let source = TestSource::Package {
            name: "test-pkg".to_string(),
            version: "3.0.0".to_string(),
        };
        let level = TestLevel::Project;

        let value = cv_string_both("test_value", source.clone(), level.clone());

        // Verify context is preserved
        assert_eq!(value.context().source, source);
        assert_eq!(value.context().level, level);
        assert_eq!(
            value.context().source.display_name(),
            "package:test-pkg@3.0.0"
        );
        assert_eq!(
            compote::CustomLevel::name(&value.context().level),
            "project"
        );
    }

    #[test]
    fn source_file_path_works_with_custom_source() {
        let source = TestSource::File(PathBuf::from("/home/user/.config/app.yaml"));
        let level = TestLevel::User;

        let value = cv_string_both("test", source.clone(), level);

        // Verify file_path extraction works
        assert_eq!(
            value.context().source.file_path(),
            Some(Path::new("/home/user/.config/app.yaml"))
        );
    }

    #[test]
    fn source_file_path_returns_none_for_non_file_source() {
        let source = TestSource::Package {
            name: "pkg".to_string(),
            version: "1.0.0".to_string(),
        };
        let level = TestLevel::Base;

        let value = cv_string_both("test", source, level);

        // Package source has no file path
        assert_eq!(value.context().source.file_path(), None);
    }
}

// ============================================================================
// Tests: Mixed scenarios
// ============================================================================

// ============================================================================
// Tests: mutable_by with Custom Levels
// ============================================================================

mod mutable_by_custom_levels {
    use super::*;
    use compote::{Config as DeriveConfig, ConfigLoaderBuilder, Format};

    /// Config struct with mutable_by constraints using custom level names.
    ///
    /// TestLevel has names: "base", "team", "project", "user"
    #[derive(DeriveConfig, Debug, PartialEq)]
    struct MutableByTestConfig {
        /// Any level can set this
        #[compote(default = "default_any")]
        any_level: String,

        /// Only base level can set this
        #[compote(mutable_by = ["base"], default = "default_base_only")]
        base_only: String,

        /// Only team level can set this
        #[compote(mutable_by = ["team"], default = "default_team_only")]
        team_only: String,

        /// Only project level can set this
        #[compote(mutable_by = ["project"], default = "default_project_only")]
        project_only: String,

        /// Only user level can set this
        #[compote(mutable_by = ["user"], default = "default_user_only")]
        user_only: String,

        /// Team or project can set
        #[compote(mutable_by = ["team", "project"], default = "default_team_project")]
        team_or_project: String,

        /// Base, team, or user can set (skip project)
        #[compote(mutable_by = ["base", "team", "user"], default = "default_skip_project")]
        skip_project: String,
    }

    const BASE_CONFIG: &str = r#"
any_level: "base_any"
base_only: "base_base_only"
team_only: "base_team_only_SKIP"
project_only: "base_project_only_SKIP"
user_only: "base_user_only_SKIP"
team_or_project: "base_team_project_SKIP"
skip_project: "base_skip_project"
"#;

    const TEAM_CONFIG: &str = r#"
any_level: "team_any"
base_only: "team_base_only_SKIP"
team_only: "team_team_only"
project_only: "team_project_only_SKIP"
user_only: "team_user_only_SKIP"
team_or_project: "team_team_project"
skip_project: "team_skip_project"
"#;

    const PROJECT_CONFIG: &str = r#"
any_level: "project_any"
base_only: "project_base_only_SKIP"
team_only: "project_team_only_SKIP"
project_only: "project_project_only"
user_only: "project_user_only_SKIP"
team_or_project: "project_team_project"
skip_project: "project_skip_project_SKIP"
"#;

    const USER_CONFIG: &str = r#"
any_level: "user_any"
base_only: "user_base_only_SKIP"
team_only: "user_team_only_SKIP"
project_only: "user_project_only_SKIP"
user_only: "user_user_only"
team_or_project: "user_team_project_SKIP"
skip_project: "user_skip_project"
"#;

    #[test]
    fn mutable_by_enforces_custom_level_names() {
        // Load all levels in order: Base → Team → Project → User
        let mut loader = ConfigLoaderBuilder::<compote::Source, TestLevel>::new()
            .load_str(BASE_CONFIG, Format::Yaml, TestLevel::Base)
            .expect("Failed to load base config")
            .load_str(TEAM_CONFIG, Format::Yaml, TestLevel::Team)
            .expect("Failed to load team config")
            .load_str(PROJECT_CONFIG, Format::Yaml, TestLevel::Project)
            .expect("Failed to load project config")
            .load_str(USER_CONFIG, Format::Yaml, TestLevel::User)
            .expect("Failed to load user config");

        let config: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        // any_level: Last wins (user)
        assert_eq!(config.any_level, "user_any");

        // base_only: Only base allowed, so base value persists
        assert_eq!(config.base_only, "base_base_only");

        // team_only: Only team allowed
        assert_eq!(config.team_only, "team_team_only");

        // project_only: Only project allowed
        assert_eq!(config.project_only, "project_project_only");

        // user_only: Only user allowed
        assert_eq!(config.user_only, "user_user_only");

        // team_or_project: Both allowed, project is last so it wins
        assert_eq!(config.team_or_project, "project_team_project");

        // skip_project: base, team, user allowed; user is last allowed
        assert_eq!(config.skip_project, "user_skip_project");
    }

    #[test]
    fn mutable_by_records_warnings_for_custom_levels() {
        let mut loader = ConfigLoaderBuilder::<compote::Source, TestLevel>::new()
            .load_str(BASE_CONFIG, Format::Yaml, TestLevel::Base)
            .expect("Failed to load base config")
            .load_str(TEAM_CONFIG, Format::Yaml, TestLevel::Team)
            .expect("Failed to load team config");

        let _config: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        let warnings = loader.errors().warnings();

        // Count warnings mentioning specific levels
        let has_base_warning = warnings
            .iter()
            .any(|w| w.message.contains("base") && w.path.contains("team_only"));
        let has_team_warning = warnings
            .iter()
            .any(|w| w.message.contains("team") && w.path.contains("base_only"));

        // Base should warn when trying to set team_only
        assert!(
            has_base_warning,
            "Expected warning for base trying to set team_only field"
        );
        // Team should warn when trying to set base_only
        assert!(
            has_team_warning,
            "Expected warning for team trying to set base_only field"
        );
    }

    #[test]
    fn mutable_by_works_with_single_level_load() {
        // Load only team config
        let mut loader = ConfigLoaderBuilder::<compote::Source, TestLevel>::new()
            .load_str(TEAM_CONFIG, Format::Yaml, TestLevel::Team)
            .expect("Failed to load team config");

        let config: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        // team_only should be set
        assert_eq!(config.team_only, "team_team_only");

        // base_only should be default (team not allowed)
        assert_eq!(config.base_only, "default_base_only");

        // any_level should be set
        assert_eq!(config.any_level, "team_any");
    }

    #[test]
    fn mutable_by_with_defaults_on_skipped_fields() {
        // Load only user config - most fields should be skipped
        let mut loader = ConfigLoaderBuilder::<compote::Source, TestLevel>::new()
            .load_str(USER_CONFIG, Format::Yaml, TestLevel::User)
            .expect("Failed to load user config");

        let config: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        // user_only should be set
        assert_eq!(config.user_only, "user_user_only");

        // base_only, team_only, project_only should be defaults
        assert_eq!(config.base_only, "default_base_only");
        assert_eq!(config.team_only, "default_team_only");
        assert_eq!(config.project_only, "default_project_only");

        // team_or_project should be default (user not allowed)
        assert_eq!(config.team_or_project, "default_team_project");
    }

    /// Test that mutable_by works with both custom Source AND custom Level
    #[test]
    fn mutable_by_with_both_custom_source_and_level() {
        // Use fully custom types
        let mut loader = ConfigLoaderBuilder::<TestSource, TestLevel>::new()
            .load_str(BASE_CONFIG, Format::Yaml, TestLevel::Base)
            .expect("Failed to load base config")
            .load_str(USER_CONFIG, Format::Yaml, TestLevel::User)
            .expect("Failed to load user config");

        let config: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        // user_only should be from user
        assert_eq!(config.user_only, "user_user_only");

        // base_only should be from base (user not allowed)
        assert_eq!(config.base_only, "base_base_only");
    }

    /// Test warning message format includes custom level names
    #[test]
    fn warning_messages_include_custom_level_names() {
        let config_str = r#"
base_only: "from_team"
"#;

        let mut loader = ConfigLoaderBuilder::<compote::Source, TestLevel>::new()
            .load_str(config_str, Format::Yaml, TestLevel::Team)
            .expect("Failed to load config");

        let _: MutableByTestConfig = loader.deserialize().expect("Failed to deserialize");

        let warnings = loader.errors().warnings();
        assert_eq!(warnings.len(), 1, "Expected exactly 1 warning");

        let warning = &warnings[0];
        // Warning should mention "team" (the level that tried to set it)
        assert!(
            warning.message.contains("team"),
            "Warning should mention 'team' level: {}",
            warning.message
        );
        // Warning should mention "base" (the only allowed level)
        assert!(
            warning.message.contains("base"),
            "Warning should mention 'base' as allowed level: {}",
            warning.message
        );
    }
}

mod mixed_scenarios {
    use super::*;
    use indexmap::IndexMap;

    /// Test that we can define a struct that specifically requires custom types
    /// by implementing FromContextValue manually with specific type parameters
    #[derive(Debug, PartialEq)]
    struct PackageOnlyConfig {
        name: String,
        package_name: Option<String>,
        package_version: Option<String>,
    }

    impl FromContextValue<TestSource, TestLevel> for PackageOnlyConfig {
        fn from_context_value(
            value: &ContextValue<TestSource, TestLevel>,
            tracker: &mut ErrorTracker,
        ) -> Result<Self, compote::Error> {
            let obj = value
                .as_object()
                .ok_or_else(|| compote::Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "object".to_string(),
                    actual: value.type_name().to_string(),
                })?;

            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "default".to_string());

            // Extract package info from the source if available
            let (package_name, package_version) = match &value.context().source {
                TestSource::Package { name, version } => {
                    (Some(name.clone()), Some(version.clone()))
                }
                _ => (None, None),
            };

            Ok(PackageOnlyConfig {
                name,
                package_name,
                package_version,
            })
        }
    }

    #[test]
    fn manual_impl_can_access_custom_source_details() {
        let source = TestSource::Package {
            name: "my-package".to_string(),
            version: "1.2.3".to_string(),
        };
        let level = TestLevel::Base;

        let mut map = IndexMap::new();
        map.insert(
            "name".to_string(),
            cv_string_both("config_name", source.clone(), level.clone()),
        );
        let value = cv_object_both(map, source, level);

        let mut tracker = ErrorTracker::new();
        let config: PackageOnlyConfig =
            PackageOnlyConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "config_name");
        assert_eq!(config.package_name, Some("my-package".to_string()));
        assert_eq!(config.package_version, Some("1.2.3".to_string()));
    }

    #[test]
    fn manual_impl_works_with_non_package_source() {
        let source = TestSource::Environment;
        let level = TestLevel::User;

        let mut map = IndexMap::new();
        map.insert(
            "name".to_string(),
            cv_string_both("env_config", source.clone(), level.clone()),
        );
        let value = cv_object_both(map, source, level);

        let mut tracker = ErrorTracker::new();
        let config: PackageOnlyConfig =
            PackageOnlyConfig::from_context_value(&value, &mut tracker).unwrap();

        assert_eq!(config.name, "env_config");
        assert_eq!(config.package_name, None);
        assert_eq!(config.package_version, None);
    }

    /// Test that macro-generated and manual impls can coexist
    #[test]
    fn macro_and_manual_impls_work_together() {
        let source = TestSource::Package {
            name: "combined".to_string(),
            version: "0.1.0".to_string(),
        };
        let level = TestLevel::Project;

        // Use macro-generated SimpleConfig
        let mut simple_map = IndexMap::new();
        simple_map.insert(
            "name".to_string(),
            cv_string_both("simple", source.clone(), level.clone()),
        );
        simple_map.insert(
            "count".to_string(),
            cv_int_both(123, source.clone(), level.clone()),
        );
        let simple_value = cv_object_both(simple_map, source.clone(), level.clone());

        let mut tracker = ErrorTracker::new();
        let simple: SimpleConfig =
            SimpleConfig::from_context_value(&simple_value, &mut tracker).unwrap();
        assert_eq!(simple.name, "simple");
        assert_eq!(simple.count, 123);

        // Use manual PackageOnlyConfig
        let mut pkg_map = IndexMap::new();
        pkg_map.insert(
            "name".to_string(),
            cv_string_both("pkg", source.clone(), level.clone()),
        );
        let pkg_value = cv_object_both(pkg_map, source, level);

        let mut tracker = ErrorTracker::new();
        let pkg: PackageOnlyConfig =
            PackageOnlyConfig::from_context_value(&pkg_value, &mut tracker).unwrap();
        assert_eq!(pkg.name, "pkg");
        assert_eq!(pkg.package_name, Some("combined".to_string()));
    }
}

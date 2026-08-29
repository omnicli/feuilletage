//! Configuration container module.
//!
//! This module provides the main [`struct@Config`] struct that holds configuration data
//! and provides methods for loading, merging, and deserializing configuration.
//!
//! # Overview
//!
//! The [`struct@Config`] struct is the primary interface for working with configuration data.
//! It maintains a tree of [`ContextValue`] nodes, each with associated
//! metadata about its source, priority level, and mutability constraints.
//!
//! # Key Features
//!
//! - **Multi-format loading**: Load configuration from JSON, YAML, or TOML strings or files
//! - **Smart merging**: Recursively merge objects, replace primitives and arrays
//! - **Path-based access**: Navigate to nested values using dot notation (`"server.host"`)
//! - **Edit API**: Fluent interface for creating and modifying configuration values
//! - **Transformation support**: Apply transformations to values based on their path
//! - **Format-aware serialization**: Serialize back to the format the config was loaded from
//!
//! # Usage Examples
//!
//! See the struct-level documentation for [`struct@Config`] for detailed examples.

#[cfg(feature = "std")]
use std::ffi::OsString;
#[cfg(feature = "std")]
use std::fs::{self, File, OpenOptions};
#[cfg(feature = "std")]
use std::io::{self, Write};
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

#[cfg(feature = "std")]
use atomic_write_file::AtomicWriteFile;
#[cfg(feature = "std")]
use fs4::FileExt;

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};

use crate::{
    context::{Context, Format, Level, LevelType, MutabilityConstraint, Source, SourceType},
    edit::{ConfigEntry, IntoPath},
    error::{Error, ErrorTracker},
    merge::merge_values,
    transform::TransformRegistry,
    value::{ContextValue, MergeModifier},
};

#[cfg(any(feature = "std", feature = "json", feature = "yaml", feature = "toml"))]
use crate::loader;

/// Main configuration container.
///
/// `Config` is the primary interface for working with configuration data. It provides:
///
/// - Loading configuration from JSON, YAML, or TOML strings or files
/// - Merging multiple configuration sources with priority handling
/// - Path-based value access and modification via [`get`](Self::get) and [`get_mut`](Self::get_mut)
/// - Edit API via [`at`](Self::at) for fluent navigation and modification
/// - Deserialization into typed Rust structs via [`deserialize`](Self::deserialize)
/// - Transformation support for path-specific value processing
/// - Format-aware serialization (serialize back to the loaded format)
/// - File operations for reading and writing configuration files (with `std` feature)
///
/// # Configuration Tree Structure
///
/// Configuration is stored as a tree of [`ContextValue`] nodes. Each node contains:
/// - A [`crate::Value`] (the actual data: string, number, bool, array, or object)
/// - A [`Context`] with metadata (source, level, format, mutability)
///
/// # Priority and Merging
///
/// When loading multiple configurations, later loads override earlier ones.
/// The [`Level`] tracks where each value came from:
/// - `System`: System-wide defaults (lowest priority)
/// - `User`: User-specific settings
/// - `Local`: Project-local settings (highest priority)
///
/// Objects are merged recursively (keys from both are preserved), while
/// primitives and arrays are replaced entirely.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// # #[cfg(feature = "json")] {
/// use feuilletage::{Config, Context, Level, Source, FromContextValue};
///
/// #[derive(Debug, feuilletage::Config)]
/// struct ServerConfig {
///     #[feuilletage(default = "localhost")]
///     host: String,
///     #[feuilletage(default = "8080")]
///     port: i32,
/// }
///
/// let mut config = Config::default();
/// config.load_json(
///     r#"{"host": "0.0.0.0", "port": 3000}"#,
///     Context::new(Source::Programmatic, Level::User),
/// );
///
/// let server: ServerConfig = config.deserialize().unwrap();
/// assert_eq!(server.host, "0.0.0.0");
/// assert_eq!(server.port, 3000);
/// # }
/// ```
///
/// ## Merging Configurations
///
/// ```
/// # #[cfg(feature = "json")] {
/// use feuilletage::{Config, Context, ContextValue, Level, Source};
///
/// let mut config = Config::default();
///
/// // Load base configuration
/// config.load_json(
///     r#"{"database": {"host": "localhost", "port": 5432}}"#,
///     Context::new(Source::Programmatic, Level::System),
/// );
///
/// // Override with user configuration
/// config.load_json(
///     r#"{"database": {"port": 5433}}"#,
///     Context::new(Source::Programmatic, Level::User),
/// );
///
/// // database.host is still "localhost", database.port is now 5433
/// let port_value = config.get("database.port").unwrap();
/// assert!(matches!(port_value, ContextValue::Int(5433, _)));
/// # }
/// ```
///
/// ## Using the Edit API
///
/// ```
/// use feuilletage::{Config, ContextValue};
///
/// let mut config = Config::default();
///
/// // Create nested structure and set values
/// config.at("server.host").set("localhost").unwrap();
/// config.at("server.port").set(8080).unwrap();
///
/// // Read values back
/// if let Some(ContextValue::String(host, _)) = config.at("server.host").get() {
///     assert_eq!(host, "localhost");
/// }
///
/// // Create a path to remove
/// config.at("old.unused.key").set("value").unwrap();
/// assert!(config.at("old.unused.key").exists());
///
/// // Remove with pruning (removes empty parent objects)
/// config.at("old.unused.key").remove().prune();
/// assert!(!config.at("old").exists()); // Parent also pruned
/// ```
/// Generic parameters:
/// - `S`: Source type (implements [`SourceType`], defaults to [`Source`])
/// - `L`: Level type (implements [`LevelType`], defaults to [`Level`])
///
/// Both parameters default to the built-in types for simple usage.
/// For custom source/level types, implement [`crate::CustomSource`] or [`crate::CustomLevel`]
/// and use them as type parameters.
#[derive(Debug)]
pub struct Config<S: SourceType = Source, L: LevelType = Level> {
    /// The root configuration value
    root: ContextValue<S, L>,
    /// Error tracker for all operations
    tracker: ErrorTracker,
    /// Transformation registry for path-specific transformations
    transforms: TransformRegistry<S, L>,
    /// The format of the most recently loaded configuration
    last_loaded_format: Format,
    /// The preferred format when no source or path determines one
    default_format: Format,
    /// Files that were successfully loaded
    #[cfg(feature = "std")]
    loaded_files: Vec<PathBuf>,
}

impl<S: SourceType, L: LevelType> Config<S, L> {
    /// Creates a new empty configuration with the given context.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let ctx = Context::new(Source::Programmatic, Level::User);
    /// let config: Config = Config::new(ctx);
    /// assert!(!config.has_errors());
    /// ```
    pub fn new(context: Context<S, L>) -> Self {
        Self {
            root: ContextValue::object(Default::default(), context),
            tracker: ErrorTracker::new(),
            transforms: TransformRegistry::new(),
            last_loaded_format: Format::Unknown,
            default_format: Format::default_format(),
            #[cfg(feature = "std")]
            loaded_files: Vec::new(),
        }
    }

    /// Loads configuration from a file and merges it into the current configuration.
    ///
    /// The file format is detected from the file extension:
    /// - `.json` for JSON
    /// - `.yaml` or `.yml` for YAML
    /// - `.toml` for TOML
    ///
    /// # Returns
    ///
    /// - `true` - File was loaded successfully and merged
    /// - `false` - File was skipped (doesn't exist, couldn't be read, format error, or parse error)
    ///
    /// When a file fails to load due to format or parse errors, the error is recorded
    /// in the error tracker (accessible via [`errors()`](Self::errors)) rather than
    /// being returned. This allows loading to continue with other files.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[cfg(feature = "std")] {
    /// use feuilletage::{Config, Level};
    ///
    /// let mut config = Config::default();
    ///
    /// // Load files - missing files are silently skipped
    /// config.load_file("/etc/app/config.yaml", Level::System);
    /// config.load_file("~/.config/app/config.yaml", Level::User);
    ///
    /// // Check which files were actually loaded
    /// for path in config.loaded_files() {
    ///     println!("Loaded: {}", path.display());
    /// }
    ///
    /// // Check for any parse/format errors
    /// if config.has_errors() {
    ///     for error in config.get_errors() {
    ///         eprintln!("Error: {}", error);
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// Loads configuration from a JSON string and merges it into the current configuration.
    ///
    /// Parse errors are recorded in the error tracker rather than returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, ContextValue, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"name": "test", "count": 42}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let name = config.get("name").unwrap();
    /// assert!(matches!(name, ContextValue::String(s, _) if s == "test"));
    /// # }
    /// ```
    #[cfg(feature = "json")]
    pub fn load_json(&mut self, content: &str, context: Context<S, L>) {
        match loader::load_json(content, context) {
            Ok(new_config) => {
                self.merge(new_config);
                self.set_loaded_format(Format::Json);
            }
            Err(e) => self.tracker.record(e),
        }
    }

    /// Loads configuration from a YAML string and merges it into the current configuration.
    ///
    /// Parse errors are recorded in the error tracker rather than returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{Config, ContextValue, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_yaml(
    ///     "name: test\ncount: 42",
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let name = config.get("name").unwrap();
    /// assert!(matches!(name, ContextValue::String(s, _) if s == "test"));
    /// # }
    /// ```
    #[cfg(feature = "yaml")]
    pub fn load_yaml(&mut self, content: &str, context: Context<S, L>) {
        match loader::load_yaml(content, context) {
            Ok(new_config) => {
                self.merge(new_config);
                self.set_loaded_format(Format::Yaml);
            }
            Err(e) => self.tracker.record(e),
        }
    }

    /// Loads configuration from a TOML string and merges it into the current configuration.
    ///
    /// Parse errors are recorded in the error tracker rather than returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "toml")] {
    /// use feuilletage::{Config, ContextValue, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_toml(
    ///     r#"name = "test"
    /// count = 42"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let name = config.get("name").unwrap();
    /// assert!(matches!(name, ContextValue::String(s, _) if s == "test"));
    /// # }
    /// ```
    #[cfg(feature = "toml")]
    pub fn load_toml(&mut self, content: &str, context: Context<S, L>) {
        match loader::load_toml(content, context) {
            Ok(new_config) => {
                self.merge(new_config);
                self.set_loaded_format(Format::Toml);
            }
            Err(e) => self.tracker.record(e),
        }
    }

    /// Merges another configuration value into this one.
    ///
    /// The merge follows these rules:
    /// - Objects are merged recursively (keys from both are kept, new values override old)
    /// - Arrays and primitives are replaced entirely
    /// - Immutable values cannot be overwritten
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, ContextValue, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    ///
    /// config.load_json(
    ///     r#"{"a": 1, "b": 2}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    /// config.load_json(
    ///     r#"{"b": 3, "c": 4}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// // a=1, b=3 (overridden), c=4 (added)
    /// assert!(matches!(config.get("a").unwrap(), ContextValue::Int(1, _)));
    /// assert!(matches!(config.get("b").unwrap(), ContextValue::Int(3, _)));
    /// assert!(matches!(config.get("c").unwrap(), ContextValue::Int(4, _)));
    /// # }
    /// ```
    pub fn merge(&mut self, new_config: ContextValue<S, L>) {
        merge_values(
            &mut self.root,
            new_config,
            MergeModifier::Default,
            &mut self.tracker,
        );
    }

    /// Returns a reference to the root configuration value.
    pub fn root(&self) -> &ContextValue<S, L> {
        &self.root
    }

    /// Returns a mutable reference to the root configuration value.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue, Value};
    ///
    /// let mut config = Config::default();
    /// if let ContextValue::Object(values, _) = config.root_mut() {
    ///     values.insert("enabled".into(), Value::Bool(true).into());
    /// }
    /// assert_eq!(config.get("enabled").and_then(ContextValue::as_bool), Some(true));
    /// ```
    pub fn root_mut(&mut self) -> &mut ContextValue<S, L> {
        &mut self.root
    }

    /// Navigate to a path in the config tree for reading or modification.
    ///
    /// Returns a [`ConfigEntry`] that can be used to read, write, or delete
    /// values at the specified path.
    ///
    /// # Path Syntax
    ///
    /// - Dot-separated string: `"a.b.c"` -> segments `["a", "b", "c"]`
    /// - Escaped dots: `"a.b\\.c"` -> segments `["a", "b.c"]`
    /// - Array of segments: `&["a", "b.c"]` -> segments `["a", "b.c"]`
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    ///
    /// // Set a nested value
    /// config.at("server.host").set("localhost").unwrap();
    ///
    /// // Read a value
    /// if let Some(ContextValue::String(host, _)) = config.at("server.host").get() {
    ///     assert_eq!(host, "localhost");
    /// }
    ///
    /// // Chain navigation
    /// config.at("a").at("b").at("c").set(42).unwrap();
    /// assert!(config.at("a.b.c").exists());
    /// ```
    pub fn at<P: IntoPath>(&mut self, path: P) -> ConfigEntry<'_, S, L> {
        ConfigEntry::new(self, path.into_path())
    }

    /// Navigate to a single key without dot splitting.
    ///
    /// Useful when the key itself contains dots that should not be
    /// interpreted as path separators.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    ///
    /// // Set a value with a key containing dots
    /// config.at("plugins").at_raw("auth.v2").at("enabled").set(true).unwrap();
    /// // Path is: ["plugins", "auth.v2", "enabled"]
    ///
    /// // Verify the path structure - "auth.v2" is a single key
    /// assert!(config.at("plugins").at_raw("auth.v2").at("enabled").exists());
    /// ```
    pub fn at_raw(&mut self, key: &str) -> ConfigEntry<'_, S, L> {
        let path = if key.is_empty() {
            Vec::new()
        } else {
            vec![key.to_string()]
        };
        ConfigEntry::new(self, path)
    }

    /// Returns a reference to the error tracker.
    pub fn errors(&self) -> &ErrorTracker {
        &self.tracker
    }

    /// Returns a mutable reference to the error tracker.
    pub fn errors_mut(&mut self) -> &mut ErrorTracker {
        &mut self.tracker
    }

    /// Returns `true` if any errors have been recorded during configuration operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let config = Config::default();
    /// assert!(!config.has_errors());
    /// ```
    pub fn has_errors(&self) -> bool {
        self.tracker.has_errors()
    }

    /// Returns all recorded errors as a slice.
    pub fn get_errors(&self) -> &[Error] {
        self.tracker.errors()
    }

    /// Clears all recorded errors.
    pub fn clear_errors(&mut self) {
        self.tracker.clear();
    }

    /// Consumes the config and returns all recorded errors.
    pub fn into_errors(self) -> Vec<Error> {
        self.tracker.into_errors()
    }

    /// Deserializes the configuration into a typed struct.
    ///
    /// The type must implement [`FromContextValue`](crate::FromContextValue), which is
    /// automatically derived when using `#[derive(Config)]`.
    ///
    /// # Accessing Errors After Deserialization
    ///
    /// This method takes `&mut self` so that errors recorded during deserialization
    /// are accumulated in the Config's internal tracker and can be accessed afterward
    /// via [`errors()`](Self::errors).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source, FromContextValue};
    ///
    /// #[derive(Debug, feuilletage::Config, PartialEq)]
    /// struct DbConfig {
    ///     #[feuilletage(default = "localhost")]
    ///     host: String,
    ///     #[feuilletage(default = "5432")]
    ///     port: i32,
    /// }
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"host": "db.example.com"}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let db: DbConfig = config.deserialize().unwrap();
    /// assert_eq!(db.host, "db.example.com");
    /// assert_eq!(db.port, 5432); // default value
    ///
    /// // Check for any errors or warnings recorded during deserialization
    /// if config.errors().has_errors() {
    ///     for error in config.errors().errors() {
    ///         eprintln!("Error: {}", error);
    ///     }
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails due to missing required fields,
    /// type mismatches, or validation failures.
    pub fn deserialize<T: crate::de::FromContextValue<S, L>>(&mut self) -> Result<T, Error> {
        T::from_context_value(&self.root, &mut self.tracker)
    }

    /// Sets a mutability constraint for a specific path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    ///
    /// ```
    /// use feuilletage::{Config, MutabilityConstraint};
    ///
    /// let mut config = Config::default();
    /// config.at("port").set(8080).unwrap();
    /// config
    ///     .set_mutability_constraint("port", MutabilityConstraint::Immutable)
    ///     .unwrap();
    ///
    /// assert_eq!(
    ///     config.get("port").unwrap().context().mutability,
    ///     MutabilityConstraint::Immutable,
    /// );
    /// ```
    pub fn set_mutability_constraint(
        &mut self,
        path: &str,
        constraint: MutabilityConstraint,
    ) -> Result<(), Error> {
        self.navigate_and_modify(path, |value| {
            value.context_mut().mutability = constraint.clone();
            Ok(())
        })
    }

    /// Makes a path immutable, preventing any future modifications.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{Config, Context, ContextValue, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_yaml(
    ///     "port: 8080\n",
    ///     Context::new(Source::Programmatic, Level::System),
    /// );
    /// config.make_immutable("port").unwrap();
    /// config.load_yaml(
    ///     "port: 3000\n",
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(8080, _))));
    /// assert!(config.has_errors());
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn make_immutable(&mut self, path: &str) -> Result<(), Error> {
        self.set_mutability_constraint(path, MutabilityConstraint::Immutable)
    }

    /// Makes a path mutable only by specific configuration level names.
    ///
    /// Level names are matched against the `name()` method of levels,
    /// e.g., "system", "user", "local", or custom level names.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{Config, Context, ContextValue, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_yaml(
    ///     "port: 8080\n",
    ///     Context::new(Source::Programmatic, Level::System),
    /// );
    /// config.make_mutable_by("port", &["local"]).unwrap();
    ///
    /// config.load_yaml(
    ///     "port: 3000\n",
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(8080, _))));
    ///
    /// config.load_yaml(
    ///     "port: 4000\n",
    ///     Context::new(Source::Programmatic, Level::Local),
    /// );
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(4000, _))));
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist.
    pub fn make_mutable_by(&mut self, path: &str, level_names: &[&str]) -> Result<(), Error> {
        self.set_mutability_constraint(path, MutabilityConstraint::mutable_by(level_names))
    }

    /// Navigate to a path and modify the value
    fn navigate_and_modify<F>(&mut self, path: &str, mut modify: F) -> Result<(), Error>
    where
        F: FnMut(&mut ContextValue<S, L>) -> Result<(), Error>,
    {
        let parts: Vec<&str> = path.split('.').collect();
        Self::navigate_and_modify_recursive(&mut self.root, &parts, 0, &mut modify)
    }

    fn navigate_and_modify_recursive<F>(
        current: &mut ContextValue<S, L>,
        parts: &[&str],
        index: usize,
        modify: &mut F,
    ) -> Result<(), Error>
    where
        F: FnMut(&mut ContextValue<S, L>) -> Result<(), Error>,
    {
        if index >= parts.len() {
            return modify(current);
        }

        let part = parts[index];

        match current {
            ContextValue::Object(ref mut map, _) => {
                if let Some(child) = map.get_mut(part) {
                    Self::navigate_and_modify_recursive(child, parts, index + 1, modify)
                } else {
                    Err(Error::InvalidValue {
                        path: parts[..=index].join("."),
                        message: format!("Path not found: {}", part),
                    })
                }
            }
            ContextValue::Array(ref mut arr, _) => {
                if let Ok(idx) = part.parse::<usize>() {
                    if let Some(child) = arr.get_mut(idx) {
                        Self::navigate_and_modify_recursive(child, parts, index + 1, modify)
                    } else {
                        Err(Error::InvalidValue {
                            path: parts[..=index].join("."),
                            message: format!("Array index out of bounds: {}", idx),
                        })
                    }
                } else {
                    Err(Error::InvalidValue {
                        path: parts[..=index].join("."),
                        message: format!("Invalid array index: {}", part),
                    })
                }
            }
            _ => Err(Error::InvalidValue {
                path: parts[..index].join("."),
                message: format!("Cannot navigate through non-object/array at: {}", part),
            }),
        }
    }

    /// Get a value at the specified dot-separated path
    ///
    /// Path format: dot-separated keys like "a.b.c" or "items.0.name" for array indices.
    ///
    /// Returns `None` if the path doesn't exist or if trying to traverse through a non-object/array.
    ///
    /// # Example
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Source, Level};
    ///
    /// let mut config = Config::default();
    /// let ctx = Context::new(Source::Programmatic, Level::User);
    /// config.load_json(r#"{"a": {"b": {"c": 42}}}"#, ctx);
    ///
    /// if let Some(value) = config.get("a.b.c") {
    ///     // Use the value
    /// }
    /// # }
    /// ```
    pub fn get(&self, path: &str) -> Option<&ContextValue<S, L>> {
        if path.is_empty() {
            return Some(&self.root);
        }
        let parts: Vec<&str> = path.split('.').collect();
        Self::get_recursive(&self.root, &parts, 0)
    }

    fn get_recursive<'a>(
        current: &'a ContextValue<S, L>,
        parts: &[&str],
        index: usize,
    ) -> Option<&'a ContextValue<S, L>> {
        if index >= parts.len() {
            return Some(current);
        }

        let part = parts[index];

        match current {
            ContextValue::Object(map, _) => map
                .get(part)
                .and_then(|child| Self::get_recursive(child, parts, index + 1)),
            ContextValue::Array(arr, _) => part
                .parse::<usize>()
                .ok()
                .and_then(|idx| arr.get(idx))
                .and_then(|child| Self::get_recursive(child, parts, index + 1)),
            _ => None,
        }
    }

    /// Get a mutable reference to a value at the specified dot-separated path
    ///
    /// Path format: dot-separated keys like "a.b.c" or "items.0.name" for array indices.
    ///
    /// Returns `None` if the path doesn't exist or if trying to traverse through a non-object/array.
    ///
    /// # Example
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, ContextValue, Context, Source, Level};
    ///
    /// let mut config = Config::default();
    /// let ctx = Context::new(Source::Programmatic, Level::User);
    /// config.load_json(r#"{"a": {"b": {"c": 42}}}"#, ctx);
    ///
    /// if let Some(ContextValue::Int(ref mut n, _)) = config.get_mut("a.b.c") {
    ///     *n = 100;
    /// }
    /// # }
    /// ```
    pub fn get_mut(&mut self, path: &str) -> Option<&mut ContextValue<S, L>> {
        if path.is_empty() {
            return Some(&mut self.root);
        }
        let parts: Vec<&str> = path.split('.').collect();
        Self::get_mut_recursive(&mut self.root, &parts, 0)
    }

    fn get_mut_recursive<'a>(
        current: &'a mut ContextValue<S, L>,
        parts: &[&str],
        index: usize,
    ) -> Option<&'a mut ContextValue<S, L>> {
        if index >= parts.len() {
            return Some(current);
        }

        let part = parts[index];

        match current {
            ContextValue::Object(ref mut map, _) => map
                .get_mut(part)
                .and_then(|child| Self::get_mut_recursive(child, parts, index + 1)),
            ContextValue::Array(ref mut arr, _) => part
                .parse::<usize>()
                .ok()
                .and_then(|idx| arr.get_mut(idx))
                .and_then(|child| Self::get_mut_recursive(child, parts, index + 1)),
            _ => None,
        }
    }

    /// Serialize the raw configuration value tree using the loaded or preferred format.
    ///
    /// This serializes the internal ContextValue representation. To serialize a typed
    /// struct back to the loaded format, use [`Config::serialize`] instead.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_yaml(
    ///     "host: localhost\nport: 8080\n",
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let raw = config.serialize_raw().unwrap();
    /// assert!(raw.contains("localhost"));
    /// assert!(raw.contains("8080"));
    /// # }
    /// ```
    pub fn serialize_raw(&self) -> Result<String, Error> {
        self.root.serialize_with_format(self.effective_format())
    }

    #[cfg(any(feature = "std", feature = "json", feature = "yaml", feature = "toml"))]
    pub(crate) fn set_loaded_format(&mut self, format: Format) {
        self.root.context_mut().format = format.clone();
        self.last_loaded_format = format;
    }

    /// Serialize to JSON
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("enabled").set(true).unwrap();
    /// assert_eq!(config.to_json().unwrap(), "{\n  \"enabled\": true\n}");
    /// # }
    /// ```
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String, Error> {
        self.root.to_json()
    }

    /// Serialize to YAML
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("enabled").set(true).unwrap();
    /// assert_eq!(config.to_yaml().unwrap(), "enabled: true\n");
    /// # }
    /// ```
    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> Result<String, Error> {
        self.root.to_yaml()
    }

    /// Serialize to TOML
    ///
    /// ```
    /// # #[cfg(feature = "toml")] {
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("enabled").set(true).unwrap();
    /// assert_eq!(config.to_toml().unwrap(), "enabled = true\n");
    /// # }
    /// ```
    #[cfg(feature = "toml")]
    pub fn to_toml(&self) -> Result<String, Error> {
        self.root.to_toml()
    }

    /// Returns the format of the most recently loaded configuration.
    ///
    /// This tracks the format from the last successful `load_*` call.
    /// If no configuration has been loaded, returns `Format::Unknown`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source, Format};
    ///
    /// let mut config = Config::default();
    /// assert_eq!(config.loaded_format(), Format::Unknown);
    ///
    /// config.load_json(
    ///     r#"{"key": "value"}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    /// assert_eq!(config.loaded_format(), Format::Json);
    /// # }
    /// ```
    pub fn loaded_format(&self) -> Format {
        self.last_loaded_format.clone()
    }

    /// Returns the output format preferred when no loaded source or file
    /// extension determines one.
    ///
    /// ```
    /// use feuilletage::{Config, Format};
    ///
    /// let config = Config::default();
    /// assert_eq!(config.default_format(), Format::default_format());
    /// ```
    pub fn default_format(&self) -> Format {
        self.default_format.clone()
    }

    /// Sets the preferred output format.
    ///
    /// The selected format must be enabled and cannot be [`Format::Unknown`].
    ///
    /// ```
    /// use feuilletage::{Config, Format};
    ///
    /// let mut config = Config::default();
    /// #[cfg(feature = "toml")]
    /// {
    ///     config.set_default_format(Format::Toml).unwrap();
    ///     assert_eq!(config.default_format(), Format::Toml);
    /// }
    /// #[cfg(not(feature = "toml"))]
    /// assert!(config.set_default_format(Format::Toml).is_err());
    /// ```
    pub fn set_default_format(&mut self, format: Format) -> Result<(), Error> {
        format.ensure_enabled()?;
        self.default_format = format;
        Ok(())
    }

    /// Sets the preferred format and returns the updated config.
    ///
    /// ```
    /// # #[cfg(feature = "toml")] {
    /// use feuilletage::{Config, Format};
    ///
    /// let config = Config::default()
    ///     .with_default_format(Format::Toml)
    ///     .unwrap();
    /// assert_eq!(config.default_format(), Format::Toml);
    /// # }
    /// ```
    pub fn with_default_format(mut self, format: Format) -> Result<Self, Error> {
        self.set_default_format(format)?;
        Ok(self)
    }

    #[cfg(feature = "std")]
    pub(crate) fn set_default_format_unchecked(&mut self, format: Format) {
        self.default_format = format;
    }

    fn effective_format(&self) -> Format {
        match self.last_loaded_format {
            Format::Unknown => self.default_format.clone(),
            ref format => format.clone(),
        }
    }

    /// Serialize a value using the most recently loaded or preferred format.
    ///
    /// This is useful when you want to serialize a deserialized struct back
    /// to the same format it was loaded from.
    ///
    /// # Arguments
    ///
    /// * `value` - Any type that implements `serde::Serialize`
    ///
    /// # Returns
    ///
    /// A `Result` containing the serialized string, or a [`Error`] if:
    /// - Serialization fails
    /// - The loaded format's feature is not enabled
    /// - No configuration was loaded (format is Unknown) and no format features are enabled
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source, FromContextValue};
    ///
    /// // Note: feuilletage::Config already derives Serialize
    /// #[derive(Debug, feuilletage::Config)]
    /// struct AppConfig {
    ///     #[feuilletage(default = "localhost")]
    ///     host: String,
    ///     #[feuilletage(default = "8080")]
    ///     port: i32,
    /// }
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"host": "example.com"}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let app: AppConfig = config.deserialize().unwrap();
    ///
    /// // Serialize back to JSON (the format we loaded from)
    /// let json_output = config.serialize(&app).unwrap();
    /// assert!(json_output.contains("example.com"));
    /// # }
    /// ```
    pub fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<String, Error> {
        crate::ser::to_format(value, self.effective_format())
    }

    // ========================================================================
    // File Operations (std feature only)
    // ========================================================================

    /// Detect config format from file extension.
    ///
    /// Returns `None` if the extension is not recognized.
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    #[cfg(feature = "std")]
    fn format_from_extension(path: &Path) -> Option<Format> {
        let ext = path.extension().and_then(|s| s.to_str())?;
        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Some(Format::Yaml),
            "json" => Some(Format::Json),
            "toml" => Some(Format::Toml),
            _ => None,
        }
    }

    /// Serialize the config to a string in the specified format.
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    #[cfg(feature = "std")]
    fn serialize_as(&self, format: Format) -> Result<String, io::Error> {
        let format = match format {
            Format::Unknown => Format::default_format(),
            format => format,
        };

        match format {
            #[cfg(feature = "json")]
            Format::Json => self.to_json().map_err(|e| io::Error::other(e.to_string())),
            #[cfg(not(feature = "json"))]
            Format::Json => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "JSON feature not enabled",
            )),

            #[cfg(feature = "yaml")]
            Format::Yaml => self.to_yaml().map_err(|e| io::Error::other(e.to_string())),
            #[cfg(not(feature = "yaml"))]
            Format::Yaml => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "YAML feature not enabled",
            )),

            #[cfg(feature = "toml")]
            Format::Toml => self.to_toml().map_err(|e| io::Error::other(e.to_string())),
            #[cfg(not(feature = "toml"))]
            Format::Toml => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "TOML feature not enabled",
            )),

            Format::Unknown => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "no serialization format available",
            )),
        }
    }

    /// Write config to file with explicit format.
    ///
    /// Creates parent directories if they don't exist, coordinates with other
    /// Feuilletage writers through an exclusive sidecar lock, and atomically
    /// replaces the destination after the complete content is written.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write to
    /// * `format` - The format to serialize the config as
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The format is not supported (feature not enabled)
    /// - Parent directories cannot be created
    /// - The file cannot be written
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::{Config, Format};
    ///
    /// let config = Config::default();
    /// config.write_file_as("config.json", Format::Json).unwrap();
    /// ```
    #[cfg(feature = "std")]
    pub fn write_file_as<P: AsRef<Path>>(&self, path: P, format: Format) -> io::Result<()> {
        let path = path.as_ref();
        Self::create_parent_directories(path)?;
        let content = self.serialize_as(format)?;
        let _lock = Self::lock_file(path)?;
        Self::atomic_replace(path, |file| file.write_all(content.as_bytes()))
    }

    /// Write config to file with auto-detected format from extension.
    ///
    /// Format detection:
    /// - `.yaml`, `.yml` -> YAML
    /// - `.json` -> JSON
    /// - `.toml` -> TOML
    /// - Unknown extension -> Uses default format based on enabled features
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The detected format is not supported (feature not enabled)
    /// - Parent directories cannot be created
    /// - The file cannot be written
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::Config;
    ///
    /// let config = Config::default();
    /// config.write_file("config.yaml").unwrap(); // Writes as YAML
    /// config.write_file("config.json").unwrap(); // Writes as JSON
    /// config.write_file("config.toml").unwrap(); // Writes as TOML
    /// config.write_file("config.txt").unwrap();  // Uses default format
    /// ```
    #[cfg(feature = "std")]
    pub fn write_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        let format = Self::format_from_extension(path).unwrap_or_else(|| self.effective_format());
        self.write_file_as(path, format)
    }

    /// Edit a config file: load, apply closure, save if closure returns true.
    ///
    /// This provides a convenient read-modify-write workflow for config files.
    ///
    /// # Format Detection
    ///
    /// The format is detected from a recognized `.yaml`, `.yml`, `.json`, or
    /// `.toml` extension, whether or not the file already exists. Use
    /// [`edit_file_with_format`](Self::edit_file_with_format) for extensionless
    /// or nonstandard paths.
    ///
    /// # Behavior
    ///
    /// - Creates parent directories if needed
    /// - Holds an exclusive sidecar lock across the read-modify-write operation
    /// - Replaces the destination atomically, leaving the original intact on failure
    /// - Empty/missing file treated as empty config `{}`
    /// - If the closure returns `true`, the file is saved
    /// - If the closure returns `false`, no changes are written
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to edit
    /// * `edit_fn` - A closure that receives a mutable reference to the config.
    ///   Return `true` to save changes, `false` to discard.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Changes were saved
    /// * `Ok(false)` - Closure returned false, no changes written
    /// * `Err(_)` - An I/O error occurred
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::edit_file;
    ///
    /// // Basic edit (using module-level convenience function)
    /// edit_file("config.yaml", |config| {
    ///     config.at("server.port").set(8080).ok();
    ///     true // Save changes
    /// }).unwrap();
    ///
    /// // Conditional save
    /// edit_file("config.yaml", |config| {
    ///     if config.at("version").get().is_none() {
    ///         config.at("version").set(1).ok();
    ///         true // Save
    ///     } else {
    ///         false // Already has version, don't save
    ///     }
    /// }).unwrap();
    /// ```
    #[cfg(feature = "std")]
    pub fn edit_file<P, F>(path: P, edit_fn: F) -> io::Result<bool>
    where
        P: AsRef<Path>,
        F: FnOnce(&mut Config) -> bool,
    {
        let path = path.as_ref();
        let format = Self::format_from_extension(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuration input requires a recognized file extension or explicit format",
            )
        })?;

        Self::edit_file_with_format(path, format, edit_fn)
    }

    /// Edit a config file using an explicit format.
    ///
    /// ```no_run
    /// # #[cfg(all(feature = "std", feature = "yaml"))] {
    /// use feuilletage::{Config, Format, Level, Source};
    ///
    /// Config::<Source, Level>::edit_file_with_format("config.data", Format::Yaml, |config| {
    ///     config.at("server.port").set(8080).is_ok()
    /// })
    /// .unwrap();
    /// # }
    /// ```
    #[cfg(feature = "std")]
    pub fn edit_file_with_format<P, F>(path: P, format: Format, edit_fn: F) -> io::Result<bool>
    where
        P: AsRef<Path>,
        F: FnOnce(&mut Config) -> bool,
    {
        let path = path.as_ref();
        if format == Format::Unknown {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "configuration input requires an explicit format",
            ));
        }

        Self::create_parent_directories(path)?;
        let _lock = Self::lock_file(path)?;

        // Load existing config or create empty
        let mut config = if path.exists() {
            let content = fs::read_to_string(path)?;
            if content.trim().is_empty() {
                // Empty file - treat as empty config
                let mut config = Config::default();
                config.set_loaded_format(format.clone());
                config
            } else {
                // Parse existing content
                let context = Context::new(Source::File(path.to_path_buf()), Level::User)
                    .with_format(format.clone());
                let mut config = Config::new(context.clone());

                let parse_result = match format {
                    #[cfg(feature = "json")]
                    Format::Json => loader::load_json(&content, context),
                    #[cfg(not(feature = "json"))]
                    Format::Json => Err(Error::FormatNotSupported {
                        format: "json".to_string(),
                        message: "JSON feature not enabled".to_string(),
                    }),

                    #[cfg(feature = "yaml")]
                    Format::Yaml => loader::load_yaml(&content, context),
                    #[cfg(not(feature = "yaml"))]
                    Format::Yaml => Err(Error::FormatNotSupported {
                        format: "yaml".to_string(),
                        message: "YAML feature not enabled".to_string(),
                    }),

                    #[cfg(feature = "toml")]
                    Format::Toml => loader::load_toml(&content, context),
                    #[cfg(not(feature = "toml"))]
                    Format::Toml => Err(Error::FormatNotSupported {
                        format: "toml".to_string(),
                        message: "TOML feature not enabled".to_string(),
                    }),

                    Format::Unknown => unreachable!("unknown formats are rejected above"),
                };

                match parse_result {
                    Ok(root) => {
                        config.root = root;
                        config
                    }
                    Err(e) => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
                    }
                }
            }
        } else {
            // File doesn't exist - create empty config
            let mut config = Config::default();
            config.set_loaded_format(format.clone());
            config
        };

        // Apply edits
        let should_save = edit_fn(&mut config);

        // Save if closure returned true
        if should_save {
            let content = config.serialize_as(format)?;
            Self::atomic_replace(path, |file| file.write_all(content.as_bytes()))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Edit first existing file from list, or create first file if none exist.
    ///
    /// This is useful for configuration hierarchies where you want to edit
    /// whichever config file exists, or create a new one at the preferred location.
    ///
    /// # Behavior
    ///
    /// 1. Iterates through paths to find the first existing file
    /// 2. If found, edits that file
    /// 3. If no file exists, creates and edits the first path in the list
    /// 4. Returns the path that was edited (or None if the closure returned false
    ///    and no file was created)
    ///
    /// # Arguments
    ///
    /// * `paths` - A list of file paths to check, in order of preference
    /// * `edit_fn` - A closure that receives a mutable reference to the config.
    ///   Return `true` to save changes, `false` to discard.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(path))` - The path that was edited/created
    /// * `Ok(None)` - Closure returned false and no changes were made
    /// * `Err(_)` - An I/O error occurred
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::edit_first_existing;
    ///
    /// let paths = [
    ///     "~/.config/myapp/config.yaml",  // Preferred location
    ///     "~/.myapp.yaml",                 // Legacy location
    /// ];
    ///
    /// let edited_path = edit_first_existing(&paths, |config| {
    ///     config.at("setting").set("value").ok();
    ///     true
    /// }).unwrap();
    ///
    /// if let Some(path) = edited_path {
    ///     println!("Updated config at: {}", path.display());
    /// }
    /// ```
    #[cfg(feature = "std")]
    pub fn edit_first_existing<P, F>(paths: &[P], edit_fn: F) -> io::Result<Option<PathBuf>>
    where
        P: AsRef<Path>,
        F: FnOnce(&mut Config) -> bool,
    {
        if paths.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "No paths provided",
            ));
        }

        // Find first existing file
        let target_path = paths
            .iter()
            .find(|p| p.as_ref().exists())
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| {
                // No existing file - use the first path
                paths[0].as_ref().to_path_buf()
            });

        // Edit the file
        let saved = Self::edit_file(&target_path, edit_fn)?;

        if saved {
            Ok(Some(target_path))
        } else {
            Ok(None)
        }
    }

    /// Edit the first writeable file from a list of candidates.
    ///
    /// Iterates through the candidate paths and finds the first one that is writeable:
    /// - If file exists: checks read+write permissions
    /// - If file doesn't exist: checks if parent directory is writeable (will create file)
    ///
    /// Once found, atomically performs the edit. This avoids race conditions between
    /// finding a writeable file and editing it.
    ///
    /// # Arguments
    ///
    /// * `candidates` - A list of file paths to check, in order of preference
    /// * `edit_fn` - A closure that receives a mutable reference to the config.
    ///   Return `true` to save changes, `false` to discard.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(path))` - The path that was edited
    /// * `Ok(None)` - No writeable file was found among the candidates
    /// * `Err(_)` - An I/O error occurred during editing
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::edit_first_writeable;
    ///
    /// let candidates = [
    ///     "/etc/myapp/config.yaml",        // System config (may not be writeable)
    ///     "~/.config/myapp/config.yaml",   // User config
    ///     "~/.myapp.yaml",                 // Fallback location
    /// ];
    ///
    /// let result = edit_first_writeable(&candidates, |config| {
    ///     config.at("setting").set("value").ok();
    ///     true
    /// }).unwrap();
    ///
    /// match result {
    ///     Some(path) => println!("Updated config at: {}", path.display()),
    ///     None => println!("No writeable config file found"),
    /// }
    /// ```
    #[cfg(feature = "std")]
    pub fn edit_first_writeable<P, F>(candidates: &[P], edit_fn: F) -> io::Result<Option<PathBuf>>
    where
        P: AsRef<Path>,
        F: FnOnce(&mut Config) -> bool,
    {
        // Find the first writeable path
        let writeable_path = Self::find_first_writeable(candidates);

        match writeable_path {
            Some(path) => {
                let saved = Self::edit_file(&path, edit_fn)?;
                if saved {
                    Ok(Some(path))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Find the first writeable path from a list of candidates.
    ///
    /// A path is considered writeable if:
    /// - The file exists and has read+write permissions, OR
    /// - The file doesn't exist but the nearest existing parent directory is writeable
    ///
    /// # Arguments
    ///
    /// * `candidates` - A list of file paths to check
    ///
    /// # Returns
    ///
    /// The first writeable path found, or `None` if no writeable path exists.
    ///
    /// # Availability
    ///
    /// This method is only available when the `std` feature is enabled.
    #[cfg(feature = "std")]
    fn find_first_writeable<P: AsRef<Path>>(candidates: &[P]) -> Option<PathBuf> {
        // First pass: look for an existing file with read+write permissions
        for candidate in candidates {
            let path = candidate.as_ref();
            if let Ok(metadata) = path.metadata() {
                if path.is_file() && Self::file_permissions_are_writeable(&metadata) {
                    return Some(path.to_path_buf());
                }
            }
        }

        // Second pass: look for a path where we can create a new file
        // (parent directory exists and is writeable)
        for candidate in candidates {
            let path = candidate.as_ref();

            // Skip if file already exists (we already checked it in first pass)
            if path.exists() {
                continue;
            }

            // Find the nearest existing parent directory
            let mut parent = path.to_path_buf();
            parent.pop();

            while !parent.exists() && parent.parent().is_some() {
                parent.pop();
            }

            // Check if the parent directory is writeable
            if parent.is_dir() {
                if let Ok(metadata) = parent.metadata() {
                    if Self::directory_permissions_allow_creation(&metadata) {
                        return Some(path.to_path_buf());
                    }
                }
            }
        }

        None
    }

    #[cfg(feature = "std")]
    fn create_parent_directories(path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "std")]
    fn lock_file(path: &Path) -> io::Result<File> {
        let file_name = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "file path has no file name")
        })?;
        let mut lock_name = OsString::from(".");
        lock_name.push(file_name);
        lock_name.push(".lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_file_name(lock_name))?;
        FileExt::lock(&lock)?;
        Ok(lock)
    }

    #[cfg(feature = "std")]
    fn atomic_replace<F>(path: &Path, write: F) -> io::Result<()>
    where
        F: FnOnce(&mut AtomicWriteFile) -> io::Result<()>,
    {
        let mut file = AtomicWriteFile::open(path)?;
        write(&mut file)?;
        file.commit()
    }

    #[cfg(all(feature = "std", unix))]
    fn file_permissions_are_writeable(metadata: &fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o600 == 0o600
    }

    #[cfg(all(feature = "std", unix))]
    fn directory_permissions_allow_creation(metadata: &fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o300 == 0o300
    }

    #[cfg(all(feature = "std", not(unix)))]
    fn file_permissions_are_writeable(metadata: &fs::Metadata) -> bool {
        !metadata.permissions().readonly()
    }

    #[cfg(all(feature = "std", not(unix)))]
    fn directory_permissions_allow_creation(metadata: &fs::Metadata) -> bool {
        !metadata.permissions().readonly()
    }

    /// Registers a transformation function for an exact path.
    ///
    /// The transformation will be applied when [`apply_transformations`](Self::apply_transformations)
    /// is called.
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_yaml(
    ///     "name: '  feuilletage  '\n",
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    /// config.register_transform("name", feuilletage::transform::trim);
    /// config.apply_transformations().unwrap();
    ///
    /// assert_eq!(config.get("name").and_then(|value| value.as_str()), Some("feuilletage"));
    /// # }
    /// ```
    pub fn register_transform(
        &mut self,
        path: &str,
        transform: crate::transform::TransformFn<S, L>,
    ) {
        self.transforms.register_exact(path, transform);
    }

    /// Registers a transformation function for a pattern.
    ///
    /// Supported patterns:
    /// - `*.field` - matches any path ending with `.field`
    /// - `**` - matches any path
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("server.host").set("  api.example.com  ").unwrap();
    /// config.at("database.host").set("  db.example.com  ").unwrap();
    /// config.register_transform_pattern("*.host", feuilletage::transform::trim);
    /// config.apply_transformations().unwrap();
    ///
    /// assert_eq!(
    ///     config.get("server.host").and_then(|value| value.as_str()),
    ///     Some("api.example.com"),
    /// );
    /// assert_eq!(
    ///     config.get("database.host").and_then(|value| value.as_str()),
    ///     Some("db.example.com"),
    /// );
    /// ```
    pub fn register_transform_pattern(
        &mut self,
        pattern: &str,
        transform: crate::transform::TransformFn<S, L>,
    ) {
        self.transforms.register_pattern(pattern, transform);
    }

    /// Applies all registered transformations to the configuration tree.
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("name").set("  demo  ").unwrap();
    /// config.register_transform("name", feuilletage::transform::trim);
    /// config.apply_transformations().unwrap();
    ///
    /// assert_eq!(config.get("name").and_then(|value| value.as_str()), Some("demo"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if any transformation fails.
    pub fn apply_transformations(&mut self) -> Result<(), Error> {
        self.transforms.apply_to_tree(&mut self.root, "")
    }
}

#[cfg(all(test, feature = "std"))]
mod file_persistence_tests {
    use super::*;

    #[test]
    fn atomic_replace_write_failure_preserves_original() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, b"original").unwrap();

        let error = Config::<Source, Level>::atomic_replace(&path, |file| {
            file.write_all(b"partial replacement")?;
            Err(io::Error::other("injected write failure"))
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fs::read(&path).unwrap(), b"original");
    }
}

impl Default for Config<Source, Level> {
    fn default() -> Self {
        Self {
            root: ContextValue::object(
                Default::default(),
                Context::new(Source::Default, Level::User),
            ),
            tracker: ErrorTracker::new(),
            transforms: TransformRegistry::new(),
            last_loaded_format: Format::Unknown,
            default_format: Format::default_format(),
            #[cfg(feature = "std")]
            loaded_files: Vec::new(),
        }
    }
}

// =============================================================================
// File loading methods specific to Config<Source, L>.
// These are only available when using the built-in Source type.
// =============================================================================

#[cfg(feature = "std")]
impl<L: LevelType> Config<Source, L> {
    /// Loads configuration from a file and merges it into the current configuration.
    ///
    /// The file format is detected from the file extension:
    /// - `.json` for JSON
    /// - `.yaml` or `.yml` for YAML
    /// - `.toml` for TOML
    ///
    /// # Returns
    ///
    /// - `true` - File was loaded successfully and merged
    /// - `false` - File was skipped (doesn't exist, couldn't be read, format error, or parse error)
    ///
    /// When a file fails to load due to format or parse errors, the error is recorded
    /// in the error tracker (accessible via [`errors()`](Self::errors)) rather than
    /// being returned. This allows loading to continue with other files.
    ///
    /// # Note
    ///
    /// This method is only available when using the built-in [`Source`] type.
    /// Recognized extensions determine the format. Paths without a recognized
    /// extension record a format error. Use [`load_file_auto`](Self::load_file_auto)
    /// to explicitly opt into best-effort content detection.
    ///
    /// For custom source types, load configuration using [`load_json`](Self::load_json),
    /// [`load_yaml`](Self::load_yaml), or [`load_toml`](Self::load_toml) methods
    /// with a custom context.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use feuilletage::{Config, Level};
    ///
    /// let mut config = Config::default();
    ///
    /// // Load files - missing files are silently skipped
    /// config.load_file("/etc/app/config.yaml", Level::System);
    /// config.load_file("~/.config/app/config.yaml", Level::User);
    ///
    /// // Check which files were actually loaded
    /// for path in config.loaded_files() {
    ///     println!("Loaded: {}", path.display());
    /// }
    ///
    /// // Check for any parse/format errors
    /// if config.has_errors() {
    ///     for error in config.get_errors() {
    ///         eprintln!("Error: {}", error);
    ///     }
    /// }
    /// ```
    pub fn load_file(&mut self, path: impl AsRef<Path>, level: L) -> bool {
        let path = path.as_ref();
        match loader::load_file(path, level) {
            Ok(Some(new_config)) => {
                let format = new_config.context().format.clone();
                self.merge(new_config);
                self.set_loaded_format(format);
                self.loaded_files.push(path.to_path_buf());
                true
            }
            Ok(None) => {
                // File doesn't exist or couldn't be read - silent skip
                false
            }
            Err(e) => {
                // Format or parse error - record in tracker
                self.tracker.record(e);
                false
            }
        }
    }

    /// Loads configuration from a file using best-effort content detection.
    ///
    /// Enabled parsers are tried from strictest to most permissive: JSON,
    /// TOML, then YAML. Failed speculative parses are discarded if a later
    /// parser succeeds. Prefer [`load_file`](Self::load_file) for recognized
    /// extensions or [`load_file_with_format`](Self::load_file_with_format)
    /// when the format is known.
    ///
    /// ```no_run
    /// use feuilletage::{Config, Level};
    ///
    /// let mut config = Config::default();
    /// if config.load_file_auto("config.data", Level::User) {
    ///     println!("Detected {:?}", config.loaded_format());
    /// }
    /// ```
    pub fn load_file_auto(&mut self, path: impl AsRef<Path>, level: L) -> bool {
        let path = path.as_ref();
        match loader::load_file_auto(path, level) {
            Ok(Some(new_config)) => {
                let format = new_config.context().format.clone();
                self.merge(new_config);
                self.set_loaded_format(format);
                self.loaded_files.push(path.to_path_buf());
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.tracker.record(error);
                false
            }
        }
    }

    /// Loads configuration from a file using an explicit format.
    ///
    /// ```no_run
    /// use feuilletage::{Config, Format, Level};
    ///
    /// let mut config = Config::default();
    /// let loaded = config.load_file_with_format("config.data", Format::Json, Level::User);
    /// assert!(loaded || config.has_errors());
    /// ```
    pub fn load_file_with_format(
        &mut self,
        path: impl AsRef<Path>,
        format: Format,
        level: L,
    ) -> bool {
        let path = path.as_ref();
        match loader::load_file_with_format(path, format, level) {
            Ok(Some(new_config)) => {
                let format = new_config.context().format.clone();
                self.merge(new_config);
                self.set_loaded_format(format);
                self.loaded_files.push(path.to_path_buf());
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.tracker.record(error);
                false
            }
        }
    }

    /// Returns the list of files that were successfully loaded.
    ///
    /// This allows you to check which configuration files were actually found
    /// and loaded, which is useful for debugging or logging.
    ///
    /// ```no_run
    /// use feuilletage::{Config, Level};
    ///
    /// let mut config = Config::default();
    /// config.load_file("config.yaml", Level::User);
    /// for path in config.loaded_files() {
    ///     println!("Loaded {}", path.display());
    /// }
    /// ```
    pub fn loaded_files(&self) -> &[PathBuf] {
        &self.loaded_files
    }
}

// =============================================================================
// Convenience functions for editing files with default Config type.
// These allow `edit_file(...)` without turbofish syntax.
// For custom types, use `Config::<MySource, MyLevel>::edit_file(...)`.
// =============================================================================

/// Edit a configuration file in place using default Config type.
///
/// This is a convenience function that calls [`Config::edit_file`] without
/// requiring turbofish syntax. For custom source/level types, use the method
/// directly: `Config::<MySource, MyLevel>::edit_file(...)`.
///
/// See [`Config::edit_file`] for full documentation.
///
/// ```no_run
/// use feuilletage::edit_file;
///
/// edit_file("config.yaml", |config| config.at("enabled").set(true).is_ok()).unwrap();
/// ```
#[cfg(feature = "std")]
pub fn edit_file<P, F>(path: P, edit_fn: F) -> io::Result<bool>
where
    P: AsRef<Path>,
    F: FnOnce(&mut Config) -> bool,
{
    Config::<Source, Level>::edit_file(path, edit_fn)
}

/// Edit a configuration file using an explicit format and the default Config type.
///
/// ```no_run
/// use feuilletage::{edit_file_with_format, Format};
///
/// edit_file_with_format("config.data", Format::Json, |config| {
///     config.at("enabled").set(true).is_ok()
/// })
/// .unwrap();
/// ```
#[cfg(feature = "std")]
pub fn edit_file_with_format<P, F>(path: P, format: Format, edit_fn: F) -> io::Result<bool>
where
    P: AsRef<Path>,
    F: FnOnce(&mut Config) -> bool,
{
    Config::<Source, Level>::edit_file_with_format(path, format, edit_fn)
}

/// Edit the first existing file from a list using default Config type.
///
/// This is a convenience function that calls [`Config::edit_first_existing`] without
/// requiring turbofish syntax. For custom source/level types, use the method
/// directly: `Config::<MySource, MyLevel>::edit_first_existing(...)`.
///
/// See [`Config::edit_first_existing`] for full documentation.
///
/// ```no_run
/// use feuilletage::edit_first_existing;
///
/// let paths = ["config.yaml", ".config.yaml"];
/// let edited = edit_first_existing(&paths, |config| {
///     config.at("enabled").set(true).is_ok()
/// })
/// .unwrap();
/// println!("Edited: {edited:?}");
/// ```
#[cfg(feature = "std")]
pub fn edit_first_existing<P, F>(paths: &[P], edit_fn: F) -> io::Result<Option<PathBuf>>
where
    P: AsRef<Path>,
    F: FnOnce(&mut Config) -> bool,
{
    Config::<Source, Level>::edit_first_existing(paths, edit_fn)
}

/// Edit the first writeable file from a list using default Config type.
///
/// This is a convenience function that calls [`Config::edit_first_writeable`] without
/// requiring turbofish syntax. For custom source/level types, use the method
/// directly: `Config::<MySource, MyLevel>::edit_first_writeable(...)`.
///
/// See [`Config::edit_first_writeable`] for full documentation.
///
/// ```no_run
/// use feuilletage::edit_first_writeable;
///
/// let paths = ["/etc/myapp/config.yaml", "config.yaml"];
/// let edited = edit_first_writeable(&paths, |config| {
///     config.at("enabled").set(true).is_ok()
/// })
/// .unwrap();
/// println!("Edited: {edited:?}");
/// ```
#[cfg(feature = "std")]
pub fn edit_first_writeable<P, F>(candidates: &[P], edit_fn: F) -> io::Result<Option<PathBuf>>
where
    P: AsRef<Path>,
    F: FnOnce(&mut Config) -> bool,
{
    Config::<Source, Level>::edit_first_writeable(candidates, edit_fn)
}

impl<S: SourceType, L: LevelType> Config<S, L> {
    /// Select entries from the root object where the predicate returns true.
    ///
    /// Creates a new Config containing only the entries for which `predicate(key, value)`
    /// returns true. This is useful for filtering configuration based on custom logic.
    ///
    /// Returns `None` if the root is not an object.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"a": 1, "b": 2, "c": 3}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// // Select only keys "a" and "c"
    /// let partial = config.select(|key, _| key == "a" || key == "c").unwrap();
    /// assert!(partial.get("a").is_some());
    /// assert!(partial.get("b").is_none());
    /// assert!(partial.get("c").is_some());
    /// # }
    /// ```
    pub fn select<F>(&self, predicate: F) -> Option<Config<S, L>>
    where
        F: Fn(&str, &ContextValue<S, L>) -> bool,
    {
        let root_map = match &self.root {
            ContextValue::Object(map, _) => map,
            _ => return None,
        };

        let mut filtered_map = crate::__private::IndexMap::default();
        for (key, value) in root_map {
            if predicate(key, value) {
                filtered_map.insert(key.clone(), value.clone());
            }
        }

        let filtered_root = ContextValue::object(filtered_map, self.root.context().clone());

        Some(Config {
            root: filtered_root,
            tracker: ErrorTracker::new(),
            transforms: TransformRegistry::new(),
            last_loaded_format: self.last_loaded_format.clone(),
            default_format: self.default_format.clone(),
            #[cfg(feature = "std")]
            loaded_files: Vec::new(),
        })
    }

    /// Split the root object into groups based on a grouping function.
    ///
    /// The function returns a group key for each entry. Entries with the same
    /// group key are collected into the same Config.
    ///
    /// Returns a Vec of `(group_key, Config)` pairs, preserving the order of
    /// first occurrence of each group key.
    ///
    /// Returns `None` if the root is not an object.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"org_a": 1, "org_b": 2, "path_x": 3, "other": 4}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// // Split by prefix
    /// let groups = config.split(|key, _| {
    ///     if key.starts_with("org_") { "orgs" }
    ///     else if key.starts_with("path_") { "paths" }
    ///     else { "misc" }
    /// }).unwrap();
    ///
    /// // Groups: [("orgs", Config with org_a, org_b), ("paths", Config with path_x), ("misc", Config with other)]
    /// assert_eq!(groups.len(), 3);
    /// assert_eq!(groups[0].0, "orgs");
    /// # }
    /// ```
    pub fn split<K, F>(&self, group_fn: F) -> Option<Vec<(K, Config<S, L>)>>
    where
        K: Eq + core::hash::Hash + Clone,
        F: Fn(&str, &ContextValue<S, L>) -> K,
    {
        let root_map = match &self.root {
            ContextValue::Object(map, _) => map,
            _ => return None,
        };

        // Use IndexMap to preserve insertion order of first occurrence
        let mut groups: crate::__private::IndexMap<
            K,
            crate::__private::IndexMap<String, ContextValue<S, L>>,
        > = crate::__private::IndexMap::default();

        for (key, value) in root_map {
            let group_key = group_fn(key, value);
            groups
                .entry(group_key)
                .or_default()
                .insert(key.clone(), value.clone());
        }

        let result = groups
            .into_iter()
            .map(|(group_key, entries)| {
                let group_root = ContextValue::object(entries, self.root.context().clone());
                let group_config = Config {
                    root: group_root,
                    tracker: ErrorTracker::new(),
                    transforms: TransformRegistry::new(),
                    last_loaded_format: self.last_loaded_format.clone(),
                    default_format: self.default_format.clone(),
                    #[cfg(feature = "std")]
                    loaded_files: Vec::new(),
                };
                (group_key, group_config)
            })
            .collect();

        Some(result)
    }

    /// Split the root object by key, returning each key as a separate Config.
    ///
    /// This is a convenience method equivalent to `split(|key, _| key.to_string())`.
    ///
    /// Returns `None` if the root is not an object.
    /// Returns a Vec of `(key, Config)` pairs preserving the original key order.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{Config, Context, Level, Source};
    ///
    /// let mut config = Config::default();
    /// config.load_json(
    ///     r#"{"org": ["example"], "path": {"append": []}}"#,
    ///     Context::new(Source::Programmatic, Level::User),
    /// );
    ///
    /// let parts = config.split_by_key().unwrap();
    /// assert_eq!(parts.len(), 2);
    /// assert_eq!(parts[0].0, "org");
    /// assert_eq!(parts[1].0, "path");
    /// # }
    /// ```
    pub fn split_by_key(&self) -> Option<Vec<(String, Config<S, L>)>> {
        self.split(|key, _| key.to_string())
    }
}

// Unit tests have been moved to feuilletage/tests/unit/config_test.rs

//! Configuration file loading utilities.
//!
//! This module provides utilities for loading configuration from files and strings
//! in various formats (JSON, YAML, TOML).
//!
//! # Overview
//!
//! There are two main approaches to loading configuration:
//!
//! 1. **Direct loading**: Use [`crate::loader::load_file`] to load a single file into a [`crate::ContextValue`]
//! 2. **Builder pattern**: Use [`ConfigLoaderBuilder`] to load multiple files with merge
//!
//! # ConfigLoaderBuilder
//!
//! The [`ConfigLoaderBuilder`] provides a fluent interface for loading multiple
//! configuration sources and merging them with proper priority handling:
//!
//! ```no_run
//! # #[cfg(feature = "std")] {
//! use feuilletage::{loader, Level, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct AppConfig {
//!     #[feuilletage(default = "default")]
//!     name: String,
//! }
//!
//! let config: AppConfig = loader()
//!     .load_file("/etc/myapp/defaults.yaml", Level::System)
//!     .load_file("~/.config/myapp/config.yaml", Level::User)
//!     .load_file("./.myapp.yaml", Level::Local)
//!     .deserialize().unwrap();
//! # }
//! ```
//!
//! # Mutability Enforcement
//!
//! When deserializing, the builder can enforce `#[feuilletage(mutable_by = [...])]`
//! constraints. Values from config levels not in a field's allowed list are
//! skipped during merge, and warnings are recorded.
//!
//! # Format Support
//!
//! - JSON (requires `json` feature)
//! - YAML (requires `yaml` feature)
//! - TOML (requires `toml` feature)
//!
//! Format is auto-detected from file extension, or can be specified explicitly.

#[cfg(feature = "std")]
use std::path::Path;

#[cfg(all(
    not(feature = "std"),
    any(feature = "json", feature = "yaml", feature = "toml")
))]
use alloc::format;
#[cfg(all(not(feature = "std"), feature = "toml"))]
use alloc::string::ToString;
#[cfg(all(
    not(feature = "std"),
    any(feature = "json", feature = "yaml", feature = "toml")
))]
use alloc::vec::Vec;

#[cfg(any(feature = "json", feature = "yaml", feature = "toml"))]
use crate::__private::IndexMap;

#[cfg(any(feature = "std", feature = "json", feature = "yaml", feature = "toml"))]
use crate::{
    context::{Context, LevelType, SourceType},
    error::Error,
    value::ContextValue,
};

#[cfg(feature = "std")]
use crate::{
    config::Config,
    context::{Format, Level, Source},
    de::{FromContextValue, MutabilityInfo},
    error::ErrorTracker,
    merge::{merge_values, merge_with_mutability_constraints},
    value::MergeModifier,
};

/// Load configuration from a file, auto-detecting format from extension.
///
/// Returns `Ok(None)` if the file doesn't exist (silent skip).
/// Returns `Err(IoError)` if the file exists but cannot be read (permission denied, etc.).
/// Returns `Err(FormatNotSupported)` if the file format is not supported.
///
/// The source type `S` must implement [`SourceType`] (via [`CustomSource`](crate::CustomSource)),
/// and its `from_file()` factory method will be used to create the source for the context.
///
/// # Availability
///
/// This function is only available when the `std` feature is enabled.
///
/// ```
/// # #[cfg(all(feature = "std", feature = "json"))] {
/// use feuilletage::{loader, ContextValue, Level, Source};
///
/// let path = std::env::temp_dir().join("feuilletage-load-file.json");
/// std::fs::write(&path, r#"{"port": 8080}"#).unwrap();
///
/// let value = loader::load_file::<Source, Level>(&path, Level::User)
///     .unwrap()
///     .unwrap();
/// let values = value.as_object().unwrap();
/// assert!(matches!(values.get("port"), Some(ContextValue::Int(8080, _))));
///
/// std::fs::remove_file(path).unwrap();
/// # }
/// ```
#[cfg(feature = "std")]
pub fn load_file<S: SourceType, L: LevelType>(
    path: impl AsRef<Path>,
    level: L,
) -> Result<Option<ContextValue<S, L>>, Error> {
    let path = path.as_ref();
    let context = Context::new(S::from_file(path.to_path_buf()), level);
    load_file_with_context(path, context, None)
}

/// Load configuration from a file using an explicit format.
///
/// This bypasses extension detection while preserving file source metadata.
/// Returns `Ok(None)` when the file does not exist.
///
/// ```no_run
/// # #[cfg(all(feature = "std", feature = "yaml"))] {
/// use feuilletage::{loader, Format, Level, Source};
///
/// let config = loader::load_file_with_format::<Source, Level>(
///     "config.data",
///     Format::Yaml,
///     Level::User,
/// )
/// .unwrap();
/// # }
/// ```
#[cfg(feature = "std")]
pub fn load_file_with_format<S: SourceType, L: LevelType>(
    path: impl AsRef<Path>,
    format: Format,
    level: L,
) -> Result<Option<ContextValue<S, L>>, Error> {
    let path = path.as_ref();
    let context = Context::new(S::from_file(path.to_path_buf()), level);
    load_file_with_context(path, context, Some(format))
}

/// Load configuration from a file by trying each enabled format.
///
/// Parsers are attempted from strictest to most permissive: JSON, TOML, then
/// YAML. This is best-effort content detection, not format identification:
/// JSON is valid YAML, and YAML can accept some TOML-looking input as a scalar.
/// Prefer [`load_file`] when the file has a recognized extension, or
/// [`load_file_with_format`] when the format is known.
///
/// Failed parser attempts are discarded when a later parser succeeds. If all
/// enabled parsers fail, one aggregate parse error describes every attempt.
/// Returns `Ok(None)` when the file does not exist.
///
/// ```no_run
/// # #[cfg(feature = "std")] {
/// use feuilletage::{loader, Level, Source};
///
/// let value = loader::load_file_auto::<Source, Level>("config", Level::User).unwrap();
/// # }
/// ```
#[cfg(feature = "std")]
pub fn load_file_auto<S: SourceType, L: LevelType>(
    path: impl AsRef<Path>,
    level: L,
) -> Result<Option<ContextValue<S, L>>, Error> {
    let path = path.as_ref();
    let context = Context::new(S::from_file(path.to_path_buf()), level);
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::IoError {
                path: path.display().to_string(),
                message: error.to_string(),
            });
        }
    };

    load_str_auto_with_context(&content, context).map(Some)
}

#[cfg(feature = "std")]
#[cfg_attr(
    not(any(feature = "json", feature = "toml", feature = "yaml")),
    allow(unused_variables, unused_mut)
)]
fn load_str_auto_with_context<S: SourceType, L: LevelType>(
    content: &str,
    context: Context<S, L>,
) -> Result<ContextValue<S, L>, Error> {
    let source = context.source.display_name();
    let mut failures: Vec<String> = Vec::new();

    #[cfg(feature = "json")]
    match load_json(content, context.clone().with_format(Format::Json)) {
        Ok(value) => return Ok(value),
        Err(error) => failures.push(format!("JSON: {}", parse_failure_message(error))),
    }

    #[cfg(feature = "toml")]
    match load_toml(content, context.clone().with_format(Format::Toml)) {
        Ok(value) => return Ok(value),
        Err(error) => failures.push(format!("TOML: {}", parse_failure_message(error))),
    }

    #[cfg(feature = "yaml")]
    match load_yaml(content, context.with_format(Format::Yaml)) {
        Ok(value) => return Ok(value),
        Err(error) => failures.push(format!("YAML: {}", parse_failure_message(error))),
    }

    if failures.is_empty() {
        Err(Error::FormatNotSupported {
            format: "auto".to_string(),
            message: "automatic format detection requires at least one enabled format feature"
                .to_string(),
        })
    } else {
        Err(Error::ParseError {
            source,
            message: format!("automatic format detection failed: {}", failures.join("; ")),
        })
    }
}

#[cfg(all(
    feature = "std",
    any(feature = "json", feature = "toml", feature = "yaml")
))]
fn parse_failure_message(error: Error) -> String {
    match error {
        Error::ParseError { message, .. } => message,
        error => error.to_string(),
    }
}

#[cfg(feature = "std")]
fn load_file_with_context<S: SourceType, L: LevelType>(
    path: &Path,
    mut context: Context<S, L>,
    format: Option<Format>,
) -> Result<Option<ContextValue<S, L>>, Error> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::IoError {
                path: path.display().to_string(),
                message: error.to_string(),
            });
        }
    };

    let format = match format {
        Some(format) => format,
        None => format_from_extension(path)?,
    };
    context.format = format.clone();

    load_str_with_context(&content, format, context).map(Some)
}

#[cfg(feature = "std")]
fn format_from_extension(path: &Path) -> Result<Format, Error> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("");

    match extension {
        "json" => Ok(Format::Json),
        "yaml" | "yml" => Ok(Format::Yaml),
        "toml" => Ok(Format::Toml),
        _ => Err(Error::FormatNotSupported {
            format: extension.to_string(),
            message: format!("unsupported file extension: {}", extension),
        }),
    }
}

#[cfg_attr(
    all(not(feature = "json"), not(feature = "yaml"), not(feature = "toml")),
    allow(unused_variables)
)]
#[cfg(feature = "std")]
fn load_str_with_context<S: SourceType, L: LevelType>(
    content: &str,
    format: Format,
    context: Context<S, L>,
) -> Result<ContextValue<S, L>, Error> {
    match format {
        #[cfg(feature = "json")]
        Format::Json => load_json(content, context),
        #[cfg(not(feature = "json"))]
        Format::Json => Err(Error::FormatNotSupported {
            format: "json".to_string(),
            message: "JSON feature not enabled".to_string(),
        }),
        #[cfg(feature = "yaml")]
        Format::Yaml => load_yaml(content, context),
        #[cfg(not(feature = "yaml"))]
        Format::Yaml => Err(Error::FormatNotSupported {
            format: "yaml".to_string(),
            message: "YAML feature not enabled".to_string(),
        }),
        #[cfg(feature = "toml")]
        Format::Toml => load_toml(content, context),
        #[cfg(not(feature = "toml"))]
        Format::Toml => Err(Error::FormatNotSupported {
            format: "toml".to_string(),
            message: "TOML feature not enabled".to_string(),
        }),
        Format::Unknown => Err(Error::FormatNotSupported {
            format: "unknown".to_string(),
            message: "configuration input requires an explicit format".to_string(),
        }),
    }
}

/// Parse a JSON string into a [`ContextValue`] tagged with the given context.
///
/// ```
/// # #[cfg(feature = "json")] {
/// use feuilletage::context::{Context, Level, Source};
/// use feuilletage::loader::load_json;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let value = load_json(r#"{"name": "feuilletage", "port": 8080}"#, ctx).unwrap();
/// assert!(value.is_object());
/// # }
/// ```
#[cfg(feature = "json")]
pub fn load_json<S: SourceType, L: LevelType>(
    content: &str,
    context: Context<S, L>,
) -> Result<ContextValue<S, L>, Error> {
    let value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| Error::ParseError {
            source: context.source.display_name(),
            message: format!("JSON parse error: {}", e),
        })?;

    Ok(convert_json_value(value, context))
}

/// Parse a YAML string into a [`ContextValue`] tagged with the given context.
///
/// ```
/// # #[cfg(feature = "yaml")] {
/// use feuilletage::context::{Context, Level, Source};
/// use feuilletage::loader::load_yaml;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let value = load_yaml("name: feuilletage\nport: 8080\n", ctx).unwrap();
/// assert!(value.is_object());
/// # }
/// ```
#[cfg(feature = "yaml")]
pub fn load_yaml<S: SourceType, L: LevelType>(
    content: &str,
    context: Context<S, L>,
) -> Result<ContextValue<S, L>, Error> {
    // Use serde_json::Value as intermediate format since serde-saphyr can deserialize to it
    let value: serde_json::Value =
        serde_saphyr::from_str(content).map_err(|e| Error::ParseError {
            source: context.source.display_name(),
            message: format!("YAML parse error: {}", e),
        })?;

    Ok(convert_json_value(value, context))
}

/// Parse a TOML string into a [`ContextValue`] tagged with the given context.
///
/// ```
/// # #[cfg(feature = "toml")] {
/// use feuilletage::context::{Context, Level, Source};
/// use feuilletage::loader::load_toml;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let value = load_toml("name = \"feuilletage\"\nport = 8080\n", ctx).unwrap();
/// assert!(value.is_object());
/// # }
/// ```
#[cfg(feature = "toml")]
pub fn load_toml<S: SourceType, L: LevelType>(
    content: &str,
    context: Context<S, L>,
) -> Result<ContextValue<S, L>, Error> {
    let value: toml::Value = toml::from_str(content).map_err(|e| Error::ParseError {
        source: context.source.display_name(),
        message: format!("TOML parse error: {}", e),
    })?;

    Ok(convert_toml_value(value, context))
}

/// Convert serde_json::Value to ContextValue.
/// Used by both `load_json` (json feature) and `load_yaml` (yaml feature)
/// as the intermediate format; not needed when only `toml` is enabled.
#[cfg(any(feature = "json", feature = "yaml"))]
fn convert_json_value<S: SourceType, L: LevelType>(
    value: serde_json::Value,
    context: Context<S, L>,
) -> ContextValue<S, L> {
    match value {
        serde_json::Value::Null => ContextValue::null(context),
        serde_json::Value::Bool(b) => ContextValue::bool(b, context),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ContextValue::int(i, context)
            } else if let Some(f) = n.as_f64() {
                ContextValue::float(f, context)
            } else {
                ContextValue::null(context)
            }
        }
        serde_json::Value::String(s) => ContextValue::string(s, context),
        serde_json::Value::Array(arr) => {
            let items: Vec<ContextValue<S, L>> = arr
                .into_iter()
                .map(|v| convert_json_value(v, context.clone()))
                .collect();
            ContextValue::array(items, context)
        }
        serde_json::Value::Object(obj) => {
            let mut map = IndexMap::default();
            for (k, v) in obj {
                map.insert(k, convert_json_value(v, context.clone()));
            }
            ContextValue::object(map, context)
        }
    }
}

/// Convert toml::Value to ContextValue
#[cfg(feature = "toml")]
fn convert_toml_value<S: SourceType, L: LevelType>(
    value: toml::Value,
    context: Context<S, L>,
) -> ContextValue<S, L> {
    match value {
        toml::Value::String(s) => ContextValue::string(s, context),
        toml::Value::Integer(i) => ContextValue::int(i, context),
        toml::Value::Float(f) => ContextValue::float(f, context),
        toml::Value::Boolean(b) => ContextValue::bool(b, context),
        toml::Value::Array(arr) => {
            let items: Vec<ContextValue<S, L>> = arr
                .into_iter()
                .map(|v| convert_toml_value(v, context.clone()))
                .collect();
            ContextValue::array(items, context)
        }
        toml::Value::Table(obj) => {
            let mut map = IndexMap::default();
            for (k, v) in obj {
                map.insert(k, convert_toml_value(v, context.clone()));
            }
            ContextValue::object(map, context)
        }
        toml::Value::Datetime(dt) => ContextValue::string(dt.to_string(), context),
    }
}

// ============================================================================
// ConfigLoaderBuilder - Multi-file loading with deferred merge
// ============================================================================

/// Builder for loading multiple config sources.
///
/// Files are parsed and stored, but NOT merged until `deserialize()` is called.
/// This enables mutability enforcement during merge, since we know the target type
/// and its `mutable_by` constraints at deserialization time.
///
/// # Example
///
/// ```no_run
/// use feuilletage::{loader, Level, FromContextValue};
///
/// #[derive(Debug, feuilletage::Config)]
/// struct AppConfig {
///     #[feuilletage(default = "default")]
///     name: String,
/// }
///
/// let config: AppConfig = loader()
///     .load_file("/etc/myapp/config.yaml", Level::System)
///     .load_file("~/.config/myapp/config.yaml", Level::User)
///     .load_file("./.myapp.yaml", Level::Local)
///     .deserialize().unwrap();
/// ```
/// Builder for loading multiple config sources with the built-in Source type.
///
/// Note: This builder uses `Source` for the source type. For custom source types,
/// use string loaders directly with your own Context.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct ConfigLoaderBuilder<S: SourceType = Source, L: LevelType = Level> {
    /// Parsed config values with their levels, stored in load order until merge
    sources: Vec<(ContextValue<S, L>, L)>,
    /// Error tracker for warnings and non-fatal errors
    tracker: ErrorTracker,
    /// Files that were successfully loaded
    loaded_files: Vec<std::path::PathBuf>,
    /// Preferred output format when no loaded format or extension determines one
    default_format: Format,
}

#[cfg(feature = "std")]
impl Default for ConfigLoaderBuilder<Source, Level> {
    fn default() -> Self {
        ConfigLoaderBuilder::new()
    }
}

#[cfg(feature = "std")]
impl<S: SourceType, L: LevelType> ConfigLoaderBuilder<S, L> {
    /// Create a new empty config loader builder.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            tracker: ErrorTracker::new(),
            loaded_files: Vec::new(),
            default_format: Format::default_format(),
        }
    }

    /// Set the preferred output format.
    ///
    /// This controls serialization when no input format has been loaded and
    /// file writes whose destination has no recognized extension. It does not
    /// affect input parsing.
    ///
    /// Invalid or disabled formats are recorded in [`errors`](Self::errors)
    /// and leave the previous preference unchanged.
    ///
    /// ```no_run
    /// # #[cfg(all(feature = "std", feature = "toml"))] {
    /// use feuilletage::{loader, Format};
    ///
    /// let config = loader()
    ///     .default_format(Format::Toml)
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(config.default_format(), Format::Toml);
    /// # }
    /// ```
    pub fn default_format(mut self, format: Format) -> Self {
        match format.ensure_enabled() {
            Ok(()) => self.default_format = format,
            Err(error) => self.tracker.record(error),
        }
        self
    }

    /// Add a config file with specified level.
    ///
    /// The file is parsed immediately, but not merged yet.
    /// Merge happens during `deserialize()` when we know the target type.
    ///
    /// Files that don't exist are silently skipped. I/O, format, and parse
    /// errors are recorded in the error tracker.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config file
    /// * `level` - The Level to associate with values from this file
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-builder-load.json");
    /// std::fs::write(&path, r#"{"name": "file"}"#).unwrap();
    ///
    /// let config = loader().load_file(&path, Level::User).build().unwrap();
    /// assert_eq!(config.get("name").and_then(|value| value.as_str()), Some("file"));
    ///
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn load_file<P: AsRef<Path>>(self, path: P, level: L) -> Self {
        let path = path.as_ref();
        let context = Context::new(S::from_file(path.to_path_buf()), level.clone());
        self.load_file_inner(path, level, context, None)
    }

    /// Add a config file parsed using an explicitly supplied format.
    ///
    /// Unlike [`load_file`](Self::load_file), this method does not inspect the
    /// file extension. The values retain file source metadata and the supplied
    /// format and level. Missing files are silently skipped; I/O, unsupported
    /// format-feature, and parse errors are recorded in the error tracker.
    /// Successfully parsed files are included in [`loaded_files`](Self::loaded_files).
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config file
    /// * `format` - Format to use when parsing the file
    /// * `level` - The level to associate with values from this file
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Format, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-explicit-format.data");
    /// std::fs::write(&path, r#"{"name": "explicit"}"#).unwrap();
    ///
    /// let config = loader()
    ///     .load_file_with_format(&path, Format::Json, Level::User)
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(config.get("name").and_then(|value| value.as_str()), Some("explicit"));
    ///
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn load_file_with_format<P: AsRef<Path>>(self, path: P, format: Format, level: L) -> Self {
        let path = path.as_ref();
        let context = Context::new(S::from_file(path.to_path_buf()), level.clone());
        self.load_file_inner(path, level, context, Some(format))
    }

    /// Add a config file using best-effort content detection.
    ///
    /// Enabled parsers are tried in JSON, TOML, YAML order. Failed attempts do
    /// not enter the error tracker if a later parser succeeds. Prefer
    /// [`load_file`](Self::load_file) or
    /// [`load_file_with_format`](Self::load_file_with_format) whenever the
    /// format is known.
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Format, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-auto-format");
    /// std::fs::write(&path, r#"{"name": "detected"}"#).unwrap();
    ///
    /// let config = loader().load_file_auto(&path, Level::User).build().unwrap();
    /// assert_eq!(config.loaded_format(), Format::Json);
    ///
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn load_file_auto<P: AsRef<Path>>(mut self, path: P, level: L) -> Self {
        let path = path.as_ref();
        match crate::loader::load_file_auto::<S, L>(path, level.clone()) {
            Ok(Some(config_value)) => {
                self.sources.push((config_value, level));
                self.loaded_files.push(path.to_path_buf());
            }
            Ok(None) => {
                // Missing files are optional.
            }
            Err(error) => self.tracker.record(error),
        }
        self
    }

    fn load_file_inner(
        mut self,
        path: &Path,
        level: L,
        context: Context<S, L>,
        format: Option<Format>,
    ) -> Self {
        match load_file_with_context(path, context, format) {
            Ok(Some(config_value)) => {
                self.sources.push((config_value, level));
                self.loaded_files.push(path.to_path_buf());
            }
            Ok(None) => {
                // Missing files are optional.
            }
            Err(e) => {
                // Preserve loading diagnostics for callers to inspect.
                self.tracker.record(e);
            }
        }
        self
    }

    /// Returns the list of files that were successfully loaded.
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-loaded-files.json");
    /// std::fs::write(&path, "{}").unwrap();
    /// let loader = loader().load_file(&path, Level::User);
    ///
    /// assert_eq!(loader.loaded_files(), &[path.clone()]);
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn loaded_files(&self) -> &[std::path::PathBuf] {
        &self.loaded_files
    }

    /// Add multiple config files at once.
    ///
    /// Files are loaded in order, each with its associated level.
    /// Files that don't exist or cannot be read are silently skipped.
    /// Format or parse errors are recorded in the error tracker.
    ///
    /// # Arguments
    ///
    /// * `files` - Iterator of (path, level) pairs
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Level};
    ///
    /// let dir = std::env::temp_dir();
    /// let system = dir.join("feuilletage-files-system.json");
    /// let user = dir.join("feuilletage-files-user.json");
    /// std::fs::write(&system, r#"{"port": 80}"#).unwrap();
    /// std::fs::write(&user, r#"{"port": 8080}"#).unwrap();
    ///
    /// let config = loader()
    ///     .load_files([(&system, Level::System), (&user, Level::User)])
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(config.get("port").and_then(|value| value.as_i64()), Some(8080));
    ///
    /// std::fs::remove_file(system).unwrap();
    /// std::fs::remove_file(user).unwrap();
    /// # }
    /// ```
    pub fn load_files<P, I>(mut self, files: I) -> Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = (P, L)>,
    {
        for (path, level) in files {
            self = self.load_file(path, level);
        }
        self
    }

    /// Add a config file with full source metadata.
    ///
    /// Use this when you need more control over the Source
    /// (e.g., for programmatic sources or custom identifiers).
    ///
    /// Files that don't exist are silently skipped.
    /// I/O errors (permission denied, etc.), format errors, and parse errors
    /// are recorded in the error tracker.
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{loader, Level, Source};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-file-source.json");
    /// std::fs::write(&path, r#"{"name": "environment"}"#).unwrap();
    ///
    /// let config = loader()
    ///     .load_file_with_source(&path, Level::User, Source::Environment)
    ///     .build()
    ///     .unwrap();
    /// assert_eq!(config.get("name").unwrap().context().source, Source::Environment);
    ///
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn load_file_with_source<P: AsRef<Path>>(self, path: P, level: L, source: S) -> Self {
        let path = path.as_ref();
        let context = Context::new(source, level.clone());
        self.load_file_inner(path, level, context, None)
    }

    /// Add config from a string with specified format and level.
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{loader, ContextValue, Format, Level};
    ///
    /// let config = loader()
    ///     .load_str(r#"{"port": 8080}"#, Format::Json, Level::User)
    ///     .unwrap()
    ///     .build()
    ///     .unwrap();
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(8080, _))));
    /// # }
    /// ```
    pub fn load_str(mut self, content: &str, format: Format, level: L) -> Result<Self, Error> {
        let context = Context::new(S::programmatic(), level.clone()).with_format(format.clone());

        let config_value = load_str_with_context(content, format, context)?;

        self.sources.push((config_value, level));
        Ok(self)
    }

    /// Merge all sources and deserialize into the target type.
    ///
    /// This is where:
    /// - Configs are merged by level priority (higher priorities override lower priorities)
    /// - Sources with equal priorities retain load order (later sources override earlier ones)
    /// - `mutable_by` constraints are enforced (values from disallowed levels are skipped)
    /// - Templates are resolved
    /// - Transforms and validation are applied
    ///
    /// # Mutability Enforcement
    ///
    /// When the target type `T` implements [`MutabilityInfo`], this method enforces
    /// the `#[feuilletage(mutable_by = [...])]` constraints during merge. Values from
    /// config levels that are not in a field's allowed list are SKIPPED (not merged),
    /// and a warning is recorded in the error tracker.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{loader, Level, Format, FromContextValue};
    ///
    /// #[derive(Debug, feuilletage::Config)]
    /// struct AppConfig {
    ///     #[feuilletage(default = "DefaultApp")]
    ///     app_name: String,  // Can be set by any level
    ///
    ///     #[feuilletage(mutable_by = ["user", "local"], default = "none")]
    ///     user_preference: String,  // Can only be set by user or local level
    /// }
    ///
    /// let config: AppConfig = loader()
    ///     .load_str(r#"{"app_name": "MyApp", "user_preference": "sys"}"#, Format::Json, Level::System).unwrap()
    ///     .load_str(r#"{"user_preference": "mine"}"#, Format::Json, Level::User).unwrap()
    ///     .deserialize().unwrap();
    ///
    /// // System's user_preference was SKIPPED due to mutable_by constraint
    /// assert_eq!(config.user_preference, "mine");
    /// # }
    /// ```
    ///
    /// # Accessing Warnings
    ///
    /// This method takes `&mut self` so you can read warnings after deserialization:
    ///
    /// ```
    /// # #[cfg(feature = "yaml")] {
    /// use feuilletage::{loader, Level, Format, FromContextValue};
    ///
    /// #[derive(Debug, feuilletage::Config)]
    /// struct AppConfig {
    ///     #[feuilletage(default = "default")]
    ///     name: String,
    /// }
    ///
    /// let mut loader = loader()
    ///     .load_str("name: test", Format::Yaml, Level::System).unwrap();
    ///
    /// let config: AppConfig = loader.deserialize().unwrap();
    ///
    /// // Read warnings for skipped values
    /// for warning in loader.errors().warnings() {
    ///     eprintln!("Warning: {}", warning);
    /// }
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn deserialize<T: FromContextValue<S, L> + MutabilityInfo>(&mut self) -> Result<T, Error>
    where
        L: Default,
    {
        let mut config = self.build_with_mutability::<T>()?;
        let result = config.deserialize();

        // Copy any errors/warnings recorded during deserialization back to our tracker
        for error in config.errors().errors() {
            self.tracker.record(error.clone());
        }
        for warning in config.errors().warnings() {
            self.tracker
                .record_warning_at(&warning.path, &warning.message);
        }

        result
    }

    /// Merge all sources and deserialize without requiring [`MutabilityInfo`].
    ///
    /// Unlike [`deserialize`](Self::deserialize), this method does not read target-type
    /// mutability metadata before merging. This is useful for any [`FromContextValue`]
    /// target that does not implement [`MutabilityInfo`], such as a top-level collection.
    ///
    /// This does not bypass checks performed by [`FromContextValue`] itself. In particular,
    /// `#[derive(Config)]` emits field-level `mutable_by` validation, so a disallowed value
    /// can still fail during deserialization. Runtime mutability constraints set on values
    /// are also enforced during merge.
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{loader, Format, Level};
    ///
    /// let mut values = loader()
    ///     .load_str(r#"["first", "second"]"#, Format::Json, Level::User)
    ///     .unwrap();
    /// let values: Vec<String> = values.deserialize_unconstrained().unwrap();
    /// assert_eq!(values, ["first", "second"]);
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn deserialize_unconstrained<T: FromContextValue<S, L>>(&mut self) -> Result<T, Error>
    where
        L: Default,
    {
        let mut config = self.build_internal()?;
        let result = config.deserialize();

        // Copy any errors/warnings recorded during deserialization back to our tracker
        for error in config.errors().errors() {
            self.tracker.record(error.clone());
        }
        for warning in config.errors().warnings() {
            self.tracker
                .record_warning_at(&warning.path, &warning.message);
        }

        result
    }

    /// Merge all sources with mutability constraints from the target type.
    ///
    /// This builds a Config while enforcing `mutable_by` constraints from `T`.
    fn build_with_mutability<T: MutabilityInfo>(&mut self) -> Result<Config<S, L>, Error>
    where
        L: Default,
    {
        let mut config = Config::new(Context::default());
        config.set_default_format_unchecked(self.default_format.clone());
        let constraints = T::mutability_constraints();

        let mut sources = std::mem::take(&mut self.sources);
        sources.sort_by_key(|(_, level)| level.priority());

        for (source_value, level) in sources {
            let format = source_value.context().format.clone();
            merge_with_mutability_constraints(
                config.root_mut(),
                source_value,
                &level,
                &constraints,
                &mut self.tracker,
            );
            config.set_loaded_format(format);
        }

        // Transfer any errors to the config
        for error in self.tracker.errors() {
            config.errors_mut().record(error.clone());
        }

        Ok(config)
    }

    /// Internal build without constraints (preserves warnings in tracker)
    fn build_internal(&mut self) -> Result<Config<S, L>, Error>
    where
        L: Default,
    {
        let mut config = Config::new(Context::default());
        config.set_default_format_unchecked(self.default_format.clone());

        let mut sources = std::mem::take(&mut self.sources);
        sources.sort_by_key(|(_, level)| level.priority());

        for (source_value, _level) in sources {
            let format = source_value.context().format.clone();
            merge_values(
                config.root_mut(),
                source_value,
                MergeModifier::Default,
                &mut self.tracker,
            );
            config.set_loaded_format(format);
        }

        // Transfer any errors to the config
        for error in self.tracker.errors() {
            config.errors_mut().record(error.clone());
        }

        Ok(config)
    }

    /// Merge all sources into a single Config without type information.
    ///
    /// **Note:** This does NOT enforce `mutable_by` constraints since we don't know
    /// the target struct. Use `deserialize<T>()` for proper mutability enforcement.
    ///
    /// This method consumes the loader. If you need to read warnings after building,
    /// use the internal methods via `deserialize()` instead.
    ///
    /// ```
    /// # #[cfg(feature = "json")] {
    /// use feuilletage::{loader, ContextValue, Format, Level};
    ///
    /// let config = loader()
    ///     .load_str(r#"{"host": "localhost", "port": 8080}"#, Format::Json, Level::System)
    ///     .unwrap()
    ///     .load_str(r#"{"port": 3000}"#, Format::Json, Level::User)
    ///     .unwrap()
    ///     .build()
    ///     .unwrap();
    ///
    /// assert_eq!(config.get("host").and_then(|value| value.as_str()), Some("localhost"));
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(3000, _))));
    /// assert_eq!(config.loaded_format(), Format::Json);
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if merging fails.
    pub fn build(self) -> Result<Config<S, L>, Error>
    where
        L: Default,
    {
        let mut config = Config::new(Context::default());
        config.set_default_format_unchecked(self.default_format);
        let mut tracker = self.tracker;

        let mut sources = self.sources;
        sources.sort_by_key(|(_, level)| level.priority());

        for (source_value, _level) in sources {
            let format = source_value.context().format.clone();
            merge_values(
                config.root_mut(),
                source_value,
                MergeModifier::Default,
                &mut tracker,
            );
            config.set_loaded_format(format);
        }

        // Transfer any errors to the config
        for error in tracker.errors() {
            config.errors_mut().record(error.clone());
        }

        Ok(config)
    }

    /// Get access to the error tracker for warnings and non-fatal errors.
    ///
    /// ```
    /// # #[cfg(feature = "std")] {
    /// use feuilletage::{loader, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-errors.unsupported");
    /// std::fs::write(&path, "value").unwrap();
    /// let loader = loader().load_file(&path, Level::User);
    ///
    /// assert!(loader.errors().has_errors());
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn errors(&self) -> &ErrorTracker {
        &self.tracker
    }

    /// Get mutable access to the error tracker.
    ///
    /// ```
    /// # #[cfg(feature = "std")] {
    /// use feuilletage::{loader, Level};
    ///
    /// let path = std::env::temp_dir().join("feuilletage-clear-errors.unsupported");
    /// std::fs::write(&path, "value").unwrap();
    /// let mut loader = loader().load_file(&path, Level::User);
    /// loader.errors_mut().clear_errors();
    ///
    /// assert!(!loader.errors().has_errors());
    /// std::fs::remove_file(path).unwrap();
    /// # }
    /// ```
    pub fn errors_mut(&mut self) -> &mut ErrorTracker {
        &mut self.tracker
    }
}

/// Convenience function to create a new ConfigLoaderBuilder with default types.
///
/// This is equivalent to `ConfigLoaderBuilder::new()`.
#[cfg(feature = "std")]
pub fn loader() -> ConfigLoaderBuilder<Source, Level> {
    ConfigLoaderBuilder::new()
}

// Unit tests have been moved to feuilletage/tests/unit/loader_test.rs

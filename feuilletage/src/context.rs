//! Configuration context module.
//!
//! This module provides types for tracking metadata about configuration values,
//! including their source, format, priority level, and mutability constraints.
//!
//! # Overview
//!
//! Every [`ContextValue`](crate::ContextValue) in the configuration tree has an associated
//! [`Context`] that tracks:
//!
//! - **Source**: Where the value came from (file, environment, programmatic)
//! - **Format**: The serialization format (JSON, YAML, TOML)
//! - **Level**: The priority level for merging (System, User, Local)
//! - **Mutability**: Whether the value can be overridden and by whom
//!
//! This metadata is preserved through configuration merges and can be inspected
//! for debugging, access control, or audit purposes.
//!
//! # Configuration Levels
//!
//! [`Level`] represents the priority of a configuration source:
//!
//! | Level  | Typical Use |
//! |--------|-------------|
//! | System | System-wide defaults |
//! | User   | User preferences |
//! | Local  | Project-specific settings |
//!
//! Custom levels can be defined by implementing the [`CustomLevel`] trait.
//!
//! # Extensibility
//!
//! Custom source and level types can be created by implementing the [`CustomSource`]
//! and [`CustomLevel`] traits:
//!
//! ```
//! # #[cfg(feature = "std")] {
//! use feuilletage::{CustomSource, CustomLevel};
//! use std::path::{Path, PathBuf};
//!
//! // Define a custom source type
//! #[derive(Clone, Debug, Default, PartialEq)]
//! enum MySource {
//!     #[default]
//!     Default,
//!     File(PathBuf),
//!     Database(String),
//!     RemoteApi(String),
//! }
//!
//! impl CustomSource for MySource {
//!     fn display_name(&self) -> String {
//!         match self {
//!             MySource::Default => "default".to_string(),
//!             MySource::File(path) => path.display().to_string(),
//!             MySource::Database(name) => format!("db:{}", name),
//!             MySource::RemoteApi(url) => format!("api:{}", url),
//!         }
//!     }
//!
//!     fn file_path(&self) -> Option<&Path> {
//!         match self {
//!             MySource::File(path) => Some(path.as_path()),
//!             _ => None,
//!         }
//!     }
//!
//!     fn from_file(path: PathBuf) -> Self {
//!         MySource::File(path)
//!     }
//!
//!     fn programmatic() -> Self {
//!         MySource::Default  // or add a Programmatic variant
//!     }
//!
//!     fn environment() -> Self {
//!         MySource::Default  // or add an Environment variant
//!     }
//! }
//!
//! // MySource can now be used as the source type parameter in Config<MySource, Level>
//! # }
//! ```
//!
//! # Mutability Constraints
//!
//! [`MutabilityConstraint`] controls which configuration levels can modify a value:
//!
//! - `Mutable`: Any level can modify (default)
//! - `MutableByName(names)`: Only levels with matching names can modify
//! - `Immutable`: Cannot be modified after initial set

#[cfg(feature = "std")]
use std::path::{Path, PathBuf};

#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};

use core::fmt::Debug;
use core::hash::Hash;

use serde::{Deserialize, Serialize};

// ============================================================================
// Sealed Trait Infrastructure
// ============================================================================

/// Private module for sealed trait pattern.
///
/// This prevents external types from implementing `SourceType` or `LevelType` directly.
/// Instead, they must implement `CustomSource` or `CustomLevel`, which provides
/// the trait implementation via blanket impls.
mod private {
    /// Marker trait for sealing `SourceType`.
    pub trait SealedSource {}

    /// Marker trait for sealing `LevelType`.
    pub trait SealedLevel {}

    // Any CustomSource automatically implements SealedSource
    // (This includes the built-in Source which implements CustomSource)
    impl<T: super::CustomSource> SealedSource for T {}

    // Any CustomLevel automatically implements SealedLevel
    // (This includes the built-in Level which implements CustomLevel)
    impl<T: super::CustomLevel> SealedLevel for T {}
}

// ============================================================================
// Source Type Traits
// ============================================================================

/// Internal trait for source types used in [`Config`](struct@crate::Config).
///
/// This trait is sealed and cannot be implemented directly. Instead:
/// - Use the built-in [`Source`] enum for standard sources
/// - Implement [`CustomSource`] to create domain-specific source types
///
/// # Why Sealed?
///
/// The sealed pattern ensures that:
/// - The library controls which types can be used as sources
/// - Internal invariants are maintained
/// - Breaking changes are minimized
pub trait SourceType:
    private::SealedSource + Clone + Debug + Default + Send + Sync + 'static
{
    /// Returns a display name for error messages and debugging.
    fn display_name(&self) -> String;

    /// Returns the file path if this source represents a file.
    #[cfg(feature = "std")]
    fn file_path(&self) -> Option<&Path>;

    /// Creates a source representing a file at the given path.
    #[cfg(feature = "std")]
    fn from_file(path: PathBuf) -> Self;

    /// Creates a source representing a file at the given path (no_std version).
    #[cfg(not(feature = "std"))]
    fn from_file(path: String) -> Self;

    /// Creates a source representing programmatically set values.
    fn programmatic() -> Self;

    /// Creates a source representing values from environment variables.
    fn environment() -> Self;
}

/// Trait for custom source types.
///
/// Implement this trait to create domain-specific source types that can be used
/// with [`Config<S, L>`](struct@crate::Config). Implementing `CustomSource` automatically
/// provides the internal [`SourceType`] trait via a blanket implementation.
///
/// Custom sources must include variants for standard source types (file, programmatic,
/// environment) via the factory methods. This allows generic functions like `load_file`
/// to work with any source type.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")] {
/// use feuilletage::CustomSource;
/// use std::path::{Path, PathBuf};
///
/// #[derive(Clone, Debug, PartialEq, Default)]
/// enum MySource {
///     File(PathBuf),
///     Environment,
///     Programmatic,
///     #[default]
///     Default,
///     // Custom variants
///     Package { name: String, version: String },
///     RemoteConfig(String),
/// }
///
/// impl CustomSource for MySource {
///     fn display_name(&self) -> String {
///         match self {
///             MySource::File(path) => path.display().to_string(),
///             MySource::Environment => "environment".to_string(),
///             MySource::Programmatic => "programmatic".to_string(),
///             MySource::Default => "default".to_string(),
///             MySource::Package { name, version } => format!("package:{}@{}", name, version),
///             MySource::RemoteConfig(url) => format!("remote:{}", url),
///         }
///     }
///
///     fn file_path(&self) -> Option<&Path> {
///         match self {
///             MySource::File(path) => Some(path.as_path()),
///             _ => None,
///         }
///     }
///
///     fn from_file(path: PathBuf) -> Self {
///         MySource::File(path)
///     }
///
///     fn programmatic() -> Self {
///         MySource::Programmatic
///     }
///
///     fn environment() -> Self {
///         MySource::Environment
///     }
/// }
/// # }
/// ```
///
/// # Using Custom Source Types with Derive Macro
///
/// The `#[derive(Config)]` macro generates generic implementations that work
/// with any custom source type:
///
/// ```
/// # #[cfg(feature = "std")] {
/// use feuilletage::{CustomSource, Context, ContextValue, ErrorTracker, FromContextValue, OrderedMap};
/// use std::path::{Path, PathBuf};
///
/// // Define a custom source
/// #[derive(Clone, Debug, PartialEq, Default)]
/// enum AppSource {
///     #[default]
///     Default,
///     File(PathBuf),
///     Database(String),
/// }
///
/// impl CustomSource for AppSource {
///     fn display_name(&self) -> String {
///         match self {
///             AppSource::Default => "default".to_string(),
///             AppSource::File(p) => p.display().to_string(),
///             AppSource::Database(name) => format!("db:{}", name),
///         }
///     }
///     fn file_path(&self) -> Option<&Path> {
///         match self { AppSource::File(p) => Some(p), _ => None }
///     }
///     fn from_file(path: PathBuf) -> Self { AppSource::File(path) }
///     fn programmatic() -> Self { AppSource::Default }
///     fn environment() -> Self { AppSource::Default }
/// }
///
/// // Use derive macro - generates generic FromContextValue impl
/// #[derive(Debug, feuilletage::Config)]
/// struct AppConfig {
///     #[feuilletage(default = "myapp")]
///     name: String,
///     #[feuilletage(default = 8080)]
///     port: u16,
/// }
///
/// // Create a value with custom source type
/// let ctx = Context::<AppSource, feuilletage::Level>::new(
///     AppSource::Database("config_db".to_string()),
///     feuilletage::Level::User,
/// );
/// let mut map = OrderedMap::default();
/// map.insert("name".to_string(), ContextValue::string("custom_app", ctx.clone()));
/// map.insert("port".to_string(), ContextValue::int(9000, ctx.clone()));
/// let value = ContextValue::object(map, ctx);
///
/// // Deserialize using the custom source type
/// let mut tracker = ErrorTracker::new();
/// let config: AppConfig = AppConfig::from_context_value(&value, &mut tracker).unwrap();
///
/// assert_eq!(config.name, "custom_app");
/// assert_eq!(config.port, 9000);
/// # }
/// ```
pub trait CustomSource: Clone + Debug + PartialEq + Default + Send + Sync + 'static {
    /// Returns a display name for error messages and debugging.
    fn display_name(&self) -> String;

    /// Returns the file path if this source represents a file.
    ///
    /// Override this if your custom source has an associated file path
    /// that should be used for relative path resolution.
    #[cfg(feature = "std")]
    fn file_path(&self) -> Option<&Path>;

    /// Creates a source representing a file at the given path.
    #[cfg(feature = "std")]
    fn from_file(path: PathBuf) -> Self;

    /// Creates a source representing a file at the given path (no_std version).
    #[cfg(not(feature = "std"))]
    fn from_file(path: String) -> Self;

    /// Creates a source representing programmatically set values.
    ///
    /// Defaults to `Self::default()`. Override if you have a specific
    /// variant for programmatically set values.
    fn programmatic() -> Self {
        Self::default()
    }

    /// Creates a source representing values from environment variables.
    ///
    /// Defaults to `Self::default()`. Override if you have a specific
    /// variant for environment-sourced values.
    fn environment() -> Self {
        Self::default()
    }
}

/// Blanket implementation: any CustomSource automatically implements SourceType.
impl<T: CustomSource> SourceType for T {
    fn display_name(&self) -> String {
        CustomSource::display_name(self)
    }

    #[cfg(feature = "std")]
    fn file_path(&self) -> Option<&Path> {
        CustomSource::file_path(self)
    }

    #[cfg(feature = "std")]
    fn from_file(path: PathBuf) -> Self {
        CustomSource::from_file(path)
    }

    #[cfg(not(feature = "std"))]
    fn from_file(path: String) -> Self {
        CustomSource::from_file(path)
    }

    fn programmatic() -> Self {
        CustomSource::programmatic()
    }

    fn environment() -> Self {
        CustomSource::environment()
    }
}

// ============================================================================
// Level Type Traits
// ============================================================================

/// Internal trait for level types used in [`Config`](struct@crate::Config).
///
/// This trait is sealed and cannot be implemented directly. Instead:
/// - Use the built-in [`Level`] enum for standard levels
/// - Implement [`CustomLevel`] to create domain-specific level types
pub trait LevelType:
    private::SealedLevel + Clone + Debug + Default + Send + Sync + PartialEq + Eq + Hash + 'static
{
    /// Returns the name of this level, used for `mutable_by` matching and display.
    fn name(&self) -> &str;

    /// Returns the merge priority of this level.
    ///
    /// Higher-priority levels override lower-priority levels. Equal priorities
    /// retain source insertion order.
    fn priority(&self) -> u32;
}

/// Trait for custom level types.
///
/// Implement this trait to create domain-specific configuration levels that can be used
/// with [`Config<S, L>`](struct@crate::Config). Implementing `CustomLevel` automatically
/// provides the internal [`LevelType`] trait via a blanket implementation.
///
/// # Examples
///
/// ```
/// use feuilletage::CustomLevel;
///
/// #[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
/// enum MyLevel {
///     Team,
///     #[default]
///     Project,
/// }
///
/// impl CustomLevel for MyLevel {
///     fn name(&self) -> &str {
///         match self {
///             MyLevel::Team => "team",
///             MyLevel::Project => "project",
///         }
///     }
///
///     fn priority(&self) -> u32 {
///         match self {
///             MyLevel::Team => 100,
///             MyLevel::Project => 200,
///         }
///     }
/// }
/// ```
///
/// # Using Custom Level Types with Derive Macro
///
/// The `#[derive(Config)]` macro generates generic implementations that work
/// with any custom level type:
///
/// ```
/// use feuilletage::{CustomLevel, Context, ContextValue, ErrorTracker, FromContextValue, OrderedMap, Source};
///
/// // Define a custom level with different priority semantics
/// #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
/// enum DeployLevel {
///     #[default]
///     Development,
///     Staging,
///     Production,
/// }
///
/// impl CustomLevel for DeployLevel {
///     fn name(&self) -> &str {
///         match self {
///             DeployLevel::Development => "development",
///             DeployLevel::Staging => "staging",
///             DeployLevel::Production => "production",
///         }
///     }
///
///     fn priority(&self) -> u32 {
///         match self {
///             DeployLevel::Development => 0,
///             DeployLevel::Staging => 100,
///             DeployLevel::Production => 200,
///         }
///     }
/// }
///
/// // Use derive macro - generates generic FromContextValue impl
/// #[derive(Debug, feuilletage::Config)]
/// struct DeployConfig {
///     #[feuilletage(default = "localhost")]
///     host: String,
///     #[feuilletage(default = false)]
///     debug: bool,
/// }
///
/// // Create a value with custom level type (using default Source)
/// let ctx = Context::<Source, DeployLevel>::new(
///     Source::Programmatic,
///     DeployLevel::Production,
/// );
/// let mut map = OrderedMap::default();
/// map.insert("host".to_string(), ContextValue::string("prod.example.com", ctx.clone()));
/// map.insert("debug".to_string(), ContextValue::bool(false, ctx.clone()));
/// let value = ContextValue::object(map, ctx);
///
/// // Deserialize using the custom level type
/// let mut tracker = ErrorTracker::new();
/// let config: DeployConfig = DeployConfig::from_context_value(&value, &mut tracker).unwrap();
///
/// assert_eq!(config.host, "prod.example.com");
/// assert_eq!(config.debug, false);
/// ```
pub trait CustomLevel:
    Clone + Debug + Default + Send + Sync + PartialEq + Eq + Hash + 'static
{
    /// Returns the name of this level, used for `mutable_by` matching and display.
    fn name(&self) -> &str;

    /// Returns the merge priority of this level.
    ///
    /// Higher values take precedence. The default preserves the historical
    /// insertion-order behavior for existing custom level implementations.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(all(feature = "std", feature = "json"))] {
    /// use feuilletage::{ConfigLoaderBuilder, ContextValue, CustomLevel, Format, Source};
    ///
    /// #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
    /// enum DeployLevel {
    ///     Production,
    ///     #[default]
    ///     Base,
    /// }
    ///
    /// impl CustomLevel for DeployLevel {
    ///     fn name(&self) -> &str {
    ///         match self {
    ///             Self::Production => "production",
    ///             Self::Base => "base",
    ///         }
    ///     }
    ///
    ///     fn priority(&self) -> u32 {
    ///         match self {
    ///             Self::Production => 100,
    ///             Self::Base => 0,
    ///         }
    ///     }
    /// }
    ///
    /// let config = ConfigLoaderBuilder::<Source, DeployLevel>::new()
    ///     .load_str(r#"{"host": "production"}"#, Format::Json, DeployLevel::Production)?
    ///     .load_str(r#"{"host": "base"}"#, Format::Json, DeployLevel::Base)?
    ///     .build()?;
    ///
    /// assert!(matches!(
    ///     config.get("host"),
    ///     Some(ContextValue::String(host, _)) if host == "production"
    /// ));
    /// # }
    /// # Ok::<(), feuilletage::Error>(())
    /// ```
    fn priority(&self) -> u32 {
        0
    }
}

/// Blanket implementation: any CustomLevel automatically implements LevelType.
impl<T: CustomLevel> LevelType for T {
    fn name(&self) -> &str {
        CustomLevel::name(self)
    }

    fn priority(&self) -> u32 {
        CustomLevel::priority(self)
    }
}

// ============================================================================
// Level
// ============================================================================

/// Configuration level indicating the priority of a config source.
///
/// Levels are used for:
/// - Determining merge order (higher-priority levels override lower-priority levels)
/// - `mutable_by` constraints (restricting which levels can modify a field)
///
/// The built-in levels are:
/// - `System`: System-wide configuration
/// - `User`: User-level configuration
/// - `Local`: Local/project-level configuration
///
/// For custom levels, implement the [`CustomLevel`] trait on your own type
/// and use it as the type parameter `L` in [`Config<S, L>`](struct@crate::Config).
///
/// # Examples
///
/// ```
/// use feuilletage::Level;
///
/// // Using built-in levels
/// let level = Level::User;
/// assert_eq!(level.name(), "user");
/// assert_eq!(level.priority(), 100);
/// ```
///
/// For custom levels:
///
/// ```
/// use feuilletage::CustomLevel;
///
/// #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
/// enum MyLevel {
///     #[default]
///     Team,
///     Project,
/// }
///
/// impl CustomLevel for MyLevel {
///     fn name(&self) -> &str {
///         match self {
///             MyLevel::Team => "team",
///             MyLevel::Project => "project",
///         }
///     }
///
///     fn priority(&self) -> u32 {
///         match self {
///             MyLevel::Team => 100,
///             MyLevel::Project => 200,
///         }
///     }
/// }
///
/// // MyLevel can now be used as the level type parameter: Config<Source, MyLevel>
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Level {
    /// System-level configuration
    System,
    /// User-level configuration
    User,
    /// Local/project-level configuration
    #[default]
    Local,
}

impl Level {
    /// Returns the name of this level.
    ///
    /// Names are used for `mutable_by` matching and display purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Level;
    ///
    /// assert_eq!(Level::System.name(), "system");
    /// assert_eq!(Level::User.name(), "user");
    /// assert_eq!(Level::Local.name(), "local");
    /// ```
    pub fn name(&self) -> &str {
        match self {
            Level::System => "system",
            Level::User => "user",
            Level::Local => "local",
        }
    }

    /// Returns this level's merge priority.
    ///
    /// Higher-priority levels override lower-priority levels regardless of
    /// source insertion order.
    pub fn priority(&self) -> u32 {
        match self {
            Level::System => 0,
            Level::User => 100,
            Level::Local => 200,
        }
    }
}

/// Built-in Level implements LevelType directly (not via CustomLevel)
/// Built-in Level implements CustomLevel, which provides LevelType via blanket impl.
impl CustomLevel for Level {
    fn name(&self) -> &str {
        match self {
            Level::System => "system",
            Level::User => "user",
            Level::Local => "local",
        }
    }

    fn priority(&self) -> u32 {
        Level::priority(self)
    }
}

impl Ord for Level {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl PartialOrd for Level {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl core::fmt::Display for Level {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.name())
    }
}

// ============================================================================
// Source
// ============================================================================

/// The source of a configuration value.
///
/// Identifies where a configuration value originated from.
///
/// For custom sources, implement the [`CustomSource`] trait on your own type
/// and use it as the type parameter `S` in [`Config<S, L>`](struct@crate::Config).
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")] {
/// use std::path::PathBuf;
/// use feuilletage::Source;
///
/// // Built-in sources
/// let file = Source::File(PathBuf::from("/etc/app/config.yaml"));
/// let env = Source::Environment;
/// let prog = Source::Programmatic;
/// # }
/// ```
///
/// For custom sources:
///
/// ```
/// # #[cfg(feature = "std")] {
/// use std::path::{Path, PathBuf};
/// use feuilletage::CustomSource;
///
/// #[derive(Clone, Debug, Default, PartialEq)]
/// struct PackageSource { name: String, path: String }
///
/// impl CustomSource for PackageSource {
///     fn display_name(&self) -> String {
///         format!("package:{}", self.name)
///     }
///
///     fn file_path(&self) -> Option<&Path> {
///         Some(Path::new(&self.path))
///     }
///
///     fn from_file(path: PathBuf) -> Self {
///         Self { name: "file".to_string(), path: path.display().to_string() }
///     }
///
///     fn programmatic() -> Self {
///         Self::default()
///     }
///
///     fn environment() -> Self {
///         Self { name: "env".to_string(), path: String::new() }
///     }
/// }
///
/// // PackageSource can now be used as the source type parameter: Config<PackageSource, Level>
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Source {
    /// Loaded from a file
    #[cfg(feature = "std")]
    File(PathBuf),
    /// Loaded from a file (String path when std is disabled)
    #[cfg(not(feature = "std"))]
    File(String),
    /// Loaded from environment variables
    Environment,
    /// Programmatically set
    Programmatic,
    /// Default value
    #[default]
    Default,
}

impl Source {
    /// Returns a display name for this source.
    ///
    /// Used for error messages and debugging.
    pub fn display_name(&self) -> String {
        match self {
            #[cfg(feature = "std")]
            Source::File(path) => path.display().to_string(),
            #[cfg(not(feature = "std"))]
            Source::File(path) => path.clone(),
            Source::Environment => "environment".to_string(),
            Source::Programmatic => "programmatic".to_string(),
            Source::Default => "default".to_string(),
        }
    }

    /// Returns the file path if this source represents a file.
    ///
    /// This is used for relative path resolution in transforms.
    #[cfg(feature = "std")]
    pub fn file_path(&self) -> Option<&Path> {
        match self {
            Source::File(path) => Some(path.as_path()),
            _ => None,
        }
    }
}

/// Built-in Source implements CustomSource, which provides SourceType via blanket impl.
impl CustomSource for Source {
    fn display_name(&self) -> String {
        match self {
            #[cfg(feature = "std")]
            Source::File(path) => path.display().to_string(),
            #[cfg(not(feature = "std"))]
            Source::File(path) => path.clone(),
            Source::Environment => "environment".to_string(),
            Source::Programmatic => "programmatic".to_string(),
            Source::Default => "default".to_string(),
        }
    }

    #[cfg(feature = "std")]
    fn file_path(&self) -> Option<&Path> {
        match self {
            Source::File(path) => Some(path.as_path()),
            _ => None,
        }
    }

    #[cfg(feature = "std")]
    fn from_file(path: PathBuf) -> Self {
        Source::File(path)
    }

    #[cfg(not(feature = "std"))]
    fn from_file(path: String) -> Self {
        Source::File(path)
    }

    fn programmatic() -> Self {
        Source::Programmatic
    }

    fn environment() -> Self {
        Source::Environment
    }
}

impl core::fmt::Display for Source {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Format
// ============================================================================

/// Format of configuration source.
///
/// Identifies the format of the configuration data, which affects parsing and serialization.
///
/// # Examples
///
/// ```
/// use feuilletage::Format;
///
/// let format = Format::Json;
/// assert_eq!(format.to_string(), "JSON");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// TOML format
    Toml,
    /// Unknown or undetected format
    Unknown,
}

impl Format {
    /// Get the default format based on enabled features
    ///
    /// Automatic fallback priority: yaml > toml > json
    #[allow(unreachable_code)]
    pub const fn default_format() -> Self {
        #[cfg(feature = "yaml")]
        return Format::Yaml;

        #[cfg(all(not(feature = "yaml"), feature = "toml"))]
        return Format::Toml;

        #[cfg(all(not(feature = "yaml"), not(feature = "toml"), feature = "json"))]
        return Format::Json;

        #[cfg(not(any(feature = "yaml", feature = "toml", feature = "json")))]
        return Format::Unknown;
    }

    pub(crate) fn ensure_enabled(&self) -> Result<(), crate::Error> {
        let message = match self {
            Format::Json if cfg!(feature = "json") => return Ok(()),
            Format::Yaml if cfg!(feature = "yaml") => return Ok(()),
            Format::Toml if cfg!(feature = "toml") => return Ok(()),
            Format::Json => "JSON feature not enabled",
            Format::Yaml => "YAML feature not enabled",
            Format::Toml => "TOML feature not enabled",
            Format::Unknown => "configuration format must be explicit",
        };

        Err(crate::Error::FormatNotSupported {
            format: self.to_string().to_lowercase(),
            message: message.to_string(),
        })
    }
}

impl Default for Format {
    fn default() -> Self {
        Format::default_format()
    }
}

impl core::fmt::Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Format::Json => write!(f, "JSON"),
            Format::Yaml => write!(f, "YAML"),
            Format::Toml => write!(f, "TOML"),
            Format::Unknown => write!(f, "Unknown"),
        }
    }
}

// ============================================================================
// MutabilityConstraint
// ============================================================================

/// Mutability constraints for configuration values.
///
/// Controls whether and by whom a configuration value can be modified.
/// Constraints are checked by comparing level names (strings), making them
/// compatible with both built-in and custom levels.
///
/// # Examples
///
/// ```
/// use feuilletage::{Level, MutabilityConstraint};
///
/// // Fully mutable (default)
/// let mutable = MutabilityConstraint::Mutable;
/// assert!(mutable.allows(&Level::User));
///
/// // Only modifiable by specific level names
/// let local_only = MutabilityConstraint::mutable_by(&["local"]);
/// assert!(local_only.allows(&Level::Local));
/// assert!(!local_only.allows(&Level::User));
///
/// // Completely immutable
/// let immutable = MutabilityConstraint::Immutable;
/// assert!(!immutable.allows(&Level::System));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutabilityConstraint {
    /// Value can be set/overridden by any level
    #[default]
    Mutable,
    /// Value can only be set/overridden by levels with names in this list
    MutableByName(Vec<String>),
    /// Value is completely immutable once set
    Immutable,
}

impl MutabilityConstraint {
    /// Check if the given level can mutate this value.
    ///
    /// Comparison is done by level name, so this works with both
    /// built-in levels and custom levels.
    pub fn allows<L: LevelType>(&self, level: &L) -> bool {
        match self {
            MutabilityConstraint::Mutable => true,
            MutabilityConstraint::Immutable => false,
            MutabilityConstraint::MutableByName(names) => names.iter().any(|n| n == level.name()),
        }
    }

    /// Create a MutableByName constraint from level names.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Level, MutabilityConstraint};
    ///
    /// let constraint = MutabilityConstraint::mutable_by(&["local", "user"]);
    /// assert!(constraint.allows(&Level::Local));
    /// assert!(constraint.allows(&Level::User));
    /// assert!(!constraint.allows(&Level::System));
    /// ```
    pub fn mutable_by(level_names: &[&str]) -> Self {
        MutabilityConstraint::MutableByName(level_names.iter().map(|s| s.to_string()).collect())
    }
}

// ============================================================================
// Context
// ============================================================================

/// Context information about where a configuration value came from.
///
/// Tracks the source, format, priority level, and mutability of each configuration value.
/// This metadata is preserved through merges and can be used for debugging and access control.
///
/// # Type Parameters
///
/// - `S`: Source type, defaults to [`Source`]
/// - `L`: Level type, defaults to [`Level`]
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")] {
/// use feuilletage::{Context, Level, Source, Format};
///
/// // Create a context for user-level programmatic configuration
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(ctx.level, Level::User);
/// assert_eq!(ctx.format, Format::Unknown);
///
/// // Use new_from_file for format auto-detection from file extension
/// let file_ctx = Context::new_from_file("config.json".into(), Level::Local);
/// assert_eq!(file_ctx.format, Format::Json);
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    bound = "S: SourceType + Serialize + for<'a> Deserialize<'a>, L: LevelType + Serialize + for<'a> Deserialize<'a>"
)]
pub struct Context<S: SourceType = Source, L: LevelType = Level> {
    /// The source of this configuration (e.g., file path)
    pub source: S,
    /// The format of the source
    pub format: Format,
    /// The level/priority of this configuration
    pub level: L,
    /// Mutability constraint for this value
    pub mutability: MutabilityConstraint,
}

impl<S: SourceType, L: LevelType> Default for Context<S, L>
where
    S: Default,
    L: Default,
{
    fn default() -> Self {
        Context {
            source: S::default(),
            format: Format::Unknown,
            level: L::default(),
            mutability: MutabilityConstraint::default(),
        }
    }
}

impl<S: SourceType, L: LevelType> Context<S, L> {
    /// Creates a new Context with the given source and level.
    ///
    /// The format defaults to `Unknown`. Use `with_format` to set it explicitly,
    /// or use `new_from_file` when loading from a file to auto-detect the format.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Context, Level, Source, Format};
    ///
    /// // Programmatic source - format is Unknown
    /// let ctx = Context::new(Source::Programmatic, Level::User);
    /// assert_eq!(ctx.format, Format::Unknown);
    /// ```
    pub fn new(source: S, level: L) -> Self {
        Self {
            source,
            format: Format::Unknown,
            level,
            mutability: MutabilityConstraint::default(),
        }
    }

    /// Sets the format for this context.
    ///
    /// Use this builder method when the format cannot be auto-detected
    /// or needs to be overridden.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Context, Level, Source, Format};
    ///
    /// let ctx = Context::new(Source::Programmatic, Level::User)
    ///     .with_format(Format::Json);
    /// assert_eq!(ctx.format, Format::Json);
    /// ```
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    /// Sets the mutability constraint for this context.
    ///
    /// This controls whether and by whom the value can be overridden.
    pub fn with_mutability_constraint(mut self, constraint: MutabilityConstraint) -> Self {
        self.mutability = constraint;
        self
    }

    /// Sets this context to be mutable only by the specified level names.
    ///
    /// This is a convenience method that creates a `MutableByName` constraint.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::{Context, Level, Source};
    ///
    /// let ctx = Context::new(Source::Programmatic, Level::System)
    ///     .with_mutable_by(&["local"]);
    ///
    /// assert!(ctx.can_be_overridden_by(&Level::Local));
    /// assert!(!ctx.can_be_overridden_by(&Level::User));
    /// ```
    pub fn with_mutable_by(mut self, level_names: &[&str]) -> Self {
        self.mutability = MutabilityConstraint::mutable_by(level_names);
        self
    }

    /// Returns `true` if a value with this context can be overridden by the given level.
    ///
    /// This checks the mutability constraint to determine if the level is allowed
    /// to modify values with this context.
    pub fn can_be_overridden_by(&self, level: &L) -> bool {
        self.mutability.allows(level)
    }
}

/// Additional methods specific to contexts using the built-in Source type.
impl<L: LevelType> Context<Source, L> {
    /// Creates a new Context from a file path, auto-detecting the format.
    ///
    /// The format is determined from the file extension:
    /// - `.json` → JSON
    /// - `.yaml` or `.yml` → YAML
    /// - `.toml` → TOML
    /// - Other → Unknown
    #[cfg(feature = "std")]
    pub fn new_from_file(path: PathBuf, level: L) -> Self {
        let format = {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            match ext {
                "json" => Format::Json,
                "yaml" | "yml" => Format::Yaml,
                "toml" => Format::Toml,
                _ => Format::Unknown,
            }
        };

        Self {
            source: Source::File(path),
            format,
            level,
            mutability: MutabilityConstraint::default(),
        }
    }
}

// Unit tests have been moved to feuilletage/tests/unit/context_test.rs

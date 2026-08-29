//! Edit API for configuration modification.
//!
//! This module provides a fluent API for navigating and modifying configuration values
//! using path-based access. The main entry point is [`ConfigEntry`], which represents
//! a reference to a location in the config tree.
//!
//! # Overview
//!
//! The Edit API allows you to:
//! - Navigate to any path in the configuration tree
//! - Create nested structures automatically when setting values
//! - Read, update, and delete values
//! - Manipulate arrays with push, map, filter operations
//! - Prune empty parent objects after removal
//!
//! # Path Syntax
//!
//! Paths can be specified in several ways:
//!
//! | Syntax | Result |
//! |--------|--------|
//! | `"a.b.c"` | `["a", "b", "c"]` |
//! | `"a.b\\.c.d"` | `["a", "b.c", "d"]` (escaped dot) |
//! | `&["a", "b.c", "d"]` | `["a", "b.c", "d"]` (literal segments) |
//! | `at_raw("a.b.c")` | `["a.b.c"]` (no splitting) |
//!
//! # Examples
//!
//! ## Creating and Setting Values
//!
//! ```
//! use feuilletage::Config;
//!
//! let mut config = Config::default();
//!
//! // Create nested structure and set values
//! config.at("server.host").set("localhost").unwrap();
//! config.at("server.port").set(8080).unwrap();
//!
//! // Use at_raw for keys containing dots
//! config.at("plugins").at_raw("auth.v2").at("enabled").set(true).unwrap();
//!
//! // Verify the values were set
//! assert!(config.at("server.host").exists());
//! assert!(config.at("plugins").at_raw("auth.v2").at("enabled").exists());
//! ```
//!
//! ## Reading Values
//!
//! ```
//! use feuilletage::{Config, ContextValue, Value};
//!
//! let mut config = Config::default();
//! config.at("server.host").set("localhost").unwrap();
//! config.at("server.port").set(8080).unwrap();
//!
//! // Check if path exists
//! if config.at("server.host").exists() {
//!     // Get value (returns Option<&ContextValue>)
//!     if let Some(value) = config.at("server.host").get() {
//!         assert!(matches!(value, ContextValue::String(s, _) if s == "localhost"));
//!     }
//! }
//!
//! // Get with default
//! let port = config.at("server.port").get_or(Value::Int(3000));
//! assert!(matches!(port, Value::Int(8080)));
//!
//! let missing = config.at("server.missing").get_or(Value::Int(3000));
//! assert!(matches!(missing, Value::Int(3000)));
//! ```
//!
//! ## Removing Values
//!
//! ```
//! use feuilletage::Config;
//!
//! let mut config = Config::default();
//! config.at("old.key").set("value").unwrap();
//! config.at("feature.deprecated.setting").set("old").unwrap();
//!
//! // Simple remove (returns the removed value)
//! let removed = config.at("old.key").remove().value();
//! assert!(removed.is_some());
//!
//! // Remove with pruning (removes empty parent objects)
//! config.at("feature.deprecated.setting").remove().prune();
//! assert!(!config.at("feature").exists()); // Parent pruned
//! ```
//!
//! ## Array Operations
//!
//! ```
//! use feuilletage::{Config, ContextValue};
//!
//! let mut config = Config::default();
//!
//! // Push to array (creates array if doesn't exist)
//! config.at("tags").push("production").unwrap();
//! config.at("tags").push("stable").unwrap();
//!
//! // Verify array was created with two elements
//! assert!(config.at("tags").is_array());
//!
//! // Transform each element
//! config.at("tags").map(|cv| {
//!     if let ContextValue::String(ref mut s, _) = cv {
//!         *s = s.to_uppercase();
//!     }
//! }).unwrap();
//!
//! // Verify transformation
//! if let Some(ContextValue::Array(arr, _)) = config.at("tags").get() {
//!     assert!(matches!(&arr[0], ContextValue::String(s, _) if s == "PRODUCTION"));
//! }
//!
//! // Create numbers array for filter example
//! let mut config2 = Config::default();
//! config2.at("numbers").push(1).unwrap();
//! config2.at("numbers").push(-2).unwrap();
//! config2.at("numbers").push(3).unwrap();
//!
//! // Filter elements
//! config2.at("numbers").filter(|cv| {
//!     matches!(cv, ContextValue::Int(n, _) if *n > 0)
//! }).unwrap();
//!
//! // Verify filter kept only positive numbers
//! if let Some(ContextValue::Array(arr, _)) = config2.at("numbers").get() {
//!     assert_eq!(arr.len(), 2); // Only 1 and 3 remain
//! }
//! ```

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec, vec::Vec};

use crate::__private::IndexMap;
use crate::config::Config;
use crate::context::{Level, LevelType, Source, SourceType};
use crate::error::Error;
use crate::value::{ContextValue, Value};

/// Trait for types that can be converted into a path.
///
/// This trait enables flexible path specification for navigation methods.
///
/// ```
/// use feuilletage::IntoPath;
///
/// assert_eq!("server.http.port".into_path(), ["server", "http", "port"]);
/// assert_eq!("labels.release\\.name".into_path(), ["labels", "release.name"]);
/// ```
pub trait IntoPath {
    /// Convert this value into a vector of path segments.
    fn into_path(self) -> Vec<String>;
}

impl IntoPath for &str {
    /// Parse a dot-separated path string.
    ///
    /// Dots can be escaped with backslash: `"a\\.b"` becomes segment `"a.b"`.
    fn into_path(self) -> Vec<String> {
        parse_path(self)
    }
}

impl IntoPath for String {
    fn into_path(self) -> Vec<String> {
        parse_path(&self)
    }
}

impl IntoPath for &String {
    fn into_path(self) -> Vec<String> {
        parse_path(self)
    }
}

impl IntoPath for &[&str] {
    /// Use literal segments without any dot splitting.
    fn into_path(self) -> Vec<String> {
        self.iter().map(|s| (*s).to_string()).collect()
    }
}

impl<const N: usize> IntoPath for [&str; N] {
    fn into_path(self) -> Vec<String> {
        self.iter().map(|s| (*s).to_string()).collect()
    }
}

impl<const N: usize> IntoPath for &[&str; N] {
    fn into_path(self) -> Vec<String> {
        self.iter().map(|s| (*s).to_string()).collect()
    }
}

impl IntoPath for Vec<String> {
    fn into_path(self) -> Vec<String> {
        self
    }
}

impl IntoPath for &[String] {
    fn into_path(self) -> Vec<String> {
        self.to_vec()
    }
}

/// Parse a dot-separated path string, handling escaped dots.
///
/// - Regular dots separate path segments
/// - `\.` is an escaped dot that becomes part of the segment
///
/// # Examples
///
/// ```
/// use feuilletage::edit::IntoPath;
///
/// // parse_path is called internally by IntoPath::into_path()
/// assert_eq!("a.b.c".into_path(), vec!["a", "b", "c"]);
/// assert_eq!("a.b\\.c.d".into_path(), vec!["a", "b.c", "d"]);
/// assert_eq!("a\\.b\\.c".into_path(), vec!["a.b.c"]);
/// ```
fn parse_path(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Check if next char is a dot (escaped dot)
                if chars.peek() == Some(&'.') {
                    chars.next(); // consume the dot
                    current.push('.');
                } else {
                    // Not an escape sequence, keep the backslash
                    current.push('\\');
                }
            }
            '.' => {
                // Unescaped dot = segment separator
                if !current.is_empty() {
                    segments.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    // Don't forget the last segment
    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

/// A reference to a location in the config tree obtained via
/// [`Config::at`](crate::Config::at).
///
/// `ConfigEntry` may be a "phantom" entry if the path doesn't exist yet.
/// Read operations like [`Self::get`] return `None`, but [`Self::set`] will
/// create the path and any intermediate objects.
///
/// ```
/// use feuilletage::Config;
///
/// let mut config = Config::default();
///
/// // Set a nested value (intermediate objects are created automatically).
/// config.at("database.host").set("localhost").unwrap();
/// config.at("database.port").set(5432_i64).unwrap();
///
/// assert!(config.at("database.host").exists());
/// assert!(config.at("database.port").exists());
/// ```
pub struct ConfigEntry<'a, S: SourceType = Source, L: LevelType = Level> {
    config: &'a mut Config<S, L>,
    path: Vec<String>,
}

impl<'a, S: SourceType, L: LevelType> ConfigEntry<'a, S, L> {
    /// Create a new ConfigEntry with the given path segments.
    pub(crate) fn new(config: &'a mut Config<S, L>, path: Vec<String>) -> Self {
        Self { config, path }
    }

    /// Navigate further into the config tree.
    ///
    /// Path segments are appended to the current path.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    ///
    /// // These two approaches produce equivalent paths
    /// config.at("a").at("b").at("c").set(1).unwrap();
    /// config.at("x.y").at("z").set(2).unwrap();
    ///
    /// assert!(config.at("a.b.c").exists());
    /// assert!(config.at("x.y.z").exists());
    /// ```
    pub fn at<P: IntoPath>(mut self, path: P) -> ConfigEntry<'a, S, L> {
        self.path.extend(path.into_path());
        self
    }

    /// Navigate with a raw key (no dot splitting).
    ///
    /// Useful for keys that contain literal dots.
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    ///
    /// // at_raw treats "auth.v2" as a single key, not "auth" -> "v2"
    /// config.at("plugins").at_raw("auth.v2").at("enabled").set(true).unwrap();
    ///
    /// // Verify the path structure
    /// assert!(config.at("plugins").at_raw("auth.v2").at("enabled").exists());
    /// // This would NOT exist because "auth.v2" is not split
    /// assert!(!config.at("plugins.auth.v2.enabled").exists());
    /// ```
    pub fn at_raw(mut self, key: &str) -> ConfigEntry<'a, S, L> {
        if !key.is_empty() {
            self.path.push(key.to_string());
        }
        self
    }

    /// Get the current path as a slice.
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// let entry = config.at("server").at("http.port");
    /// assert_eq!(entry.path(), ["server", "http", "port"]);
    /// ```
    pub fn path(&self) -> &[String] {
        &self.path
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Check if the path exists in the config.
    ///
    /// Returns `true` even if the value is `null` - the key exists.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("present").set(Value::Null).unwrap();
    /// assert!(config.at("present").exists());
    /// assert!(!config.at("missing").exists());
    /// ```
    pub fn exists(&self) -> bool {
        self.get_config_value().is_some()
    }

    /// Get a reference to the value at the current path.
    ///
    /// Returns `None` if the path doesn't exist.
    /// Returns `Some(&ContextValue)` if it exists (may be `ContextValue::Null`).
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// config.at("port").set(8080).unwrap();
    /// assert!(matches!(config.at("port").get(), Some(ContextValue::Int(8080, _))));
    /// ```
    pub fn get(&self) -> Option<&ContextValue<S, L>> {
        self.get_config_value()
    }

    /// Get the ContextValue at the current path.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get_config(&self) -> Option<&ContextValue<S, L>> {
        self.get_config_value()
    }

    /// Get a mutable ContextValue at the current path.
    ///
    /// Returns `None` if the path doesn't exist.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// config.at("port").set(8080).unwrap();
    /// if let Some(ContextValue::Int(port, _)) = config.at("port").get_config_mut() {
    ///     *port = 9090;
    /// }
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(9090, _))));
    /// ```
    pub fn get_config_mut(&mut self) -> Option<&mut ContextValue<S, L>> {
        self.get_config_value_mut()
    }

    /// Get the value or a default if missing or null.
    ///
    /// Returns the actual value if it exists and is not null,
    /// otherwise returns the provided default.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("port").set(8080).unwrap();
    /// assert_eq!(config.at("port").get_or(Value::Int(3000)), Value::Int(8080));
    /// assert_eq!(config.at("missing").get_or(Value::Int(3000)), Value::Int(3000));
    /// ```
    pub fn get_or(&self, default: Value) -> Value {
        match self.get() {
            Some(ContextValue::Null(_)) | None => default,
            Some(cv) => Value::from(cv),
        }
    }

    /// Get the value or insert a default.
    ///
    /// If the path doesn't exist, creates it with the default value.
    /// Returns a mutable reference to the ContextValue.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue, Value};
    ///
    /// let mut config = Config::default();
    /// if let ContextValue::Int(value, _) = config.at("retries").get_or_insert(Value::Int(3)) {
    ///     *value += 1;
    /// }
    /// assert!(matches!(config.get("retries"), Some(ContextValue::Int(4, _))));
    /// ```
    pub fn get_or_insert(&mut self, default: Value) -> &mut ContextValue<S, L> {
        if !self.exists() {
            // Create the path with the default value
            let _ = self.set_value_internal(default);
        }
        // Now get a mutable reference
        // We know it exists now, but we need to navigate again
        self.get_config_value_mut()
            .expect("Value should exist after set")
    }

    /// Check if the value exists and is null.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("optional").set(Value::Null).unwrap();
    /// assert!(config.at("optional").is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self.get(), Some(ContextValue::Null(_)))
    }

    /// Check if the value exists and is an array.
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("ports").push(8080).unwrap();
    /// assert!(config.at("ports").is_array());
    /// ```
    pub fn is_array(&self) -> bool {
        matches!(self.get(), Some(ContextValue::Array(_, _)))
    }

    /// Check if the value exists and is an object.
    ///
    /// ```
    /// use feuilletage::Config;
    ///
    /// let mut config = Config::default();
    /// config.at("server.port").set(8080).unwrap();
    /// assert!(config.at("server").is_object());
    /// ```
    pub fn is_object(&self) -> bool {
        matches!(self.get(), Some(ContextValue::Object(_, _)))
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Set a value at the current path, creating intermediate objects as needed.
    ///
    /// # Errors
    ///
    /// Returns an error if traversing through a non-object value
    /// (e.g., trying to set "a.b.c" when "a.b" is an integer).
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// config.at("server.port").set(8080).unwrap();
    /// assert!(matches!(config.get("server.port"), Some(ContextValue::Int(8080, _))));
    /// ```
    pub fn set<T: Into<Value>>(mut self, value: T) -> Result<(), Error> {
        self.set_value_internal(value.into())
    }

    /// Set a value only if the path doesn't already exist.
    ///
    /// Returns `Ok(true)` if the value was set, `Ok(false)` if it already existed.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// assert!(config.at("port").set_if_missing(8080).unwrap());
    /// assert!(!config.at("port").set_if_missing(3000).unwrap());
    /// assert!(matches!(config.get("port"), Some(ContextValue::Int(8080, _))));
    /// ```
    pub fn set_if_missing<T: Into<Value>>(mut self, value: T) -> Result<bool, Error> {
        if self.exists() {
            Ok(false)
        } else {
            self.set_value_internal(value.into())?;
            Ok(true)
        }
    }

    // =========================================================================
    // Delete Operations
    // =========================================================================

    /// Remove the value at the current path.
    ///
    /// Returns a `RemoveResult` which can be used to get the removed value
    /// or to prune empty ancestors.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("obsolete").set("old").unwrap();
    /// assert_eq!(config.at("obsolete").remove().value(), Some(Value::String("old".into())));
    /// assert!(!config.at("obsolete").exists());
    /// ```
    pub fn remove(mut self) -> RemoveResult<'a, S, L> {
        let removed = self.remove_value_internal();
        RemoveResult {
            removed,
            config: self.config,
            path: self.path,
        }
    }

    // =========================================================================
    // Array Operations
    // =========================================================================

    /// Push a value to an array at the current path.
    ///
    /// If the path doesn't exist, creates an array with the value.
    /// If the path exists but is not an array, returns an error.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// config.at("ports").push(8080).unwrap();
    /// config.at("ports").push(9090).unwrap();
    /// assert!(matches!(config.get("ports.1"), Some(ContextValue::Int(9090, _))));
    /// ```
    pub fn push<T: Into<Value>>(mut self, value: T) -> Result<(), Error> {
        let config_value = value.into();
        let path_str = self.path.join(".");

        if !self.exists() {
            // Create array with the single value
            let arr = vec![config_value];
            return self.set_value_internal(Value::Array(arr));
        }

        // Get the type first to avoid borrow issues
        let is_array = self.is_array();
        let type_name = self.get().map(|v| v.type_name().to_string());

        if is_array {
            let context = self.config.root().context().clone();
            if let Some(ContextValue::Array(ref mut arr, _)) = self.get_config_mut() {
                arr.push(ContextValue::new(config_value, context));
            }
            Ok(())
        } else {
            Err(Error::TypeMismatch {
                path: path_str,
                expected: "array".to_string(),
                actual: type_name.unwrap_or_else(|| "unknown".to_string()),
            })
        }
    }

    /// Transform each element in an array.
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or is not an array.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// config.at("numbers").push(1).unwrap();
    /// config.at("numbers").push(2).unwrap();
    /// config.at("numbers").map(|value| {
    ///     if let ContextValue::Int(number, _) = value {
    ///         *number *= 10;
    ///     }
    /// }).unwrap();
    /// assert!(matches!(config.get("numbers.1"), Some(ContextValue::Int(20, _))));
    /// ```
    pub fn map<F>(mut self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(&mut ContextValue<S, L>),
    {
        let path_str = self.path.join(".");
        let is_array = self.is_array();
        let exists = self.exists();
        let type_name = self.get().map(|v| v.type_name().to_string());

        if !exists {
            return Err(Error::InvalidValue {
                path: path_str,
                message: "Path does not exist".to_string(),
            });
        }

        if is_array {
            if let Some(ContextValue::Array(ref mut arr, _)) = self.get_config_mut() {
                for item in arr.iter_mut() {
                    f(item);
                }
            }
            Ok(())
        } else {
            Err(Error::TypeMismatch {
                path: path_str,
                expected: "array".to_string(),
                actual: type_name.unwrap_or_else(|| "unknown".to_string()),
            })
        }
    }

    /// Keep only elements matching a predicate.
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or is not an array.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue};
    ///
    /// let mut config = Config::default();
    /// for number in 1..=4 {
    ///     config.at("numbers").push(number).unwrap();
    /// }
    /// config.at("numbers").filter(|value| {
    ///     matches!(value, ContextValue::Int(number, _) if number % 2 == 0)
    /// }).unwrap();
    /// assert!(matches!(config.get("numbers.0"), Some(ContextValue::Int(2, _))));
    /// assert!(matches!(config.get("numbers.1"), Some(ContextValue::Int(4, _))));
    /// ```
    pub fn filter<F>(mut self, mut f: F) -> Result<(), Error>
    where
        F: FnMut(&ContextValue<S, L>) -> bool,
    {
        let path_str = self.path.join(".");
        let is_array = self.is_array();
        let exists = self.exists();
        let type_name = self.get().map(|v| v.type_name().to_string());

        if !exists {
            return Err(Error::InvalidValue {
                path: path_str,
                message: "Path does not exist".to_string(),
            });
        }

        if is_array {
            if let Some(ContextValue::Array(ref mut arr, _)) = self.get_config_mut() {
                arr.retain(|item| f(item));
            }
            Ok(())
        } else {
            Err(Error::TypeMismatch {
                path: path_str,
                expected: "array".to_string(),
                actual: type_name.unwrap_or_else(|| "unknown".to_string()),
            })
        }
    }

    /// Transform elements matching a predicate.
    ///
    /// ```
    /// use feuilletage::{Config, ContextValue, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("items").set(Value::Array(vec![
    ///     Value::Int(1),
    ///     Value::Int(2),
    ///     Value::Int(3),
    ///     Value::Int(4),
    /// ])).unwrap();
    /// config.at("items").map_where(
    ///     |value| matches!(value, ContextValue::Int(number, _) if number % 2 == 0),
    ///     |value| {
    ///         if let ContextValue::Int(number, _) = value {
    ///             *number *= 2;
    ///         }
    ///     },
    /// ).unwrap();
    ///
    /// assert!(matches!(config.get("items.0"), Some(ContextValue::Int(1, _))));
    /// assert!(matches!(config.get("items.1"), Some(ContextValue::Int(4, _))));
    /// assert!(matches!(config.get("items.2"), Some(ContextValue::Int(3, _))));
    /// assert!(matches!(config.get("items.3"), Some(ContextValue::Int(8, _))));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exist or is not an array.
    pub fn map_where<P, F>(mut self, mut predicate: P, mut transform: F) -> Result<(), Error>
    where
        P: FnMut(&ContextValue<S, L>) -> bool,
        F: FnMut(&mut ContextValue<S, L>),
    {
        let path_str = self.path.join(".");
        let is_array = self.is_array();
        let exists = self.exists();
        let type_name = self.get().map(|v| v.type_name().to_string());

        if !exists {
            return Err(Error::InvalidValue {
                path: path_str,
                message: "Path does not exist".to_string(),
            });
        }

        if is_array {
            if let Some(ContextValue::Array(ref mut arr, _)) = self.get_config_mut() {
                for item in arr.iter_mut() {
                    if predicate(item) {
                        transform(item);
                    }
                }
            }
            Ok(())
        } else {
            Err(Error::TypeMismatch {
                path: path_str,
                expected: "array".to_string(),
                actual: type_name.unwrap_or_else(|| "unknown".to_string()),
            })
        }
    }

    // =========================================================================
    // Private Helper Methods
    // =========================================================================

    /// Get a reference to the ContextValue at the current path.
    fn get_config_value(&self) -> Option<&ContextValue<S, L>> {
        if self.path.is_empty() {
            return Some(self.config.root());
        }

        let mut current = self.config.root();
        for segment in &self.path {
            current = match current {
                ContextValue::Object(map, _) => map.get(segment)?,
                ContextValue::Array(arr, _) => {
                    let idx: usize = segment.parse().ok()?;
                    arr.get(idx)?
                }
                _ => return None,
            };
        }
        Some(current)
    }

    /// Get a mutable reference to the ContextValue at the current path.
    fn get_config_value_mut(&mut self) -> Option<&mut ContextValue<S, L>> {
        if self.path.is_empty() {
            return Some(self.config.root_mut());
        }

        let mut current = self.config.root_mut();
        for segment in &self.path {
            current = match current {
                ContextValue::Object(ref mut map, _) => map.get_mut(segment)?,
                ContextValue::Array(ref mut arr, _) => {
                    let idx: usize = segment.parse().ok()?;
                    arr.get_mut(idx)?
                }
                _ => return None,
            };
        }
        Some(current)
    }

    /// Set a value at the current path, creating intermediate objects.
    fn set_value_internal(&mut self, value: Value) -> Result<(), Error> {
        if self.path.is_empty() {
            // Setting the root - replace with new value but keep same structure
            let ctx = self.config.root().context().clone();
            *self.config.root_mut() = ContextValue::new(value, ctx);
            return Ok(());
        }

        let context = self.config.root().context().clone();
        let path = self.path.clone();

        // Navigate to parent, creating intermediate objects as needed
        let mut current = self.config.root_mut();

        for (i, segment) in path.iter().enumerate() {
            let is_last = i == path.len() - 1;

            if is_last {
                // Set the final value
                match current {
                    ContextValue::Object(ref mut map, _) => {
                        map.insert(
                            segment.clone(),
                            ContextValue::new(value.clone(), context.clone()),
                        );
                        return Ok(());
                    }
                    ContextValue::Array(ref mut arr, _) => {
                        if let Ok(idx) = segment.parse::<usize>() {
                            if idx < arr.len() {
                                arr[idx] = ContextValue::new(value.clone(), context.clone());
                                return Ok(());
                            } else if idx == arr.len() {
                                // Allow appending at the next index
                                arr.push(ContextValue::new(value.clone(), context.clone()));
                                return Ok(());
                            } else {
                                return Err(Error::InvalidValue {
                                    path: path[..=i].join("."),
                                    message: format!(
                                        "Array index {} out of bounds (len: {})",
                                        idx,
                                        arr.len()
                                    ),
                                });
                            }
                        } else {
                            return Err(Error::TypeMismatch {
                                path: path[..i].join("."),
                                expected: "object (for string key)".to_string(),
                                actual: "array".to_string(),
                            });
                        }
                    }
                    other => {
                        return Err(Error::TypeMismatch {
                            path: path[..i].join("."),
                            expected: "object or array".to_string(),
                            actual: other.type_name().to_string(),
                        });
                    }
                }
            } else {
                // Navigate or create intermediate object
                match current {
                    ContextValue::Object(ref mut map, _) => {
                        if !map.contains_key(segment) {
                            // Check if next segment looks like an array index
                            let next_segment = &path[i + 1];
                            let is_array_index = next_segment.parse::<usize>().is_ok();

                            if is_array_index {
                                map.insert(
                                    segment.clone(),
                                    ContextValue::new(Value::Array(Vec::new()), context.clone()),
                                );
                            } else {
                                map.insert(
                                    segment.clone(),
                                    ContextValue::new(
                                        Value::Object(IndexMap::default()),
                                        context.clone(),
                                    ),
                                );
                            }
                        }
                        current = map.get_mut(segment).unwrap();
                    }
                    ContextValue::Array(ref mut arr, _) => {
                        if let Ok(idx) = segment.parse::<usize>() {
                            if idx < arr.len() {
                                current = &mut arr[idx];
                            } else {
                                return Err(Error::InvalidValue {
                                    path: path[..=i].join("."),
                                    message: format!(
                                        "Array index {} out of bounds (len: {})",
                                        idx,
                                        arr.len()
                                    ),
                                });
                            }
                        } else {
                            return Err(Error::TypeMismatch {
                                path: path[..i].join("."),
                                expected: "object (for string key)".to_string(),
                                actual: "array".to_string(),
                            });
                        }
                    }
                    other => {
                        return Err(Error::TypeMismatch {
                            path: path[..i].join("."),
                            expected: "object or array".to_string(),
                            actual: other.type_name().to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove the value at the current path.
    fn remove_value_internal(&mut self) -> Option<Value> {
        if self.path.is_empty() {
            // Cannot remove root
            return None;
        }

        let parent_path = &self.path[..self.path.len() - 1];
        let key = &self.path[self.path.len() - 1];

        // Navigate to parent
        let mut current = self.config.root_mut();
        for segment in parent_path {
            current = match current {
                ContextValue::Object(ref mut map, _) => map.get_mut(segment)?,
                ContextValue::Array(ref mut arr, _) => {
                    let idx: usize = segment.parse().ok()?;
                    arr.get_mut(idx)?
                }
                _ => return None,
            };
        }

        // Remove from parent
        match current {
            ContextValue::Object(ref mut map, _) => map.shift_remove(key).map(Value::from),
            ContextValue::Array(ref mut arr, _) => {
                let idx: usize = key.parse().ok()?;
                if idx < arr.len() {
                    Some(arr.remove(idx).into())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Result of a remove operation.
///
/// This struct allows chaining operations after removal, such as
/// getting the removed value or pruning empty ancestors.
pub struct RemoveResult<'a, S: SourceType = Source, L: LevelType = Level> {
    removed: Option<Value>,
    config: &'a mut Config<S, L>,
    path: Vec<String>,
}

impl<'a, S: SourceType, L: LevelType> RemoveResult<'a, S, L> {
    /// Get the removed value without pruning.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("name").set("demo").unwrap();
    /// assert_eq!(config.at("name").remove().value(), Some(Value::String("demo".into())));
    /// ```
    pub fn value(self) -> Option<Value> {
        self.removed
    }

    /// Prune empty ancestors and return the removed value.
    ///
    /// After removal, walks up the path and removes any empty objects
    /// or arrays until a non-empty ancestor is found.
    ///
    /// ```
    /// use feuilletage::{Config, Value};
    ///
    /// let mut config = Config::default();
    /// config.at("feature.old.enabled").set(false).unwrap();
    /// let removed = config.at("feature.old.enabled").remove().prune();
    /// assert_eq!(removed, Some(Value::Bool(false)));
    /// assert!(!config.at("feature").exists());
    /// ```
    pub fn prune(self) -> Option<Value> {
        if self.removed.is_some() {
            // Walk up the path and remove empty containers one at a time
            // We start from the deepest ancestor (parent of removed value)
            // and work our way up, checking emptiness after each removal
            for i in (1..self.path.len()).rev() {
                let ancestor_path: Vec<String> = self.path[..i].to_vec();

                // Check if ancestor is empty
                let is_empty = is_path_empty(self.config, &ancestor_path);

                if is_empty {
                    // Remove this empty container
                    remove_at_path(self.config, &ancestor_path);
                } else {
                    // Found a non-empty container, stop pruning
                    break;
                }
            }
        }

        self.removed
    }
}

/// Check if the value at a path is an empty container.
fn is_path_empty<S: SourceType, L: LevelType>(config: &Config<S, L>, path: &[String]) -> bool {
    let value = if path.is_empty() {
        Some(config.root())
    } else {
        let mut current = config.root();
        let mut found = true;
        for segment in path {
            current = match current {
                ContextValue::Object(map, _) => {
                    if let Some(v) = map.get(segment) {
                        v
                    } else {
                        found = false;
                        break;
                    }
                }
                ContextValue::Array(arr, _) => {
                    if let Ok(idx) = segment.parse::<usize>() {
                        if let Some(v) = arr.get(idx) {
                            v
                        } else {
                            found = false;
                            break;
                        }
                    } else {
                        found = false;
                        break;
                    }
                }
                _ => {
                    found = false;
                    break;
                }
            };
        }
        if found {
            Some(current)
        } else {
            None
        }
    };

    match value {
        Some(ContextValue::Object(map, _)) => map.is_empty(),
        Some(ContextValue::Array(arr, _)) => arr.is_empty(),
        _ => false,
    }
}

/// Remove the value at a path.
fn remove_at_path<S: SourceType, L: LevelType>(config: &mut Config<S, L>, path: &[String]) {
    if path.is_empty() {
        return;
    }

    let parent_path = &path[..path.len() - 1];
    let key = &path[path.len() - 1];

    // Navigate to parent
    let mut current = config.root_mut();
    for segment in parent_path {
        current = match current {
            ContextValue::Object(ref mut map, _) => {
                if let Some(v) = map.get_mut(segment) {
                    v
                } else {
                    return;
                }
            }
            ContextValue::Array(ref mut arr, _) => {
                if let Ok(idx) = segment.parse::<usize>() {
                    if let Some(v) = arr.get_mut(idx) {
                        v
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            _ => return,
        };
    }

    // Remove from parent
    match current {
        ContextValue::Object(ref mut map, _) => {
            map.shift_remove(key);
        }
        ContextValue::Array(ref mut arr, _) => {
            if let Ok(idx) = key.parse::<usize>() {
                if idx < arr.len() {
                    arr.remove(idx);
                }
            }
        }
        _ => {}
    }
}

// ============================================================================
// Into<Value> implementations for common types
// ============================================================================

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<u32> for Value {
    fn from(i: u32) -> Self {
        Value::Int(i as i64)
    }
}

impl From<u64> for Value {
    fn from(i: u64) -> Self {
        Value::Int(i as i64)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<usize> for Value {
    fn from(i: usize) -> Self {
        Value::Int(i as i64)
    }
}

impl From<isize> for Value {
    fn from(i: isize) -> Self {
        Value::Int(i as i64)
    }
}

// Unit tests have been moved to feuilletage/tests/unit/edit_test.rs

//! Serialization module - converts ContextValue and typed structs to various formats.
//!
//! This module provides:
//! - Methods on [`ContextValue`] to serialize to JSON, YAML, or TOML
//! - Standalone functions to serialize any [`serde::Serialize`] type to each format
//!
//! # Examples
//!
//! ## Serializing typed structs
//!
//! ```
//! # #[cfg(feature = "json")] {
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct AppConfig {
//!     host: String,
//!     port: u16,
//! }
//!
//! let config = AppConfig { host: "localhost".to_string(), port: 8080 };
//! let json = compote::to_json(&config).unwrap();
//! assert!(json.contains("localhost"));
//! # }
//! ```

#[cfg(all(
    not(feature = "std"),
    any(feature = "json", feature = "yaml", feature = "toml")
))]
use alloc::{format, vec::Vec};
#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString};

use serde::Serialize;

use crate::{
    context::{Format, LevelType, SourceType},
    error::Error,
    value::ContextValue,
};

impl<S: SourceType, L: LevelType> ContextValue<S, L> {
    /// Serialize to the original format based on context
    pub fn serialize(&self) -> Result<String, Error> {
        let format = match self.context().format {
            Format::Unknown => Format::default_format(),
            ref format => format.clone(),
        };

        self.serialize_with_format(format)
    }

    pub(crate) fn serialize_with_format(&self, format: Format) -> Result<String, Error> {
        match format {
            Format::Json => {
                #[cfg(feature = "json")]
                return self.to_json();
                #[cfg(not(feature = "json"))]
                return Err(Error::InvalidValue {
                    path: "<root>".to_string(),
                    message: "JSON feature not enabled".to_string(),
                });
            }
            Format::Yaml => {
                #[cfg(feature = "yaml")]
                return self.to_yaml();
                #[cfg(not(feature = "yaml"))]
                return Err(Error::InvalidValue {
                    path: "<root>".to_string(),
                    message: "YAML feature not enabled".to_string(),
                });
            }
            Format::Toml => {
                #[cfg(feature = "toml")]
                return self.to_toml();
                #[cfg(not(feature = "toml"))]
                return Err(Error::InvalidValue {
                    path: "<root>".to_string(),
                    message: "TOML feature not enabled".to_string(),
                });
            }
            Format::Unknown => Err(Error::InvalidValue {
                path: "<root>".to_string(),
                message: "No serialization format available".to_string(),
            }),
        }
    }

    /// Serialize to JSON
    ///
    /// Keys are sorted alphabetically for consistent output.
    #[cfg(feature = "json")]
    pub fn to_json(&self) -> Result<String, Error> {
        let json_value = config_value_to_json_sorted(self);
        serde_json::to_string_pretty(&json_value).map_err(|e| Error::InvalidValue {
            path: "<root>".to_string(),
            message: format!("JSON serialization error: {}", e),
        })
    }

    /// Serialize to YAML
    ///
    /// Keys are sorted alphabetically for consistent output.
    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> Result<String, Error> {
        // Convert to serde_json::Value with sorted keys
        let json_value = config_value_to_json_sorted(self);
        serde_saphyr::to_string(&json_value).map_err(|e| Error::InvalidValue {
            path: "<root>".to_string(),
            message: format!("YAML serialization error: {}", e),
        })
    }

    /// Serialize to TOML
    ///
    /// Keys are sorted alphabetically for consistent output.
    #[cfg(feature = "toml")]
    pub fn to_toml(&self) -> Result<String, Error> {
        let toml_value = config_value_to_toml_sorted(self);
        toml::to_string_pretty(&toml_value).map_err(|e| Error::InvalidValue {
            path: "<root>".to_string(),
            message: format!("TOML serialization error: {}", e),
        })
    }
}

/// Convert ContextValue to serde_json::Value with keys sorted alphabetically.
/// Used by `to_json` (json feature) and `to_yaml` (yaml feature); not by
/// `to_toml`, which has its own `config_value_to_toml_sorted`.
#[cfg(any(feature = "json", feature = "yaml"))]
fn config_value_to_json_sorted<S: SourceType, L: LevelType>(
    value: &ContextValue<S, L>,
) -> serde_json::Value {
    match value {
        ContextValue::Null(_) => serde_json::Value::Null,
        ContextValue::Bool(b, _) => serde_json::Value::Bool(*b),
        ContextValue::Int(i, _) => serde_json::Value::Number((*i).into()),
        ContextValue::Float(f, _) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ContextValue::String(s, _) => serde_json::Value::String(s.clone()),
        ContextValue::Array(arr, _) => {
            let items: Vec<serde_json::Value> =
                arr.iter().map(config_value_to_json_sorted).collect();
            serde_json::Value::Array(items)
        }
        ContextValue::Object(obj, _) => {
            let mut map = serde_json::Map::new();
            // Collect keys and sort them alphabetically
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            for k in keys {
                if let Some(v) = obj.get(k) {
                    map.insert(k.clone(), config_value_to_json_sorted(v));
                }
            }
            serde_json::Value::Object(map)
        }
    }
}

/// Convert ContextValue to toml::Value with keys sorted alphabetically
/// This ensures consistent output across all serialization formats
#[cfg(feature = "toml")]
fn config_value_to_toml_sorted<S: SourceType, L: LevelType>(
    value: &ContextValue<S, L>,
) -> toml::Value {
    match value {
        ContextValue::Null(_) => toml::Value::String(String::new()), // TOML doesn't have null
        ContextValue::Bool(b, _) => toml::Value::Boolean(*b),
        ContextValue::Int(i, _) => toml::Value::Integer(*i),
        ContextValue::Float(f, _) => toml::Value::Float(*f),
        ContextValue::String(s, _) => toml::Value::String(s.clone()),
        ContextValue::Array(arr, _) => {
            let items: Vec<toml::Value> = arr.iter().map(config_value_to_toml_sorted).collect();
            toml::Value::Array(items)
        }
        ContextValue::Object(obj, _) => {
            let mut map = toml::map::Map::new();
            // Collect keys and sort them alphabetically
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            for k in keys {
                if let Some(v) = obj.get(k) {
                    map.insert(k.clone(), config_value_to_toml_sorted(v));
                }
            }
            toml::Value::Table(map)
        }
    }
}

// ============================================================================
// Standalone serialization functions for Serialize types
// ============================================================================

/// Serialize a value to a pretty-printed JSON string.
///
/// This function provides a consistent API for JSON serialization across
/// the compote library. It produces human-readable, indented JSON output.
///
/// For compact (minified) output, use [`to_json_compact`].
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
///
/// # Returns
///
/// A `Result` containing the JSON string, or a [`Error`] if serialization fails.
///
/// # Availability
///
/// This function is only available when the `json` feature is enabled.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use compote::to_json;
///
/// #[derive(Serialize)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// let config = Config { host: "localhost".to_string(), port: 8080 };
/// let json = to_json(&config).unwrap();
/// assert!(json.contains("localhost"));
/// assert!(json.contains("8080"));
/// ```
#[cfg(feature = "json")]
pub fn to_json<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string_pretty(value).map_err(|e| Error::InvalidValue {
        path: "<root>".to_string(),
        message: format!("JSON serialization error: {}", e),
    })
}

/// Serialize a value to a compact JSON string.
///
/// This function produces minified JSON output without whitespace.
/// For human-readable output, use [`to_json`].
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
///
/// # Returns
///
/// A `Result` containing the compact JSON string, or a [`Error`] if serialization fails.
///
/// # Availability
///
/// This function is only available when the `json` feature is enabled.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use compote::to_json_compact;
///
/// #[derive(Serialize)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// let config = Config { host: "localhost".to_string(), port: 8080 };
/// let json = to_json_compact(&config).unwrap();
/// assert_eq!(json, r#"{"host":"localhost","port":8080}"#);
/// ```
#[cfg(feature = "json")]
pub fn to_json_compact<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|e| Error::InvalidValue {
        path: "<root>".to_string(),
        message: format!("JSON serialization error: {}", e),
    })
}

/// Serialize a value to a YAML string.
///
/// This function provides a consistent API for YAML serialization across
/// the compote library.
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
///
/// # Returns
///
/// A `Result` containing the YAML string, or a [`Error`] if serialization fails.
///
/// # Availability
///
/// This function is only available when the `yaml` feature is enabled.
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use compote::to_yaml;
///
/// #[derive(Serialize)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// let config = Config { host: "localhost".to_string(), port: 8080 };
/// let yaml = to_yaml(&config).unwrap();
/// assert!(yaml.contains("localhost"));
/// assert!(yaml.contains("8080"));
/// ```
#[cfg(feature = "yaml")]
pub fn to_yaml<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_saphyr::to_string(value).map_err(|e| Error::InvalidValue {
        path: "<root>".to_string(),
        message: format!("YAML serialization error: {}", e),
    })
}

/// Serialize a value to a TOML string.
///
/// This function provides a consistent API for TOML serialization across
/// the compote library. It produces pretty-printed TOML output.
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
///
/// # Returns
///
/// A `Result` containing the TOML string, or a [`Error`] if serialization fails.
///
/// # Availability
///
/// This function is only available when the `toml` feature is enabled.
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "toml")] {
/// use serde::Serialize;
/// use compote::to_toml;
///
/// #[derive(Serialize)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// let config = Config { host: "localhost".to_string(), port: 8080 };
/// let toml = to_toml(&config).unwrap();
/// assert!(toml.contains("localhost"));
/// assert!(toml.contains("8080"));
/// # }
/// ```
#[cfg(feature = "toml")]
pub fn to_toml<T: Serialize>(value: &T) -> Result<String, Error> {
    toml::to_string_pretty(value).map_err(|e| Error::InvalidValue {
        path: "<root>".to_string(),
        message: format!("TOML serialization error: {}", e),
    })
}

/// Serialize a value to a string using the specified format.
///
/// This function provides a unified API for serializing to any supported format.
/// It's useful when the format needs to be determined at runtime.
///
/// # Arguments
///
/// * `value` - Any type that implements `serde::Serialize`
/// * `format` - The target format (JSON, YAML, or TOML)
///
/// # Returns
///
/// A `Result` containing the serialized string, or a [`Error`] if:
/// - Serialization fails
/// - The requested format feature is not enabled
/// - The format is `Unknown` and no format features are enabled
///
/// # Examples
///
/// ```
/// use serde::Serialize;
/// use compote::{to_format, Format};
///
/// #[derive(Serialize)]
/// struct Config {
///     host: String,
///     port: u16,
/// }
///
/// let config = Config { host: "localhost".to_string(), port: 8080 };
/// #[cfg(feature = "yaml")]
/// {
///     let output = to_format(&config, Format::Yaml).unwrap();
///     assert!(output.contains("localhost"));
/// }
/// #[cfg(not(feature = "yaml"))]
/// assert!(to_format(&config, Format::Yaml).is_err());
/// ```
pub fn to_format<T: Serialize>(_value: &T, format: Format) -> Result<String, Error> {
    let format = match format {
        Format::Unknown => Format::default_format(),
        format => format,
    };

    match format {
        Format::Json => {
            #[cfg(feature = "json")]
            return to_json(_value);
            #[cfg(not(feature = "json"))]
            return Err(Error::InvalidValue {
                path: "<root>".to_string(),
                message: "JSON feature not enabled".to_string(),
            });
        }
        Format::Yaml => {
            #[cfg(feature = "yaml")]
            return to_yaml(_value);
            #[cfg(not(feature = "yaml"))]
            return Err(Error::InvalidValue {
                path: "<root>".to_string(),
                message: "YAML feature not enabled".to_string(),
            });
        }
        Format::Toml => {
            #[cfg(feature = "toml")]
            return to_toml(_value);
            #[cfg(not(feature = "toml"))]
            return Err(Error::InvalidValue {
                path: "<root>".to_string(),
                message: "TOML feature not enabled".to_string(),
            });
        }
        Format::Unknown => Err(Error::InvalidValue {
            path: "<root>".to_string(),
            message: "No serialization format available".to_string(),
        }),
    }
}

// Unit tests have been moved to compote/tests/unit/ser_test.rs

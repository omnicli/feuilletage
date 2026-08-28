//! Deserialization module - converts ContextValue to Rust types.
//!
//! This module provides the [`FromContextValue`] trait and implementations
//! for converting configuration values into Rust types.
//!
//! # Overview
//!
//! The [`FromContextValue`] trait is the core deserialization interface. It's
//! automatically implemented for types that derive `Config`, and also has
//! built-in implementations for common Rust types.
//!
//! # Built-in Implementations
//!
//! The following types have `FromContextValue` implementations:
//!
//! | Category | Types |
//! |----------|-------|
//! | Integers | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `usize` |
//! | Floats | `f32`, `f64` |
//! | Text | `String` |
//! | Boolean | `bool` |
//! | Path | `PathBuf` (with `std` feature) |
//! | Collections | `Vec<T>`, `Option<T>`, `HashMap<K, V>` where `T`/`V: FromContextValue` and `K: FromStr + Hash + Eq` |
//!
//! # Type Coercion
//!
//! Built-in implementations support flexible type coercion:
//!
//! - Strings can be parsed to numbers (`"42"` -> `42i32`)
//! - Numbers can be converted to strings (`42` -> `"42"`)
//! - Boolean strings are recognized (`"true"`, `"yes"`, `"1"`, `"on"`)
//! - Integers can represent booleans (`0` = false, non-zero = true)
//!
//! # Error Tracking
//!
//! The [`ErrorTracker`] parameter accumulates errors with
//! path context, enabling detailed error messages that show exactly where
//! problems occurred in the configuration tree.

#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec::Vec};

use crate::{
    context::{Level, LevelType, Source, SourceType},
    error::{Error, ErrorTracker},
    value::{ContextValue, Value},
};

/// Information about which config levels can modify each field.
///
/// This trait is automatically implemented for types that derive `Config`.
/// It provides mutability constraint information that is used during merge
/// to skip values from disallowed levels.
///
/// # How it works
///
/// When merging multiple config sources with different levels (System, User, Local),
/// the loader checks each field's mutability constraints before merging. If a field
/// has `#[feuilletage(mutable_by = ["user", "local"])]`, values from `System` level
/// are skipped (not merged) for that field.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "json")] {
/// use feuilletage::{Config, Context, Level, Source, FromContextValue};
///
/// #[derive(Debug, feuilletage::Config)]
/// struct AppConfig {
///     // Can be set by any level
///     #[feuilletage(default = "DefaultApp")]
///     app_name: String,
///
///     // Can only be set by user or local level
///     #[feuilletage(mutable_by = ["user", "local"], default = "none")]
///     user_preference: String,
/// }
///
/// // When loading multiple sources, mutable_by constraints are enforced
/// // by ConfigLoaderBuilder. Simple Config::load_json doesn't enforce them.
/// let mut config = Config::default();
/// config.load_json(
///     r#"{"app_name": "MyApp", "user_preference": "my_pref"}"#,
///     Context::new(Source::Programmatic, Level::User),
/// );
///
/// let app: AppConfig = config.deserialize().unwrap();
/// assert_eq!(app.app_name, "MyApp");
/// assert_eq!(app.user_preference, "my_pref");
/// # }
/// ```
///
/// When loading with ConfigLoaderBuilder:
/// - System config: `{app_name: "MyApp", user_preference: "sys_default"}`
/// - User config: `{user_preference: "my_pref"}`
///
/// Result: `{app_name: "MyApp", user_preference: "my_pref"}`
/// (System's `user_preference` was skipped due to mutable_by constraint)
pub trait MutabilityInfo {
    /// Returns a map of field paths to their allowed level names.
    ///
    /// Fields without mutable_by constraints are NOT included in this map
    /// (they can be set by any level).
    ///
    /// The returned map uses serialized field paths as keys, and
    /// slices of allowed level names as values. Level names are compared
    /// using `Level::name()`, which works for both built-in levels
    /// ("system", "user", "local") and custom levels.
    fn mutability_constraints() -> crate::merge::MutabilityConstraints;
}

/// Trait for types that can be deserialized from a [`ContextValue`].
///
/// This trait is automatically implemented for types that derive `Config`.
/// It is also implemented for common Rust types like `String`, `i32`, `bool`, etc.
///
/// # Generic Parameters
///
/// The trait has two generic parameters with default types:
/// - `S: SourceType = Source` - Source type for the context
/// - `L: LevelType = Level` - Level type for the context
///
/// Using default parameters allows simple usage without specifying types:
/// ```
/// use feuilletage::{Error, ContextValue, ErrorTracker, FromContextValue, Value};
///
/// struct Percentage(u8);
///
/// impl FromContextValue for Percentage {
///     fn from_context_value(
///         value: &ContextValue,
///         tracker: &mut ErrorTracker,
///     ) -> Result<Self, Error> {
///         let n = i64::from_context_value(value, tracker)?;
///         if n < 0 || n > 100 {
///             return Err(Error::InvalidValue {
///                 path: tracker.current_path(),
///                 message: "percentage must be between 0 and 100".to_string(),
///             });
///         }
///         Ok(Percentage(n as u8))
///     }
/// }
/// ```
///
/// # Custom Source Types
///
/// For applications with custom source types, specify the type parameters:
/// ```
/// # #[cfg(feature = "std")] {
/// use std::path::{Path, PathBuf};
/// use feuilletage::{Context, ContextValue, CustomSource, Error, ErrorTracker, FromContextValue, Level};
///
/// #[derive(Clone, Debug, Default, PartialEq)]
/// enum MySource {
///     File(PathBuf),
///     #[default]
///     Programmatic,
///     Environment,
/// }
///
/// impl CustomSource for MySource {
///     fn display_name(&self) -> String { format!("{self:?}") }
///     fn file_path(&self) -> Option<&Path> {
///         match self { Self::File(path) => Some(path), _ => None }
///     }
///     fn from_file(path: PathBuf) -> Self { Self::File(path) }
///     fn programmatic() -> Self { Self::Programmatic }
///     fn environment() -> Self { Self::Environment }
/// }
///
/// #[derive(Debug, PartialEq)]
/// struct MyType(String);
///
/// impl FromContextValue<MySource, Level> for MyType {
///     fn from_context_value(
///         value: &ContextValue<MySource, Level>,
///         tracker: &mut ErrorTracker,
///     ) -> Result<Self, Error> {
///         Ok(Self(String::from_context_value(value, tracker)?))
///     }
/// }
///
/// let value = ContextValue::string("custom", Context::<MySource>::default());
/// let parsed = MyType::from_context_value(&value, &mut ErrorTracker::new()).unwrap();
/// assert_eq!(parsed, MyType("custom".into()));
/// # }
/// ```
///
/// # Built-in Implementations
///
/// - **Primitives**: `String`, `bool`, `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `usize`, `f32`, `f64`
/// - **Path**: `PathBuf`
/// - **Collections**: `Vec<T>`, `Option<T>`, `HashMap<K, V>` where `T`/`V: FromContextValue` and `K: FromStr + Hash + Eq`
pub trait FromContextValue<S: SourceType = Source, L: LevelType = Level>: Sized {
    /// Converts a configuration value into this type.
    ///
    /// # Arguments
    ///
    /// * `value` - The configuration value to deserialize
    /// * `tracker` - Error tracker for recording errors with path context
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be converted to this type.
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error>;
}

/// Constructs a configuration type from an intermediate parsed representation.
///
/// Implement this trait on a target that uses
/// `#[feuilletage(parse_as = "WireType")]`. The generated
/// [`FromContextValue`] implementation first parses `Parsed`, then passes the
/// result here together with the original, untouched input value and the same
/// error tracker.
///
/// `S` and `L` are explicit trait parameters so implementations can support
/// custom source and level types without using `impl Trait` in method
/// signatures.
pub trait FromParsed<Parsed, S: SourceType = Source, L: LevelType = Level>: Sized {
    /// Projects `parsed` into the target configuration type.
    ///
    /// `original` is the value supplied to the target's
    /// [`FromContextValue`] implementation, before any transforms or
    /// `scalar_as` / `array_as` handling performed while parsing `Parsed`.
    /// `tracker` is the same tracker used to parse `Parsed`, with its current
    /// path preserved.
    fn from_parsed(
        parsed: Parsed,
        original: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error>;
}

/// Trait for types that expose their map key field names for detection.
///
/// This trait is automatically implemented for types that use `#[feuilletage(allow_map(key = field))]`
/// at the struct level. It returns the list of field names (including aliases) that should be
/// used to detect whether a map input represents a single item (with explicit fields) or
/// multiple items (where each key becomes a field value).
///
/// # How it works
///
/// When a `Vec<T>` field has `#[feuilletage(allow_map)]` and receives a map as input, it needs
/// to decide whether to:
/// 1. Treat the map as a single item (if any key matches a struct field name)
/// 2. Split the map into multiple items (if no key matches)
///
/// This trait provides the field names to check for that detection.
///
/// # Example
///
/// ```
/// use feuilletage::AllowMapKeys;
///
/// #[derive(feuilletage::Config)]
/// #[feuilletage(allow_map(key = repository, scalar_as = version), scalar_as = "repository")]
/// struct GithubRelease {
///     #[feuilletage(alias = "repo")]
///     repository: String,
///     version: Option<String>,
/// }
///
/// assert_eq!(GithubRelease::map_key_fields(), ["repository", "repo"]);
/// ```
pub trait AllowMapKeys {
    /// Returns the list of field names (including aliases) for the map key field.
    ///
    /// These are used by `Vec<T>` with `allow_map` to detect whether a map input
    /// should be treated as a single struct instance or split into multiple instances.
    fn map_key_fields() -> &'static [&'static str];
}

// Implementations for primitive types

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for Value {
    /// Convert a `ContextValue<S, L>` into a context-less `Value`.
    ///
    /// All context metadata (source, format, level, mutability) is
    /// discarded. The value tree is preserved exactly.
    ///
    /// This lets struct fields of type `Value` accept any input:
    /// scalars, arrays, objects — the whole subtree is captured as-is.
    /// Used by config types that need to defer interpretation of a
    /// payload (e.g. a default value that could be any shape).
    fn from_context_value(
        value: &ContextValue<S, L>,
        _tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        Ok(Value::from(value))
    }
}

impl MutabilityInfo for Value {
    fn mutability_constraints() -> crate::merge::MutabilityConstraints {
        crate::merge::MutabilityConstraints::default()
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for String {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::String(s, _) => Ok(s.clone()),
            ContextValue::Int(i, _) => Ok(i.to_string()),
            ContextValue::Float(f, _) => Ok(f.to_string()),
            ContextValue::Bool(b, _) => Ok(b.to_string()),
            _ => {
                tracker.record_type_mismatch("string", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "string".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for bool {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Bool(b, _) => Ok(*b),
            ContextValue::String(s, _) => match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Ok(true),
                "false" | "no" | "0" | "off" => Ok(false),
                _ => {
                    tracker.record_invalid_value(format!("Cannot parse '{}' as bool", s));
                    Err(Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Cannot parse '{}' as bool", s),
                    })
                }
            },
            ContextValue::Int(i, _) => Ok(*i != 0),
            _ => {
                tracker.record_type_mismatch("bool", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "bool".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for i64 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Int(i, _) => Ok(*i),
            ContextValue::Float(f, _) => Ok(*f as i64),
            ContextValue::String(s, _) => s.parse().map_err(|_| {
                tracker.record_invalid_value(format!("Cannot parse '{}' as i64", s));
                Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("Cannot parse '{}' as i64", s),
                }
            }),
            _ => {
                tracker.record_type_mismatch("i64", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "i64".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for i32 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let i64_val = <i64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        i64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for i32", i64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for i32", i64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for u64 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Int(i, _) => (*i).try_into().map_err(|_| {
                tracker.record_invalid_value(format!("{} is negative, cannot convert to u64", i));
                Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("{} is negative, cannot convert to u64", i),
                }
            }),
            ContextValue::Float(f, _) => Ok(*f as u64),
            ContextValue::String(s, _) => s.parse().map_err(|_| {
                tracker.record_invalid_value(format!("Cannot parse '{}' as u64", s));
                Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("Cannot parse '{}' as u64", s),
                }
            }),
            _ => {
                tracker.record_type_mismatch("u64", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "u64".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for u32 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let u64_val = <u64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        u64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for u32", u64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for u32", u64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for i16 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let i64_val = <i64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        i64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for i16", i64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for i16", i64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for u16 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let u64_val = <u64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        u64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for u16", u64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for u16", u64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for i8 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let i64_val = <i64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        i64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for i8", i64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for i8", i64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for u8 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let u64_val = <u64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        u64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for u8", u64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for u8", u64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for usize {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let u64_val = <u64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        u64_val.try_into().map_err(|_| {
            tracker.record_invalid_value(format!("{} is out of range for usize", u64_val));
            Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("{} is out of range for usize", u64_val),
            }
        })
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for f64 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Float(f, _) => Ok(*f),
            ContextValue::Int(i, _) => Ok(*i as f64),
            ContextValue::String(s, _) => s.parse().map_err(|_| {
                tracker.record_invalid_value(format!("Cannot parse '{}' as f64", s));
                Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("Cannot parse '{}' as f64", s),
                }
            }),
            _ => {
                tracker.record_type_mismatch("f64", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "f64".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<S: SourceType, L: LevelType> FromContextValue<S, L> for f32 {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        let f64_val = <f64 as FromContextValue<S, L>>::from_context_value(value, tracker)?;
        Ok(f64_val as f32)
    }
}

#[cfg(feature = "std")]
impl<S: SourceType, L: LevelType> FromContextValue<S, L> for PathBuf {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::String(s, _) => Ok(PathBuf::from(s)),
            _ => {
                tracker.record_type_mismatch("string (path)", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "string (path)".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<T: FromContextValue<S, L>, S: SourceType, L: LevelType> FromContextValue<S, L> for Vec<T> {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Array(arr, _) => {
                let mut result = Vec::with_capacity(arr.len());
                for (i, item) in arr.iter().enumerate() {
                    tracker.push_index(i);
                    match T::from_context_value(item, tracker) {
                        Ok(v) => result.push(v),
                        Err(e) => {
                            // Record the error but continue processing other elements.
                            // The failed element is skipped from the result.
                            tracker.record(e.clone());
                        }
                    }
                    tracker.pop();
                }
                // Always return the successfully deserialized items.
                // Errors are already recorded in the tracker for informational purposes.
                // This allows partial success: elements with defaults will succeed,
                // while truly invalid elements (missing required fields) are skipped.
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("array", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "array".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<T: FromContextValue<S, L>, S: SourceType, L: LevelType> FromContextValue<S, L> for Option<T> {
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        if value.is_null() {
            Ok(None)
        } else {
            // Graceful: on error, return None and record the error
            match T::from_context_value(value, tracker) {
                Ok(v) => Ok(Some(v)),
                Err(e) => {
                    tracker.record(e);
                    Ok(None)
                }
            }
        }
    }
}

#[cfg(feature = "std")]
impl<K, V, S: SourceType, L: LevelType> FromContextValue<S, L> for std::collections::HashMap<K, V>
where
    K: core::hash::Hash + Eq + core::str::FromStr,
    V: FromContextValue<S, L>,
{
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Object(map, _) => {
                let mut result = std::collections::HashMap::with_capacity(map.len());
                for (key, item) in map.iter() {
                    tracker.push_field(key);
                    // Parse the key from string
                    match K::from_str(key) {
                        Ok(parsed_key) => {
                            match V::from_context_value(item, tracker) {
                                Ok(v) => {
                                    result.insert(parsed_key, v);
                                }
                                Err(e) => {
                                    // Record the error but continue processing other entries.
                                    // The failed entry is skipped from the result.
                                    tracker.record(e.clone());
                                }
                            }
                        }
                        Err(_) => {
                            let err = Error::InvalidValue {
                                path: tracker.current_path(),
                                message: format!("Cannot parse key '{}' into target type", key),
                            };
                            tracker.record(err);
                        }
                    }
                    tracker.pop();
                }
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("object", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "object".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

impl<K, V, S: SourceType, L: LevelType> FromContextValue<S, L> for hashbrown::HashMap<K, V>
where
    K: core::hash::Hash + Eq + core::str::FromStr,
    V: FromContextValue<S, L>,
{
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Object(map, _) => {
                let mut result = hashbrown::HashMap::with_capacity(map.len());
                for (key, item) in map.iter() {
                    tracker.push_field(key);
                    // Parse the key from string
                    match K::from_str(key) {
                        Ok(parsed_key) => {
                            match V::from_context_value(item, tracker) {
                                Ok(v) => {
                                    result.insert(parsed_key, v);
                                }
                                Err(e) => {
                                    // Record the error but continue processing other entries.
                                    // The failed entry is skipped from the result.
                                    tracker.record(e.clone());
                                }
                            }
                        }
                        Err(_) => {
                            let err = Error::InvalidValue {
                                path: tracker.current_path(),
                                message: format!("Cannot parse key '{}' into target type", key),
                            };
                            tracker.record(err);
                        }
                    }
                    tracker.pop();
                }
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("object", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "object".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

#[cfg(feature = "std")]
impl<T: FromContextValue<S, L> + Ord, S: SourceType, L: LevelType> FromContextValue<S, L>
    for std::collections::BTreeSet<T>
{
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Array(arr, _) => {
                let mut result = std::collections::BTreeSet::new();
                for (i, item) in arr.iter().enumerate() {
                    tracker.push_index(i);
                    match T::from_context_value(item, tracker) {
                        Ok(v) => {
                            result.insert(v);
                        }
                        Err(e) => {
                            // Record the error but continue processing other elements.
                            // The failed element is skipped from the result.
                            tracker.record(e.clone());
                        }
                    }
                    tracker.pop();
                }
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("array", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "array".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

#[cfg(feature = "std")]
impl<K, V, S: SourceType, L: LevelType> FromContextValue<S, L> for std::collections::BTreeMap<K, V>
where
    K: Ord + core::str::FromStr,
    V: FromContextValue<S, L>,
{
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Object(map, _) => {
                let mut result = std::collections::BTreeMap::new();
                for (key, item) in map.iter() {
                    tracker.push_field(key);
                    // Parse the key from string
                    match K::from_str(key) {
                        Ok(parsed_key) => {
                            match V::from_context_value(item, tracker) {
                                Ok(v) => {
                                    result.insert(parsed_key, v);
                                }
                                Err(e) => {
                                    // Record the error but continue processing other entries.
                                    // The failed entry is skipped from the result.
                                    tracker.record(e.clone());
                                }
                            }
                        }
                        Err(_) => {
                            let err = Error::InvalidValue {
                                path: tracker.current_path(),
                                message: format!("Cannot parse key '{}' into target type", key),
                            };
                            tracker.record(err);
                        }
                    }
                    tracker.pop();
                }
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("object", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "object".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

#[cfg(feature = "std")]
impl<T: FromContextValue<S, L> + Eq + std::hash::Hash, S: SourceType, L: LevelType>
    FromContextValue<S, L> for std::collections::HashSet<T>
{
    fn from_context_value(
        value: &ContextValue<S, L>,
        tracker: &mut ErrorTracker,
    ) -> Result<Self, Error> {
        match value {
            ContextValue::Array(arr, _) => {
                let mut result = std::collections::HashSet::new();
                for (i, item) in arr.iter().enumerate() {
                    tracker.push_index(i);
                    match T::from_context_value(item, tracker) {
                        Ok(v) => {
                            result.insert(v);
                        }
                        Err(e) => {
                            // Record the error but continue processing other elements.
                            // The failed element is skipped from the result.
                            tracker.record(e.clone());
                        }
                    }
                    tracker.pop();
                }
                Ok(result)
            }
            _ => {
                tracker.record_type_mismatch("array", value.type_name());
                Err(Error::TypeMismatch {
                    path: tracker.current_path(),
                    expected: "array".to_string(),
                    actual: value.type_name().to_string(),
                })
            }
        }
    }
}

// Unit tests have been moved to feuilletage/tests/unit/de_test.rs

//! Template interpolation support for configuration values.
//!
//! This module provides template interpolation using `%{field}` syntax to reference
//! other fields within the same configuration struct. Templates are resolved after
//! all fields are deserialized, allowing fields to reference each other.
//!
//! # Overview
//!
//! Template interpolation enables dynamic value composition where one field's value
//! can incorporate values from other fields. This is useful for:
//!
//! - Building URLs from host/port/path components
//! - Constructing paths from directory and filename parts
//! - Creating messages that include other configuration values
//!
//! # Syntax
//!
//! | Pattern | Description |
//! |---------|-------------|
//! | `%{field_name}` | Reference another field's value |
//! | `%%{` | Escape sequence for literal `%{` |
//!
//! # Using with the Derive Macro
//!
//! Mark fields with `#[feuilletage(template)]` to enable interpolation:
//!
//! ```
//! # #[cfg(all(feature = "std", feature = "json"))] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct ServerConfig {
//!     #[feuilletage(default = "localhost")]
//!     host: String,           // "localhost"
//!     #[feuilletage(default = "8080")]
//!     port: i32,              // 8080
//!     #[feuilletage(default = "v2")]
//!     api_version: String,    // "v2"
//!
//!     #[feuilletage(template)]
//!     base_url: String,       // "http://%{host}:%{port}"
//!                             // -> "http://localhost:8080"
//!
//!     #[feuilletage(template)]
//!     api_endpoint: String,   // "%{base_url}/api/%{api_version}"
//!                             // -> "http://localhost:8080/api/v2"
//! }
//!
//! let mut config = Config::default();
//! config.load_json(
//!     r#"{"base_url": "http://%{host}:%{port}", "api_endpoint": "%{base_url}/api/%{api_version}"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let server: ServerConfig = config.deserialize().unwrap();
//! assert_eq!(server.base_url, "http://localhost:8080");
//! assert_eq!(server.api_endpoint, "http://localhost:8080/api/v2");
//! # }
//! ```
//!
//! # Dependency Resolution
//!
//! Templates that reference other templates are resolved in dependency order.
//! The system performs topological sorting to ensure referenced fields are
//! resolved before fields that depend on them.
//!
//! # Error Handling
//!
//! The following errors can occur during template processing:
//!
//! - [`TemplateError::MissingField`]: Referenced field does not exist
//! - [`TemplateError::CircularDependency`]: Templates reference each other in a cycle
//! - [`TemplateError::TypeConversionError`]: Field value cannot be converted to string
//! - [`TemplateError::SyntaxError`]: Malformed template syntax
//!
//! # Type Conversion
//!
//! When a field is referenced in a template, it is converted to a string:
//!
//! - Strings are used as-is
//! - Numbers and booleans are converted to their string representation
//! - Arrays are joined with commas by default
//! - Objects are converted to empty string (not recommended for templates)

#[cfg(not(feature = "std"))]
use alloc::{
    collections::BTreeMap,
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

use crate::error::Error;
use crate::value::ContextValue;

/// Errors that can occur during template processing.
///
/// These errors are returned when template interpolation fails. They provide
/// detailed information about what went wrong and where.
///
/// # Examples
///
/// ```
/// use feuilletage::template::TemplateError;
///
/// // Missing field error
/// let error = TemplateError::MissingField {
///     template_field: "url".to_string(),
///     referenced_field: "host".to_string(),
/// };
/// assert!(error.to_string().contains("host"));
///
/// // Circular dependency error
/// let error = TemplateError::CircularDependency {
///     cycle: vec!["a".to_string(), "b".to_string(), "a".to_string()],
/// };
/// assert!(error.to_string().contains("Circular"));
/// ```
#[derive(Debug, Clone)]
pub enum TemplateError {
    /// A referenced field does not exist in the struct.
    ///
    /// This occurs when a template like `%{unknown_field}` references
    /// a field that is not defined in the configuration struct.
    MissingField {
        /// The field containing the template
        template_field: String,
        /// The field name that was referenced but doesn't exist
        referenced_field: String,
    },

    /// Circular dependency detected between template fields.
    ///
    /// This occurs when templates reference each other in a cycle,
    /// e.g., field A references B, and B references A.
    CircularDependency {
        /// The fields involved in the cycle
        cycle: Vec<String>,
    },

    /// A referenced field has not been resolved yet (internal error).
    ///
    /// This should not occur during normal operation; it indicates
    /// a bug in the dependency resolution algorithm.
    UnresolvedReference {
        /// The field containing the template
        template_field: String,
        /// The field that should have been resolved but wasn't
        referenced_field: String,
    },

    /// A field's type cannot be converted to a string for interpolation.
    ///
    /// Most types can be converted, but complex nested objects may not
    /// produce meaningful string representations.
    TypeConversionError {
        /// The field being converted
        field: String,
        /// The type of the field
        field_type: String,
        /// Details about the conversion failure
        message: String,
    },

    /// Malformed template syntax.
    ///
    /// This occurs when a template string contains invalid syntax,
    /// such as unclosed `%{` markers.
    SyntaxError {
        /// The field containing the invalid template
        field: String,
        /// Description of the syntax error
        message: String,
    },
}

impl core::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TemplateError::MissingField {
                template_field,
                referenced_field,
            } => {
                write!(
                    f,
                    "Template error at '{}': Referenced field '{}' does not exist",
                    template_field, referenced_field
                )
            }
            TemplateError::CircularDependency { cycle } => {
                write!(
                    f,
                    "Template error: Circular dependency detected: {}",
                    cycle.join(" -> ")
                )
            }
            TemplateError::UnresolvedReference {
                template_field,
                referenced_field,
            } => {
                write!(
                    f,
                    "Template error at '{}': Field '{}' has not been resolved yet",
                    template_field, referenced_field
                )
            }
            TemplateError::TypeConversionError {
                field,
                field_type,
                message,
            } => {
                write!(
                    f,
                    "Template error at '{}': Cannot convert {} to string: {}",
                    field, field_type, message
                )
            }
            TemplateError::SyntaxError { field, message } => {
                write!(f, "Template syntax error at '{}': {}", field, message)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TemplateError {}

impl From<TemplateError> for Error {
    fn from(err: TemplateError) -> Self {
        Error::InvalidValue {
            path: String::new(),
            message: err.to_string(),
        }
    }
}

/// Extract field references from a template string.
///
/// Returns a list of field names referenced via `%{field}` syntax.
/// Escape sequences `%%{` are not treated as references.
///
/// ```
/// use feuilletage::template::extract_field_references;
///
/// assert_eq!(
///     extract_field_references("hello %{name}, value is %{count}"),
///     vec!["name".to_string(), "count".to_string()]
/// );
/// // Escaped %% is not a reference
/// assert!(extract_field_references("100%%{escaped}").is_empty());
/// ```
pub fn extract_field_references(template: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                if next == '%' {
                    // Escaped %%, skip both
                    chars.next();
                } else if next == '{' {
                    // Start of field reference
                    chars.next(); // consume '{'
                    let mut field_name = String::new();

                    while let Some(&inner) = chars.peek() {
                        if inner == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        field_name.push(inner);
                        chars.next();
                    }

                    if !field_name.is_empty() {
                        // Only add top-level field (before any dot for nested refs - future feature)
                        let top_level = field_name.split('.').next().unwrap_or(&field_name);
                        if !refs.contains(&top_level.to_string()) {
                            refs.push(top_level.to_string());
                        }
                    }
                }
            }
        }
    }

    refs
}

/// Convert a ContextValue to its string representation for template interpolation.
///
/// # Arguments
///
/// * `value` - The ContextValue to convert
/// * `vec_delimiter` - Delimiter to use when joining Vec elements (default: ",")
///
/// ```
/// use feuilletage::{Context, ContextValue, Level, Source, value_to_string};
///
/// let context = Context::new(Source::Programmatic, Level::User);
/// let value = ContextValue::array(
///     vec![
///         ContextValue::string("one", context.clone()),
///         ContextValue::string("two", context.clone()),
///     ],
///     context,
/// );
/// assert_eq!(value_to_string(&value, ":"), "one:two");
/// ```
pub fn value_to_string(value: &ContextValue, vec_delimiter: &str) -> String {
    match value {
        ContextValue::String(s, _) => s.clone(),
        ContextValue::Int(i, _) => i.to_string(),
        ContextValue::Float(f, _) => f.to_string(),
        ContextValue::Bool(b, _) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        ContextValue::Null(_) => String::new(),
        ContextValue::Array(arr, _) => {
            let parts: Vec<String> = arr
                .iter()
                .map(|v| value_to_string(v, vec_delimiter))
                .collect();
            parts.join(vec_delimiter)
        }
        ContextValue::Object(_, _) => {
            // Objects are converted to empty string by default
            // Could be enhanced to support JSON serialization
            String::new()
        }
    }
}

/// Interpolate template references in a string.
///
/// # Arguments
///
/// * `template` - The template string containing `%{field}` references
/// * `field_values` - Map of field names to their string values
/// * `vec_delimiter` - Delimiter for Vec values
///
/// # Returns
///
/// The interpolated string with all `%{field}` references replaced.
///
/// ```
/// use feuilletage::interpolate_template;
/// use std::collections::HashMap;
///
/// let values = HashMap::from([
///     ("host".to_string(), "localhost".to_string()),
///     ("port".to_string(), "8080".to_string()),
/// ]);
/// let result = interpolate_template("%{host}:%{port}", &values, ",").unwrap();
/// assert_eq!(result, "localhost:8080");
/// ```
#[cfg(feature = "std")]
pub fn interpolate_template(
    template: &str,
    field_values: &HashMap<String, String>,
    _vec_delimiter: &str,
) -> Result<String, TemplateError> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                if next == '%' {
                    // Escaped %%{ -> %{
                    chars.next();
                    if let Some(&after) = chars.peek() {
                        if after == '{' {
                            // %%{ -> %{
                            result.push('%');
                            result.push('{');
                            chars.next();
                            continue;
                        }
                    }
                    // Just %% -> %%
                    result.push('%');
                    result.push('%');
                } else if next == '{' {
                    // Start of field reference
                    chars.next(); // consume '{'
                    let mut field_name = String::new();

                    while let Some(&inner) = chars.peek() {
                        if inner == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        field_name.push(inner);
                        chars.next();
                    }

                    if let Some(value) = field_values.get(&field_name) {
                        result.push_str(value);
                    } else {
                        // Field not found - this is an error in strict mode
                        // For now, we'll leave it as-is
                        result.push_str("%{");
                        result.push_str(&field_name);
                        result.push('}');
                    }
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

#[cfg(not(feature = "std"))]
pub fn interpolate_template(
    template: &str,
    field_values: &BTreeMap<String, String>,
    _vec_delimiter: &str,
) -> Result<String, TemplateError> {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                if next == '%' {
                    // Escaped %%{ -> %{
                    chars.next();
                    if let Some(&after) = chars.peek() {
                        if after == '{' {
                            // %%{ -> %{
                            result.push('%');
                            result.push('{');
                            chars.next();
                            continue;
                        }
                    }
                    // Just %% -> %%
                    result.push('%');
                    result.push('%');
                } else if next == '{' {
                    // Start of field reference
                    chars.next(); // consume '{'
                    let mut field_name = String::new();

                    while let Some(&inner) = chars.peek() {
                        if inner == '}' {
                            chars.next(); // consume '}'
                            break;
                        }
                        field_name.push(inner);
                        chars.next();
                    }

                    if let Some(value) = field_values.get(&field_name) {
                        result.push_str(value);
                    } else {
                        // Field not found - leave as-is
                        result.push_str("%{");
                        result.push_str(&field_name);
                        result.push('}');
                    }
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    Ok(result)
}

/// Build a dependency graph from template fields and their references.
///
/// # Arguments
///
/// * `template_fields` - Map of field name to list of fields it references
///
/// # Returns
///
/// A topologically sorted list of field names (dependencies first).
///
/// ```
/// use feuilletage::topological_sort;
/// use std::collections::{HashMap, HashSet};
///
/// let dependencies = HashMap::from([
///     ("url".to_string(), vec!["host".to_string()]),
/// ]);
/// let fields = HashSet::from(["host".to_string(), "url".to_string()]);
/// let order = topological_sort(&dependencies, &fields).unwrap();
/// assert!(order.iter().position(|field| field == "host")
///     < order.iter().position(|field| field == "url"));
/// ```
#[cfg(feature = "std")]
pub fn topological_sort(
    template_fields: &HashMap<String, Vec<String>>,
    all_fields: &HashSet<String>,
) -> Result<Vec<String>, TemplateError> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize all fields
    for field in all_fields {
        in_degree.insert(field.clone(), 0);
        graph.insert(field.clone(), Vec::new());
    }

    // Build the graph
    for (field, refs) in template_fields {
        for ref_field in refs {
            if !all_fields.contains(ref_field) {
                return Err(TemplateError::MissingField {
                    template_field: field.clone(),
                    referenced_field: ref_field.clone(),
                });
            }
            graph.get_mut(ref_field).unwrap().push(field.clone());
            *in_degree.get_mut(field).unwrap() += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(f, _)| f.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(field) = queue.pop() {
        result.push(field.clone());

        for dependent in graph.get(&field).unwrap_or(&Vec::new()) {
            let deg = in_degree.get_mut(dependent).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push(dependent.clone());
            }
        }
    }

    if result.len() != all_fields.len() {
        // Find the cycle
        let remaining: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(f, _)| f.clone())
            .collect();
        return Err(TemplateError::CircularDependency { cycle: remaining });
    }

    Ok(result)
}

#[cfg(not(feature = "std"))]
pub fn topological_sort(
    template_fields: &BTreeMap<String, Vec<String>>,
    all_fields: &BTreeSet<String>,
) -> Result<Vec<String>, TemplateError> {
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Initialize all fields
    for field in all_fields {
        in_degree.insert(field.clone(), 0);
        graph.insert(field.clone(), Vec::new());
    }

    // Build the graph
    for (field, refs) in template_fields {
        for ref_field in refs {
            if !all_fields.contains(ref_field) {
                return Err(TemplateError::MissingField {
                    template_field: field.clone(),
                    referenced_field: ref_field.clone(),
                });
            }
            graph.get_mut(ref_field).unwrap().push(field.clone());
            *in_degree.get_mut(field).unwrap() += 1;
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(f, _)| f.clone())
        .collect();

    let mut result = Vec::new();

    while let Some(field) = queue.pop() {
        result.push(field.clone());

        for dependent in graph.get(&field).unwrap_or(&Vec::new()) {
            let deg = in_degree.get_mut(dependent).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push(dependent.clone());
            }
        }
    }

    if result.len() != all_fields.len() {
        // Find the cycle
        let remaining: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg > 0)
            .map(|(f, _)| f.clone())
            .collect();
        return Err(TemplateError::CircularDependency { cycle: remaining });
    }

    Ok(result)
}

// Unit tests have been moved to feuilletage/tests/unit/template_test.rs

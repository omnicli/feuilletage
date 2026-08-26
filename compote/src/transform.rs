//! Path-specific transformation module.
//!
//! This module provides functionality for transforming configuration values
//! based on their path in the configuration tree. Transformations can be
//! registered for exact paths or patterns.
//!
//! # Built-in Transform Functions
//!
//! - [`crate::transform::expand_env_vars`]: Expand `${VAR}` environment variables in strings (std only)
//! - [`crate::transform::to_uppercase`]: Convert strings to uppercase
//! - [`crate::transform::to_lowercase`]: Convert strings to lowercase
//! - [`crate::transform::trim`]: Remove leading/trailing whitespace
//! - [`crate::transform::relative_path`]: Resolve relative paths against the config file location (std only)
//! - [`crate::transform::parse_duration`]: Parse duration strings like "5m", "2h" into seconds
//! - [`crate::transform::parse_duration_ms`]: Parse duration strings like "500ms", "5s" into milliseconds
//!
//! # Duration Parsing Functions
//!
//! The module provides comprehensive duration parsing with multiple output formats:
//!
//! - [`crate::transform::parse_duration_u64`]: Parse to any unit as u64 (truncated)
//! - [`crate::transform::parse_duration_f64`]: Parse to any unit as f64 (precise)
//! - [`crate::transform::parse_duration_to_secs`] / [`crate::transform::parse_duration_to_secs_u64`]: Parse to seconds (u64)
//! - [`crate::transform::parse_duration_to_secs_f64`]: Parse to seconds (f64, full precision)
//! - [`crate::transform::parse_duration_to_ms_u64`]: Parse to milliseconds (u64)
//! - [`crate::transform::parse_duration_to_ms_f64`]: Parse to milliseconds (f64, full precision)
//! - [`crate::transform::parse_duration_to_nanos`]: Parse to nanoseconds (f64, maximum precision)
//! - [`crate::transform::unit_to_nanos`]: Convert unit string to nanoseconds multiplier
//!
//! Supported units: `ns`, `us`/`µs`/`μs`, `ms`, `s`, `m`, `h`, `d`, `w`
//!
//! Combined formats are supported: "1h30m500ms", "100us", "1s500ms250us"
//!
//! # Using with the Derive Macro
//!
//! Transform functions can be applied to fields using the `transform` attribute:
//!
//! ```
//! # #[cfg(feature = "json")] {
//! use compote::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, compote::Config)]
//! struct MyConfig {
//!     #[compote(transform = "to_uppercase")]
//!     name: String,
//!
//!     #[compote(duration)]  // Default: seconds
//!     timeout_secs: u64,
//!
//!     #[compote(duration(ms))]  // Specify unit
//!     timeout_ms: u64,
//!
//!     #[compote(duration(ns))]  // Nanoseconds
//!     latency_ns: u64,
//!
//!     #[compote(duration)]  // Float field preserves precision
//!     fractional_secs: f64,
//! }
//!
//! let mut config = Config::default();
//! config.load_json(
//!     r#"{"name": "hello", "timeout_secs": "5m", "timeout_ms": "500ms", "latency_ns": "1ms", "fractional_secs": "1s500ms"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let my: MyConfig = config.deserialize().unwrap();
//! assert_eq!(my.name, "HELLO"); // transform = "to_uppercase"
//! assert_eq!(my.timeout_secs, 300); // 5 minutes = 300 seconds
//! assert_eq!(my.timeout_ms, 500); // 500ms
//! assert_eq!(my.latency_ns, 1_000_000); // 1ms = 1,000,000 ns
//! assert!((my.fractional_secs - 1.5).abs() < 0.001); // 1s500ms = 1.5s
//! # }
//! ```

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec::Vec};

use hashbrown::HashMap;

use crate::{
    context::{Context, Level, LevelType, Source, SourceType},
    error::Error,
    value::ContextValue,
};

/// Type for transformation functions.
///
/// A transform function takes a mutable reference to a [`ContextValue`] and its
/// context, and can modify the value in place.
///
/// The type parameters `S` and `L` match the generic parameters of the Config
/// being transformed.
pub type TransformFn<S, L> = fn(&mut ContextValue<S, L>, &Context<S, L>) -> Result<(), Error>;

/// Registry for path-specific transformations.
///
/// ```
/// use compote::{Context, ContextValue, Level, Source};
/// use compote::transform::{trim, TransformRegistry};
///
/// let context = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("  compote  ", context.clone());
/// let mut transforms = TransformRegistry::new();
/// transforms.register_exact("name", trim);
/// transforms.apply("name", &mut value, &context).unwrap();
///
/// assert_eq!(value.as_str(), Some("compote"));
/// ```
pub struct TransformRegistry<S: SourceType = Source, L: LevelType = Level> {
    /// Exact path matches
    exact_transforms: HashMap<String, Vec<TransformFn<S, L>>>,
    /// Glob pattern matches (simplified - just suffix matching for now)
    pattern_transforms: Vec<(String, TransformFn<S, L>)>,
}

impl<S: SourceType, L: LevelType> Default for TransformRegistry<S, L> {
    fn default() -> Self {
        Self {
            exact_transforms: HashMap::new(),
            pattern_transforms: Vec::new(),
        }
    }
}

impl<S: SourceType, L: LevelType> core::fmt::Debug for TransformRegistry<S, L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TransformRegistry")
            .field(
                "exact_transforms",
                &format!("{} paths", self.exact_transforms.len()),
            )
            .field(
                "pattern_transforms",
                &format!("{} patterns", self.pattern_transforms.len()),
            )
            .finish()
    }
}

impl<S: SourceType, L: LevelType> TransformRegistry<S, L> {
    /// Construct an empty `TransformRegistry` with no registered transforms.
    ///
    /// Equivalent to [`TransformRegistry::default`]. Add transforms via
    /// [`Self::register_exact`] or [`Self::register_pattern`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a transformation for an exact path
    pub fn register_exact(&mut self, path: &str, transform: TransformFn<S, L>) {
        self.exact_transforms
            .entry(path.to_string())
            .or_default()
            .push(transform);
    }

    /// Register a transformation for a pattern (glob-like)
    /// Currently supports:
    /// - `*.field` - matches any path ending with `.field`
    /// - `**` - matches any path
    pub fn register_pattern(&mut self, pattern: &str, transform: TransformFn<S, L>) {
        self.pattern_transforms
            .push((pattern.to_string(), transform));
    }

    /// Apply transformations to a value at a specific path
    pub fn apply(
        &self,
        path: &str,
        value: &mut ContextValue<S, L>,
        context: &Context<S, L>,
    ) -> Result<(), Error> {
        // Apply exact matches
        if let Some(transforms) = self.exact_transforms.get(path) {
            for transform in transforms {
                transform(value, context)?;
            }
        }

        // Apply pattern matches
        for (pattern, transform) in &self.pattern_transforms {
            if self.matches_pattern(path, pattern) {
                transform(value, context)?;
            }
        }

        Ok(())
    }

    /// Check if a path matches a pattern
    fn matches_pattern(&self, path: &str, pattern: &str) -> bool {
        if pattern == "**" {
            return true;
        }

        if let Some(suffix) = pattern.strip_prefix("*.") {
            return path.ends_with(&format!(".{}", suffix)) || path == suffix;
        }

        if pattern.contains('*') {
            // More complex glob patterns could be implemented here
            // For now, only support simple patterns
            return false;
        }

        path == pattern
    }

    /// Apply transformations recursively to a ContextValue tree
    pub fn apply_to_tree(
        &self,
        value: &mut ContextValue<S, L>,
        current_path: &str,
    ) -> Result<(), Error> {
        // Apply transformations to current value
        let ctx = value.context().clone();
        self.apply(current_path, value, &ctx)?;

        // Recursively apply to children
        match value {
            ContextValue::Object(ref mut map, _) => {
                for (key, child_value) in map {
                    let child_path = if current_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", current_path, key)
                    };
                    self.apply_to_tree(child_value, &child_path)?;
                }
            }
            ContextValue::Array(ref mut arr, _) => {
                for (i, child_value) in arr.iter_mut().enumerate() {
                    let child_path = format!("{}.{}", current_path, i);
                    self.apply_to_tree(child_value, &child_path)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

// ============================================================================
// Built-in Transform Functions
// ============================================================================

/// Expands environment variables in string values.
///
/// Replaces `${VAR_NAME}` patterns with the corresponding environment variable value.
/// If the variable is not set, the pattern is left unchanged.
///
/// # Availability
///
/// This function is only available when the `std` feature is enabled.
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::expand_env_vars;
///
/// std::env::set_var("MY_APP_HOST", "localhost");
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("http://${MY_APP_HOST}:8080", ctx.clone());
///
/// expand_env_vars(&mut value, &ctx).unwrap();
///
/// if let ContextValue::String(s, _) = &value {
///     assert_eq!(s, "http://localhost:8080");
/// }
///
/// std::env::remove_var("MY_APP_HOST");
/// ```
#[cfg(feature = "std")]
pub fn expand_env_vars<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::String(ref mut s, _) = value {
        // Simple implementation: replace ${VAR} with env var
        let mut result = s.clone();
        let mut start = 0;

        while let Some(begin) = result[start..].find("${") {
            let begin = start + begin;
            if let Some(end) = result[begin..].find('}') {
                let end = begin + end;
                let var_name = &result[begin + 2..end];

                if let Ok(var_value) = std::env::var(var_name) {
                    result.replace_range(begin..=end, &var_value);
                    start = begin + var_value.len();
                } else {
                    start = end + 1;
                }
            } else {
                break;
            }
        }

        *s = result;
    }

    Ok(())
}

/// Converts string values to uppercase.
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::to_uppercase;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("hello world", ctx.clone());
///
/// to_uppercase(&mut value, &ctx).unwrap();
///
/// if let ContextValue::String(s, _) = &value {
///     assert_eq!(s, "HELLO WORLD");
/// }
/// ```
pub fn to_uppercase<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::String(ref mut s, _) = value {
        *s = s.to_uppercase();
    }
    Ok(())
}

/// Converts string values to lowercase.
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::to_lowercase;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("HELLO WORLD", ctx.clone());
///
/// to_lowercase(&mut value, &ctx).unwrap();
///
/// if let ContextValue::String(s, _) = &value {
///     assert_eq!(s, "hello world");
/// }
/// ```
pub fn to_lowercase<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::String(ref mut s, _) = value {
        *s = s.to_lowercase();
    }
    Ok(())
}

/// Trims leading and trailing whitespace from string values.
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::trim;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("  hello world  ", ctx.clone());
///
/// trim(&mut value, &ctx).unwrap();
///
/// if let ContextValue::String(s, _) = &value {
///     assert_eq!(s, "hello world");
/// }
/// ```
pub fn trim<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    if let ContextValue::String(ref mut s, _) = value {
        *s = s.trim().to_string();
    }
    Ok(())
}

/// Converts relative paths to absolute paths based on the config file location.
///
/// If the value is a string containing a path:
/// - Absolute paths are left unchanged
/// - Relative paths are resolved against the parent directory of the source file
/// - If the source is not a file, relative paths are left unchanged
///
/// # Availability
///
/// This function is only available when the `std` feature is enabled.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::relative_path;
///
/// // With a file source, relative paths are resolved
/// let ctx = Context::new(
///     Source::File(PathBuf::from("/etc/app/config.yaml")),
///     Level::User,
/// );
/// let mut value = ContextValue::string("data/cache", ctx.clone());
///
/// relative_path(&mut value, &ctx).unwrap();
///
/// if let ContextValue::String(s, _) = &value {
///     // Path is resolved relative to /etc/app/
///     assert!(s.contains("etc/app/data/cache") || s.contains("etc\\app\\data\\cache"));
/// }
///
/// // Absolute paths are unchanged
/// let mut abs_value = ContextValue::string("/var/log/app.log", ctx.clone());
/// relative_path(&mut abs_value, &ctx).unwrap();
/// if let ContextValue::String(s, _) = &abs_value {
///     assert_eq!(s, "/var/log/app.log");
/// }
/// ```
#[cfg(feature = "std")]
pub fn relative_path<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    context: &Context<S, L>,
) -> Result<(), Error> {
    use std::path::Path;

    if let ContextValue::String(ref mut s, _) = value {
        let path = Path::new(s.as_str());

        // Only process relative paths
        if !path.is_absolute() {
            // Get the source file directory using the file_path() method from Source/CustomSource
            if let Some(source_path) = context.source.file_path() {
                if let Some(parent) = source_path.parent() {
                    // Join the relative path with the source file's directory
                    let absolute = parent.join(path);

                    // Try to convert to string - this could fail if the path contains invalid UTF-8
                    let absolute_str = absolute.to_str().ok_or_else(|| Error::InvalidValue {
                        path: "<relative_path_transform>".to_string(),
                        message: format!(
                            "Path '{}' cannot be converted to a valid UTF-8 string",
                            absolute.display()
                        ),
                    })?;

                    *s = absolute_str.to_string();
                } else {
                    // Source file has no parent directory
                    return Err(Error::InvalidValue {
                        path: "<relative_path_transform>".to_string(),
                        message: format!(
                            "Cannot resolve relative path '{}': source file '{}' has no parent directory",
                            s,
                            source_path.display()
                        ),
                    });
                }
            }
            // If no file path available (e.g., Programmatic source), leave unchanged
        }
    }

    Ok(())
}

/// Normalize a file path by resolving `.` and `..` components without
/// touching the filesystem.
///
/// This transform resolves `.` (current directory) and `..` (parent directory)
/// components in path strings, producing a clean canonical form. Unlike
/// `std::fs::canonicalize`, this does not require the path to exist on disk.
///
/// Non-string values are left unchanged.
///
/// # Examples
///
/// ```
/// use compote::{Config, Context, Level, Source, ContextValue};
/// use compote::transform::normalize_path;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("foo/./bar/../baz".to_string(), ctx.clone());
/// normalize_path(&mut value, &ctx).unwrap();
/// if let ContextValue::String(s, _) = &value {
///     assert_eq!(s, "foo/baz");
/// }
/// ```
#[cfg(feature = "std")]
pub fn normalize_path<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    use std::path::{Component, PathBuf};

    if let ContextValue::String(ref mut s, _) = value {
        let path = PathBuf::from(s.as_str());
        let mut components = Vec::new();

        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    // Only pop normal components; preserve leading `..` and root/prefix
                    if matches!(components.last(), Some(Component::Normal(_))) {
                        components.pop();
                    } else {
                        components.push(component);
                    }
                }
                _ => components.push(component),
            }
        }

        let normalized: PathBuf = if components.is_empty() {
            PathBuf::from(".")
        } else {
            components.iter().collect()
        };

        *s = normalized.to_string_lossy().to_string();
    }

    Ok(())
}

// ============================================================================
// Duration Parsing Functions
// ============================================================================

/// Convert unit string to nanoseconds multiplier.
///
/// # Arguments
/// * `unit` - Target unit: "ns", "us", "ms", "s", "m", "h", "d", "w"
///
/// # Returns
/// The number of nanoseconds in one unit.
///
/// # Examples
///
/// ```
/// use compote::transform::unit_to_nanos;
///
/// assert!((unit_to_nanos("s").unwrap() - 1_000_000_000.0).abs() < f64::EPSILON);
/// assert!((unit_to_nanos("ms").unwrap() - 1_000_000.0).abs() < f64::EPSILON);
/// assert!((unit_to_nanos("m").unwrap() - 60_000_000_000.0).abs() < f64::EPSILON);
/// ```
pub fn unit_to_nanos(unit: &str) -> Result<f64, String> {
    match unit {
        "ns" => Ok(1.0),
        "us" => Ok(1_000.0),
        "ms" => Ok(1_000_000.0),
        "s" => Ok(1_000_000_000.0),
        "m" => Ok(60.0 * 1_000_000_000.0),
        "h" => Ok(3600.0 * 1_000_000_000.0),
        "d" => Ok(86400.0 * 1_000_000_000.0),
        "w" => Ok(604800.0 * 1_000_000_000.0),
        _ => Err(format!(
            "Unknown duration unit: '{}'. Use ns/us/ms/s/m/h/d/w",
            unit
        )),
    }
}

/// Parses duration strings into nanoseconds (f64) for maximum precision.
///
/// This is the core function that all other duration parsing functions use.
///
/// # Supported Units
///
/// - `ns` - nanoseconds (e.g., "100ns")
/// - `us`, `µs` (U+00B5), `μs` (U+03BC) - microseconds (e.g., "100us", "100µs")
/// - `ms` - milliseconds (e.g., "500ms")
/// - `s` - seconds (e.g., "30s")
/// - `m` - minutes (e.g., "5m")
/// - `h` - hours (e.g., "2h")
/// - `d` - days (e.g., "1d")
/// - `w` - weeks (e.g., "1w")
///
/// Combined formats are supported: "1h30m500ms", "100us", "1s500ms250us"
///
/// Plain numbers without units are treated as seconds.
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_nanos;
///
/// assert!((parse_duration_to_nanos("1s").unwrap() - 1_000_000_000.0).abs() < f64::EPSILON);
/// assert!((parse_duration_to_nanos("500ms").unwrap() - 500_000_000.0).abs() < f64::EPSILON);
/// assert!((parse_duration_to_nanos("1m").unwrap() - 60_000_000_000.0).abs() < f64::EPSILON);
/// ```
pub fn parse_duration_to_nanos(s: &str) -> Result<f64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    // Nanoseconds per unit
    const NS_PER_NS: f64 = 1.0;
    const NS_PER_US: f64 = 1_000.0;
    const NS_PER_MS: f64 = 1_000_000.0;
    const NS_PER_S: f64 = 1_000_000_000.0;
    const NS_PER_M: f64 = 60.0 * NS_PER_S;
    const NS_PER_H: f64 = 60.0 * NS_PER_M;
    const NS_PER_D: f64 = 24.0 * NS_PER_H;
    const NS_PER_W: f64 = 7.0 * NS_PER_D;

    let mut total_nanos: f64 = 0.0;
    let mut current_num = String::new();
    let mut chars = s.chars().peekable();
    let mut found_any_unit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' {
            current_num.push(c);
        } else if c.is_alphabetic() || c == '\u{00B5}' || c == '\u{03BC}' {
            // Handle both ASCII letters and Unicode mu characters
            if current_num.is_empty() {
                return Err(format!(
                    "Invalid duration format: missing number before '{}'",
                    c
                ));
            }
            let num: f64 = current_num
                .parse()
                .map_err(|_| format!("Invalid number in duration: {}", current_num))?;
            current_num.clear();

            // Determine the unit by peeking ahead for multi-character suffixes
            let multiplier = match c {
                'n' => {
                    // ns - nanoseconds
                    if chars.peek() == Some(&'s') {
                        chars.next();
                        NS_PER_NS
                    } else {
                        return Err(
                            "Unknown duration unit starting with 'n'. Did you mean 'ns'?"
                                .to_string(),
                        );
                    }
                }
                'u' => {
                    // us - microseconds (ASCII)
                    if chars.peek() == Some(&'s') {
                        chars.next();
                        NS_PER_US
                    } else {
                        return Err(
                            "Unknown duration unit starting with 'u'. Did you mean 'us'?"
                                .to_string(),
                        );
                    }
                }
                '\u{00B5}' | '\u{03BC}' => {
                    // µs or μs - microseconds (Unicode micro sign or Greek mu)
                    if chars.peek() == Some(&'s') {
                        chars.next();
                        NS_PER_US
                    } else {
                        return Err(
                            "Unknown duration unit. Did you mean 'µs' (microseconds)?".to_string()
                        );
                    }
                }
                'm' => {
                    // Could be 'ms' (milliseconds) or 'm' (minutes)
                    if chars.peek() == Some(&'s') {
                        chars.next();
                        NS_PER_MS
                    } else {
                        NS_PER_M
                    }
                }
                's' => NS_PER_S,
                'h' => NS_PER_H,
                'd' => NS_PER_D,
                'w' => NS_PER_W,
                _ => {
                    return Err(format!(
                        "Unknown duration unit: '{}'. Use ns/us/µs/ms/s/m/h/d/w",
                        c
                    ))
                }
            };
            total_nanos += num * multiplier;
            found_any_unit = true;
        } else if !c.is_whitespace() {
            return Err(format!("Invalid character in duration: '{}'", c));
        }
    }

    // If there's a trailing number without unit, treat as seconds
    if !current_num.is_empty() {
        let num: f64 = current_num
            .parse()
            .map_err(|_| format!("Invalid number in duration: {}", current_num))?;
        total_nanos += num * NS_PER_S;
    }

    // Allow "0" without any unit as a valid duration
    if total_nanos == 0.0 && !found_any_unit {
        // Check if the input was a valid zero (e.g., "0", "0.0")
        let trimmed = s.trim();
        if trimmed.parse::<f64>() == Ok(0.0) {
            return Ok(0.0);
        }
        // If we didn't find any units and the total is 0, but it wasn't a valid zero number,
        // it might be an invalid format - but we already handled this above with trailing number
    }

    Ok(total_nanos)
}

/// Parses duration strings into seconds as u64.
///
/// This is the primary public function for duration parsing, returning seconds as an integer.
/// Fractional seconds are truncated (rounded down).
///
/// # Supported Units
///
/// - `ns` - nanoseconds (e.g., "1000000000ns" = 1 second)
/// - `us`, `µs`, `μs` - microseconds (e.g., "1000000us" = 1 second)
/// - `ms` - milliseconds (e.g., "1000ms" = 1 second)
/// - `s` - seconds (e.g., "30s" = 30)
/// - `m` - minutes (e.g., "5m" = 300)
/// - `h` - hours (e.g., "2h" = 7200)
/// - `d` - days (e.g., "1d" = 86400)
/// - `w` - weeks (e.g., "1w" = 604800)
///
/// Combined formats are supported: "1h30m", "2d12h", "1s500ms"
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_secs_u64;
///
/// assert_eq!(parse_duration_to_secs_u64("30s").unwrap(), 30);
/// assert_eq!(parse_duration_to_secs_u64("5m").unwrap(), 300);
/// assert_eq!(parse_duration_to_secs_u64("2h").unwrap(), 7200);
/// assert_eq!(parse_duration_to_secs_u64("1d").unwrap(), 86400);
/// assert_eq!(parse_duration_to_secs_u64("1h30m").unwrap(), 5400);
/// assert_eq!(parse_duration_to_secs_u64("100").unwrap(), 100); // Plain number = seconds
/// assert_eq!(parse_duration_to_secs_u64("500ms").unwrap(), 0); // Truncated to 0 seconds
/// assert_eq!(parse_duration_to_secs_u64("1500ms").unwrap(), 1); // 1.5s truncated to 1
/// assert_eq!(parse_duration_to_secs_u64("100us").unwrap(), 0); // Microseconds truncated to 0
/// ```
pub fn parse_duration_to_secs_u64(s: &str) -> Result<u64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    // Convert nanoseconds to seconds, truncating fractional part
    Ok((nanos / 1_000_000_000.0) as u64)
}

/// Parses duration strings into seconds as f64.
///
/// This function provides full precision for sub-second durations.
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_secs_f64;
///
/// assert!((parse_duration_to_secs_f64("30s").unwrap() - 30.0).abs() < f64::EPSILON);
/// assert!((parse_duration_to_secs_f64("500ms").unwrap() - 0.5).abs() < f64::EPSILON);
/// assert!((parse_duration_to_secs_f64("1s500ms").unwrap() - 1.5).abs() < f64::EPSILON);
/// assert!((parse_duration_to_secs_f64("100us").unwrap() - 0.0001).abs() < 1e-9);
/// assert!((parse_duration_to_secs_f64("1000ns").unwrap() - 0.000001).abs() < 1e-12);
/// ```
pub fn parse_duration_to_secs_f64(s: &str) -> Result<f64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    Ok(nanos / 1_000_000_000.0)
}

/// Parses duration strings into milliseconds as u64.
///
/// This function is useful when you need millisecond precision but want an integer result.
/// Fractional milliseconds are truncated.
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_ms_u64;
///
/// assert_eq!(parse_duration_to_ms_u64("1s").unwrap(), 1000);
/// assert_eq!(parse_duration_to_ms_u64("500ms").unwrap(), 500);
/// assert_eq!(parse_duration_to_ms_u64("1s500ms").unwrap(), 1500);
/// assert_eq!(parse_duration_to_ms_u64("100us").unwrap(), 0); // Truncated to 0ms
/// assert_eq!(parse_duration_to_ms_u64("1500us").unwrap(), 1); // 1.5ms truncated to 1
/// assert_eq!(parse_duration_to_ms_u64("5m").unwrap(), 300000);
/// ```
pub fn parse_duration_to_ms_u64(s: &str) -> Result<u64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    // Convert nanoseconds to milliseconds, truncating fractional part
    Ok((nanos / 1_000_000.0) as u64)
}

/// Parses duration strings into milliseconds as f64.
///
/// This function provides full precision for sub-millisecond durations.
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_ms_f64;
///
/// assert!((parse_duration_to_ms_f64("1s").unwrap() - 1000.0).abs() < f64::EPSILON);
/// assert!((parse_duration_to_ms_f64("500ms").unwrap() - 500.0).abs() < f64::EPSILON);
/// assert!((parse_duration_to_ms_f64("100us").unwrap() - 0.1).abs() < 1e-9);
/// assert!((parse_duration_to_ms_f64("1000ns").unwrap() - 0.001).abs() < 1e-9);
/// ```
pub fn parse_duration_to_ms_f64(s: &str) -> Result<f64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    Ok(nanos / 1_000_000.0)
}

/// Parses duration strings into seconds (backward compatibility alias).
///
/// This is an alias for [`parse_duration_to_secs_u64`] for backward compatibility.
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_to_secs;
///
/// assert_eq!(parse_duration_to_secs("30s").unwrap(), 30);
/// assert_eq!(parse_duration_to_secs("5m").unwrap(), 300);
/// assert_eq!(parse_duration_to_secs("2h").unwrap(), 7200);
/// assert_eq!(parse_duration_to_secs("1d").unwrap(), 86400);
/// assert_eq!(parse_duration_to_secs("1h30m").unwrap(), 5400);
/// assert_eq!(parse_duration_to_secs("100").unwrap(), 100); // Plain number = seconds
/// assert_eq!(parse_duration_to_secs("500ms").unwrap(), 0); // Milliseconds truncated to seconds
/// assert_eq!(parse_duration_to_secs("1500ms").unwrap(), 1); // 1500ms = 1.5s, truncated to 1
/// ```
#[inline]
pub fn parse_duration_to_secs(s: &str) -> Result<u64, String> {
    parse_duration_to_secs_u64(s)
}

/// Parse duration string to the specified unit, returning u64 (truncated).
///
/// This function parses a duration string and converts it to the specified target unit.
/// Fractional values are truncated (rounded down).
///
/// # Arguments
/// * `s` - Duration string like "1h30m", "500ms", "100us"
/// * `unit` - Target unit: "ns", "us", "ms", "s", "m", "h", "d", "w"
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_u64;
///
/// // Parse to seconds (default)
/// assert_eq!(parse_duration_u64("1h30m", "s").unwrap(), 5400);
///
/// // Parse to milliseconds
/// assert_eq!(parse_duration_u64("5s", "ms").unwrap(), 5000);
/// assert_eq!(parse_duration_u64("500ms", "ms").unwrap(), 500);
///
/// // Parse to minutes
/// assert_eq!(parse_duration_u64("2h", "m").unwrap(), 120);
///
/// // Parse to nanoseconds
/// assert_eq!(parse_duration_u64("1ms", "ns").unwrap(), 1_000_000);
///
/// // Fractional values are truncated
/// assert_eq!(parse_duration_u64("500ms", "s").unwrap(), 0);
/// assert_eq!(parse_duration_u64("1500ms", "s").unwrap(), 1);
/// ```
pub fn parse_duration_u64(s: &str, unit: &str) -> Result<u64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    let divisor = unit_to_nanos(unit)?;
    Ok((nanos / divisor) as u64)
}

/// Parse duration string to the specified unit, returning f64 (precise).
///
/// This function parses a duration string and converts it to the specified target unit.
/// Unlike `parse_duration_u64`, this preserves fractional values.
///
/// # Arguments
/// * `s` - Duration string like "1h30m", "500ms", "100us"
/// * `unit` - Target unit: "ns", "us", "ms", "s", "m", "h", "d", "w"
///
/// # Examples
///
/// ```
/// use compote::transform::parse_duration_f64;
///
/// // Parse to seconds with fractional part
/// assert!((parse_duration_f64("1s500ms", "s").unwrap() - 1.5).abs() < f64::EPSILON);
/// assert!((parse_duration_f64("500ms", "s").unwrap() - 0.5).abs() < f64::EPSILON);
///
/// // Parse to milliseconds
/// assert!((parse_duration_f64("1s", "ms").unwrap() - 1000.0).abs() < f64::EPSILON);
/// assert!((parse_duration_f64("100us", "ms").unwrap() - 0.1).abs() < 1e-9);
///
/// // Parse to hours
/// assert!((parse_duration_f64("90m", "h").unwrap() - 1.5).abs() < f64::EPSILON);
/// ```
pub fn parse_duration_f64(s: &str, unit: &str) -> Result<f64, String> {
    let nanos = parse_duration_to_nanos(s)?;
    let divisor = unit_to_nanos(unit)?;
    Ok(nanos / divisor)
}

/// Transform function that parses duration strings into integer seconds.
///
/// This is the transform function used by the `#[compote(duration)]` attribute.
/// It converts a string value like "5m" into an integer value (300).
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::parse_duration;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("5m", ctx.clone());
///
/// parse_duration(&mut value, &ctx).unwrap();
///
/// // Value is now an integer (300 seconds)
/// assert!(matches!(value, ContextValue::Int(300, _)));
/// ```
pub fn parse_duration<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    match value {
        ContextValue::String(ref s, ref ctx) => {
            match parse_duration_to_secs(s) {
                Ok(secs) => {
                    // Replace the string with the parsed value as a number
                    *value = ContextValue::Int(secs as i64, ctx.clone());
                    Ok(())
                }
                Err(msg) => Err(Error::InvalidValue {
                    path: "<duration_transform>".to_string(),
                    message: msg,
                }),
            }
        }
        ContextValue::Int(_, _) => {
            // Already a number, keep as is
            Ok(())
        }
        _ => Err(Error::TypeMismatch {
            path: "<duration_transform>".to_string(),
            expected: "string or integer".to_string(),
            actual: value.type_name().to_string(),
        }),
    }
}

/// Transform function that parses duration strings into integer milliseconds.
///
/// This is the transform function used by the `#[compote(duration_ms)]` attribute.
/// It converts a string value like "5s" into an integer value (5000 milliseconds).
///
/// # Examples
///
/// ```
/// use compote::{ContextValue, Context, Level, Source};
/// use compote::transform::parse_duration_ms;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// let mut value = ContextValue::string("5s", ctx.clone());
///
/// parse_duration_ms(&mut value, &ctx).unwrap();
///
/// // Value is now an integer (5000 milliseconds)
/// assert!(matches!(value, ContextValue::Int(5000, _)));
///
/// // Milliseconds work directly
/// let mut value_ms = ContextValue::string("500ms", ctx.clone());
/// parse_duration_ms(&mut value_ms, &ctx).unwrap();
/// assert!(matches!(value_ms, ContextValue::Int(500, _)));
/// ```
pub fn parse_duration_ms<S: SourceType, L: LevelType>(
    value: &mut ContextValue<S, L>,
    _context: &Context<S, L>,
) -> Result<(), Error> {
    match value {
        ContextValue::String(ref s, ref ctx) => {
            match parse_duration_to_ms_u64(s) {
                Ok(ms) => {
                    // Replace the string with the parsed value as a number
                    *value = ContextValue::Int(ms as i64, ctx.clone());
                    Ok(())
                }
                Err(msg) => Err(Error::InvalidValue {
                    path: "<duration_ms_transform>".to_string(),
                    message: msg,
                }),
            }
        }
        ContextValue::Int(_, _) => {
            // Already a number, keep as is (assumed to be milliseconds)
            Ok(())
        }
        _ => Err(Error::TypeMismatch {
            path: "<duration_ms_transform>".to_string(),
            expected: "string or integer".to_string(),
            actual: value.type_name().to_string(),
        }),
    }
}

// Unit tests have been moved to compote/tests/unit/transform_test.rs

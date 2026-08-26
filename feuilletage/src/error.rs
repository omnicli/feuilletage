//! Error types for configuration processing.
//!
//! This module provides error types and an error tracking system for
//! configuration operations.
//!
//! # Overview
//!
//! The error system consists of two main components:
//!
//! 1. [`Error`]: The primary error type for configuration operations
//! 2. [`ErrorTracker`]: Accumulates errors during processing with path context
//!
//! # Error Types
//!
//! [`Error`] variants cover different failure scenarios:
//!
//! | Variant | Code | Description |
//! |---------|------|-------------|
//! | `MissingField` | C001 | Required field not provided |
//! | `TypeMismatch` | C101 | Expected one type, got another |
//! | `InvalidValue` | C102 | Value failed validation |
//! | `MergeConflict` | C105 | Conflicting values during merge |
//! | `ImmutableOverride` | C111 | Attempted to modify immutable value |
//! | `ParseError` | C120 | Failed to parse config format |
//! | `FormatNotSupported` | C121 | Format not supported or feature not enabled |
//! | `IoError` | C122 | I/O error reading file |
//! | `Custom` | variable | Custom error with arbitrary code |
//!
//! # Error Codes
//!
//! Error codes follow a categorization scheme:
//! - `C0xx` - Key/structural errors (MissingField)
//! - `C10x` - Value type/content errors (TypeMismatch, InvalidValue)
//! - `C11x` - Context/constraint errors (ImmutableOverride)
//! - `C12x` - File loading errors (ParseError, FormatNotSupported, IoError)
//!
//! # Error Output Format
//!
//! Errors are displayed in the format: `<file>:<lineno>:<code>:<message>`
//!
//! For example:
//! - `/path/to/config.yaml:0:C101:expected integer, got string`
//! - `server.port:0:C001:missing required field`
//!
//! # Error Tracking
//!
//! [`ErrorTracker`] provides path-aware error accumulation. Instead of failing
//! at the first error, it collects all errors with their full path context:
//!
//! ```
//! use feuilletage::ErrorTracker;
//!
//! let mut tracker = ErrorTracker::new();
//! tracker.push_field("server");
//! tracker.push_field("port");
//! // Now at path "server.port"
//! tracker.record_invalid_value("port must be between 1 and 65535");
//! tracker.pop();
//! tracker.pop();
//!
//! assert!(tracker.has_errors());
//! ```
//!
//! # Warnings
//!
//! [`ConfigWarning`] represents non-fatal issues that don't prevent loading
//! but should be addressed. Common warnings include deprecated field usage
//! and mutability constraint violations.

use core::fmt;

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, string::ToString, vec::Vec};

/// Errors that can occur during configuration processing.
///
/// Each error variant has an associated error code for easy identification:
/// - `C0xx` - Key/structural errors
/// - `C10x` - Value type/content errors
/// - `C11x` - Context/constraint errors
/// - `C12x` - File loading errors
///
/// # Output Format
///
/// Errors are displayed in the format: `<file>:<lineno>:<code>:<message>`
///
/// # Examples
///
/// ```
/// use feuilletage::{Error, Source};
///
/// // Type mismatch error
/// let error = Error::TypeMismatch {
///     path: "server.port".to_string(),
///     expected: "integer".to_string(),
///     actual: "string".to_string(),
/// };
/// assert_eq!(error.code(), "C101");
/// // Output format: server.port:0:C101:expected integer, got string
///
/// // Missing field error
/// let error = Error::MissingField {
///     path: "database.host".to_string(),
/// };
/// assert_eq!(error.code(), "C001");
/// // Output format: database.host:0:C001:missing required field
///
/// // Custom error with arbitrary code
/// let error = Error::Custom {
///     code: "C999".to_string(),
///     path: "custom.field".to_string(),
///     message: "custom error message".to_string(),
/// };
/// assert_eq!(error.code(), "C999");
/// ```
#[derive(Debug, Clone)]
pub enum Error {
    /// Attempted to override an immutable value (C111)
    ImmutableOverride {
        /// Dotted path of the value that was overridden.
        path: String,
        /// Identifier of the source that attempted the override.
        source: String,
    },

    /// Value type does not match expected type (C101)
    TypeMismatch {
        /// Dotted path of the offending value.
        path: String,
        /// Expected type name (e.g. `"string"`).
        expected: String,
        /// Actual type name received (e.g. `"integer"`).
        actual: String,
    },

    /// Value failed validation or is otherwise invalid (C102)
    InvalidValue {
        /// Dotted path of the offending value.
        path: String,
        /// Human-readable description of why the value is invalid.
        message: String,
    },

    /// Conflicting values during merge (C105)
    MergeConflict {
        /// Dotted path where the conflict occurred.
        path: String,
        /// Description of the conflict.
        message: String,
    },

    /// Required field was not provided (C001)
    MissingField {
        /// Dotted path of the missing field.
        path: String,
    },

    /// Failed to parse configuration (C120)
    ParseError {
        /// Source identifier (filename or label) being parsed.
        source: String,
        /// Underlying parser error message.
        message: String,
    },

    /// Format not supported or feature not enabled (C121)
    FormatNotSupported {
        /// Format name that was requested (e.g. `"toml"`).
        format: String,
        /// Hint about how to enable the format (typically a feature flag).
        message: String,
    },

    /// I/O error reading file (C122)
    IoError {
        /// Filesystem path that caused the error.
        path: String,
        /// OS error message.
        message: String,
    },

    // ========== Custom error for arbitrary error codes ==========
    /// Custom error with arbitrary code (variable)
    ///
    /// Allows using any error code string for extensibility.
    /// Use this for application-specific errors that don't fit existing variants.
    Custom {
        /// Application-defined error code string.
        code: String,
        /// Dotted path associated with the error (empty string if none).
        path: String,
        /// Human-readable error message.
        message: String,
    },
}

impl Error {
    /// Returns the error code for this error variant.
    ///
    /// Error codes follow a categorization scheme:
    /// - `C0xx` - Key/structural errors (MissingField)
    /// - `C10x` - Value type/content errors (TypeMismatch, InvalidValue)
    /// - `C11x` - Context/constraint errors (ImmutableOverride)
    /// - `C12x` - File loading errors (ParseError, FormatNotSupported, IoError)
    ///
    /// # Examples
    ///
    /// ```
    /// use feuilletage::Error;
    ///
    /// let error = Error::TypeMismatch {
    ///     path: "foo".to_string(),
    ///     expected: "string".to_string(),
    ///     actual: "integer".to_string(),
    /// };
    /// assert_eq!(error.code(), "C101");
    ///
    /// let error = Error::MissingField {
    ///     path: "bar".to_string(),
    /// };
    /// assert_eq!(error.code(), "C001");
    /// ```
    pub fn code(&self) -> &str {
        match self {
            Error::MissingField { .. } => "C001",
            Error::TypeMismatch { .. } => "C101",
            Error::InvalidValue { .. } => "C102",
            Error::MergeConflict { .. } => "C105",
            Error::ImmutableOverride { .. } => "C111",
            Error::ParseError { .. } => "C120",
            Error::FormatNotSupported { .. } => "C121",
            Error::IoError { .. } => "C122",
            // Custom error returns the stored code
            Error::Custom { code, .. } => code.as_str(),
        }
    }

    /// Returns the location identifier for this error.
    ///
    /// This is typically a file path if a config source is available,
    /// otherwise it's the configuration path where the error occurred.
    pub fn location(&self) -> String {
        match self {
            Error::MissingField { path } => path.clone(),
            Error::TypeMismatch { path, .. } => path.clone(),
            Error::InvalidValue { path, .. } => path.clone(),
            Error::MergeConflict { path, .. } => path.clone(),
            Error::ImmutableOverride { source, .. } => source.to_string(),
            Error::ParseError { source, .. } => source.to_string(),
            Error::FormatNotSupported { format, .. } => format.clone(),
            Error::IoError { path, .. } => path.clone(),
            // Custom error
            Error::Custom { path, .. } => path.clone(),
        }
    }
}

/// Display implementation for Error
///
/// Format: `<location>:0:<code>:<message>`
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingField { path } => {
                write!(f, "{}:0:C001:missing required field", path)
            }
            Error::TypeMismatch {
                path,
                expected,
                actual,
            } => {
                write!(f, "{}:0:C101:expected {}, got {}", path, expected, actual)
            }
            Error::InvalidValue { path, message } => {
                write!(f, "{}:0:C102:{}", path, message)
            }
            Error::MergeConflict { path, message } => {
                write!(f, "{}:0:C105:{}", path, message)
            }
            Error::ImmutableOverride { path, source } => {
                write!(
                    f,
                    "{}:0:C111:cannot override immutable value at '{}'",
                    source, path
                )
            }
            Error::ParseError { source, message } => {
                write!(f, "{}:0:C120:{}", source, message)
            }
            Error::FormatNotSupported { format, message } => {
                write!(f, "{}:0:C121:{}", format, message)
            }
            Error::IoError { path, message } => {
                write!(f, "{}:0:C122:{}", path, message)
            }
            // Custom error
            Error::Custom {
                code,
                path,
                message,
            } => {
                write!(f, "{}:0:{}:{}", path, code, message)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

/// A warning about a configuration issue that doesn't prevent loading.
///
/// Warnings are non-fatal issues that are recorded during configuration processing.
/// Unlike errors, warnings don't prevent the configuration from being loaded,
/// but they indicate potential problems that should be addressed.
#[derive(Debug, Clone)]
pub struct ConfigWarning {
    /// The path where the warning occurred
    pub path: String,
    /// A description of the warning
    pub message: String,
}

impl fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Warning at '{}': {}", self.path, self.message)
    }
}

/// Tracks errors during configuration processing with full path context.
///
/// `ErrorTracker` accumulates errors as configuration is processed, tracking
/// the path to each error. This allows collecting multiple errors in a single
/// pass rather than failing at the first error.
///
/// # Examples
///
/// ```
/// use feuilletage::ErrorTracker;
///
/// let mut tracker = ErrorTracker::new();
/// assert!(!tracker.has_errors());
///
/// tracker.push_field("server");
/// tracker.push_field("port");
/// assert_eq!(tracker.current_path(), "server.port");
///
/// tracker.record_invalid_value("port must be positive");
/// tracker.record_warning("using the fallback port");
/// assert!(tracker.has_errors());
/// assert!(tracker.has_warnings());
/// assert_eq!(tracker.errors().len(), 1);
/// assert_eq!(tracker.warnings().len(), 1);
///
/// tracker.pop();
/// tracker.pop();
/// tracker.clear();
/// assert!(!tracker.has_errors());
/// assert!(!tracker.has_warnings());
/// ```
#[derive(Debug, Clone, Default)]
pub struct ErrorTracker {
    errors: Vec<Error>,
    warnings: Vec<ConfigWarning>,
    current_path: Vec<PathSegment>,
}

#[derive(Debug, Clone)]
enum PathSegment {
    Field(String),
    Index(usize),
}

impl ErrorTracker {
    /// Construct an empty `ErrorTracker` with no recorded errors or warnings
    /// and a root path of `""`.
    ///
    /// Equivalent to [`ErrorTracker::default`].
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            current_path: Vec::new(),
        }
    }

    /// Create an empty child tracker at the current path.
    ///
    /// Child trackers isolate diagnostics produced by speculative work. If the
    /// work succeeds, pass the child to [`ErrorTracker::commit_child`]; dropping
    /// it discards the diagnostics without changing this tracker.
    pub fn child(&self) -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            current_path: self.current_path.clone(),
        }
    }

    /// Append the errors and warnings recorded by another tracker.
    ///
    /// This is typically used with a tracker returned by [`ErrorTracker::child`].
    /// The current path is not changed.
    pub fn commit_child(&mut self, child: Self) {
        self.errors.extend(child.errors);
        self.warnings.extend(child.warnings);
    }

    /// Enter a field in the configuration path
    pub fn push_field(&mut self, field: &str) {
        self.current_path
            .push(PathSegment::Field(field.to_string()));
    }

    /// Enter an array index in the configuration path
    pub fn push_index(&mut self, index: usize) {
        self.current_path.push(PathSegment::Index(index));
    }

    /// Exit the current path segment
    pub fn pop(&mut self) {
        self.current_path.pop();
    }

    /// Get the current path as a string (e.g., "a.b.2.c")
    pub fn current_path(&self) -> String {
        if self.current_path.is_empty() {
            return "<root>".to_string();
        }

        let mut path = String::new();
        for (i, segment) in self.current_path.iter().enumerate() {
            if i > 0 {
                path.push('.');
            }
            match segment {
                PathSegment::Field(name) => path.push_str(name),
                PathSegment::Index(idx) => path.push_str(&idx.to_string()),
            }
        }
        path
    }

    /// Record an error at the current path
    pub fn record(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Record an immutable override error at the current path
    pub fn record_immutable_override(&mut self, source: impl ToString) {
        let path = self.current_path();
        self.record(Error::ImmutableOverride {
            path,
            source: source.to_string(),
        });
    }

    /// Record a type mismatch error at the current path
    pub fn record_type_mismatch(&mut self, expected: &str, actual: &str) {
        let path = self.current_path();
        self.record(Error::TypeMismatch {
            path,
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }

    /// Record an invalid value error at the current path
    pub fn record_invalid_value(&mut self, message: impl Into<String>) {
        let path = self.current_path();
        self.record(Error::InvalidValue {
            path,
            message: message.into(),
        });
    }

    /// Record a merge conflict error at the current path
    pub fn record_merge_conflict(&mut self, message: impl Into<String>) {
        let path = self.current_path();
        self.record(Error::MergeConflict {
            path,
            message: message.into(),
        });
    }

    /// Record a warning at the current path
    pub fn record_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(ConfigWarning {
            path: self.current_path(),
            message: message.into(),
        });
    }

    /// Record a warning at an explicit configuration path.
    ///
    /// Use this when copying a warning that already carries path information.
    /// Unlike [`record_warning`](Self::record_warning), this method does not use
    /// or modify the tracker's current path.
    pub fn record_warning_at(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.warnings.push(ConfigWarning {
            path: path.into(),
            message: message.into(),
        });
    }

    /// Record a mutability constraint warning.
    ///
    /// This is used when a value from a config level is skipped because it's not
    /// allowed by the field's `mutable_by` constraint.
    ///
    /// # Arguments
    ///
    /// * `field` - The field name that was constrained
    /// * `source_level` - The level that tried to set the value
    /// * `allowed_levels` - The levels that are allowed to modify this field
    pub fn record_mutability_warning(
        &mut self,
        field: &str,
        source_level: &str,
        allowed_levels: &[&str],
    ) {
        let allowed_str = allowed_levels.join(", ");
        let message = format!(
            "value from '{}' level ignored (allowed by: [{}])",
            source_level, allowed_str
        );
        self.warnings.push(ConfigWarning {
            path: field.to_string(),
            message,
        });
    }

    /// Check if any errors have been recorded
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if any warnings have been recorded
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get all recorded errors
    pub fn errors(&self) -> &[Error] {
        &self.errors
    }

    /// Get all recorded warnings
    pub fn warnings(&self) -> &[ConfigWarning] {
        &self.warnings
    }

    /// Consume the tracker and return all errors
    pub fn into_errors(self) -> Vec<Error> {
        self.errors
    }

    /// Consume the tracker and return all warnings
    pub fn into_warnings(self) -> Vec<ConfigWarning> {
        self.warnings
    }

    /// Clear all errors and warnings
    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }

    /// Clear only errors (keeping warnings)
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Clear only warnings (keeping errors)
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }
}

impl fmt::Display for ErrorTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let has_errors = !self.errors.is_empty();
        let has_warnings = !self.warnings.is_empty();

        if !has_errors && !has_warnings {
            return write!(f, "No errors or warnings");
        }

        if has_errors {
            writeln!(f, "Configuration errors ({}):", self.errors.len())?;
            for (i, error) in self.errors.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, error)?;
            }
        }

        if has_warnings {
            if has_errors {
                writeln!(f)?;
            }
            writeln!(f, "Configuration warnings ({}):", self.warnings.len())?;
            for (i, warning) in self.warnings.iter().enumerate() {
                writeln!(f, "  {}. {}", i + 1, warning)?;
            }
        }

        Ok(())
    }
}

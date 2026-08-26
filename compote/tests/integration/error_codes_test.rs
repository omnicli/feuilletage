//! Tests for error codes in Error variants.
//!
//! Error codes follow this categorization:
//! - `C0xx` - Key/structural errors (MissingField)
//! - `C10x` - Value type/content errors (TypeMismatch, InvalidValue)
//! - `C11x` - Context/constraint errors (ImmutableOverride)
//! - `C12x` - File loading errors (ParseError, FormatNotSupported, IoError)
//!
//! Output format: `<file>:<lineno>:<code>:<message>`

use std::path::PathBuf;

use compote::{Config, Context, Error, Level, Source};

// ============================================================================
// Error code method tests
// ============================================================================

#[test]
fn test_type_mismatch_code() {
    let error = Error::TypeMismatch {
        path: "server.port".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
    };
    assert_eq!(error.code(), "C101");
}

#[test]
fn test_missing_field_code() {
    let error = Error::MissingField {
        path: "database.host".to_string(),
    };
    assert_eq!(error.code(), "C001");
}

#[test]
fn test_invalid_value_code() {
    let error = Error::InvalidValue {
        path: "server.port".to_string(),
        message: "port must be between 1 and 65535".to_string(),
    };
    assert_eq!(error.code(), "C102");
}

#[test]
fn test_merge_conflict_code() {
    let error = Error::MergeConflict {
        path: "config.value()".to_string(),
        message: "conflicting values from different sources".to_string(),
    };
    assert_eq!(error.code(), "C105");
}

#[test]
fn test_immutable_override_code() {
    let error = Error::ImmutableOverride {
        path: "readonly.setting".to_string(),
        source: Source::Programmatic.display_name(),
    };
    assert_eq!(error.code(), "C111");
}

#[test]
fn test_parse_error_code() {
    let error = Error::ParseError {
        source: Source::File(PathBuf::from("config.yaml")).display_name(),
        message: "invalid YAML syntax".to_string(),
    };
    assert_eq!(error.code(), "C120");
}

#[test]
fn test_format_not_supported_code() {
    let error = Error::FormatNotSupported {
        format: "xyz".to_string(),
        message: "XYZ format not supported".to_string(),
    };
    assert_eq!(error.code(), "C121");
}

// ============================================================================
// Display format tests - verify format is <location>:0:<code>:<message>
// ============================================================================

#[test]
fn test_type_mismatch_display_format() {
    let error = Error::TypeMismatch {
        path: "foo.bar".to_string(),
        expected: "string".to_string(),
        actual: "integer".to_string(),
    };
    let display = error.to_string();
    // Format: <path>:0:C101:expected <expected>, got <actual>
    assert_eq!(display, "foo.bar:0:C101:expected string, got integer");
}

#[test]
fn test_missing_field_display_format() {
    let error = Error::MissingField {
        path: "required.field".to_string(),
    };
    let display = error.to_string();
    // Format: <path>:0:C001:missing required field
    assert_eq!(display, "required.field:0:C001:missing required field");
}

#[test]
fn test_invalid_value_display_format() {
    let error = Error::InvalidValue {
        path: "port".to_string(),
        message: "must be positive".to_string(),
    };
    let display = error.to_string();
    // Format: <path>:0:C102:<message>
    assert_eq!(display, "port:0:C102:must be positive");
}

#[test]
fn test_merge_conflict_display_format() {
    let error = Error::MergeConflict {
        path: "setting".to_string(),
        message: "cannot merge".to_string(),
    };
    let display = error.to_string();
    // Format: <path>:0:C105:<message>
    assert_eq!(display, "setting:0:C105:cannot merge");
}

#[test]
fn test_immutable_override_display_format() {
    let error = Error::ImmutableOverride {
        path: "immutable.value()".to_string(),
        source: Source::Programmatic.display_name(),
    };
    let display = error.to_string();
    // Format: <source>:0:C111:cannot override immutable value at '<path>'
    assert_eq!(
        display,
        "programmatic:0:C111:cannot override immutable value at 'immutable.value()'"
    );
}

#[test]
fn test_immutable_override_with_file_source() {
    let error = Error::ImmutableOverride {
        path: "immutable.value()".to_string(),
        source: Source::File(PathBuf::from("/etc/config.yaml")).display_name(),
    };
    let display = error.to_string();
    // Format: <file_path>:0:C111:cannot override immutable value at '<path>'
    assert_eq!(
        display,
        "/etc/config.yaml:0:C111:cannot override immutable value at 'immutable.value()'"
    );
}

#[test]
fn test_parse_error_display_format() {
    let error = Error::ParseError {
        source: Source::Programmatic.display_name(),
        message: "unexpected token".to_string(),
    };
    let display = error.to_string();
    // Format: <source>:0:C120:<message>
    assert_eq!(display, "programmatic:0:C120:unexpected token");
}

#[test]
fn test_parse_error_with_file_source() {
    let error = Error::ParseError {
        source: Source::File(PathBuf::from("/path/to/config.yaml")).display_name(),
        message: "invalid YAML syntax".to_string(),
    };
    let display = error.to_string();
    // Format: <file_path>:0:C120:<message>
    assert_eq!(display, "/path/to/config.yaml:0:C120:invalid YAML syntax");
}

#[test]
fn test_format_not_supported_display_format() {
    let error = Error::FormatNotSupported {
        format: "xyz".to_string(),
        message: "XYZ format not supported".to_string(),
    };
    let display = error.to_string();
    // Format: <format>:0:C121:<message>
    assert_eq!(display, "xyz:0:C121:XYZ format not supported");
}

// ============================================================================
// Integration tests - error codes in actual config operations
// ============================================================================

#[test]
fn test_parse_error_code_from_invalid_json() {
    let mut config = Config::default();
    config.load_json(
        "{ invalid json }",
        Context::new(Source::Programmatic, Level::User),
    );

    assert!(config.has_errors());
    let errors = config.get_errors();
    assert!(!errors.is_empty());

    // Should be a parse error
    let error = &errors[0];
    assert_eq!(error.code(), "C120");
    assert!(error.to_string().contains(":0:C120:"));
}

#[test]
fn test_error_tracker_display_shows_codes() {
    use compote::ErrorTracker;

    let mut tracker = ErrorTracker::new();
    tracker.push_field("server");
    tracker.push_field("port");
    tracker.record_type_mismatch("integer", "string");
    tracker.pop();
    tracker.pop();

    tracker.push_field("database");
    tracker.record_invalid_value("connection string required");
    tracker.pop();

    let display = tracker.to_string();
    println!("ErrorTracker display:\n{}", display);

    // Verify both error codes appear in the tracker display
    assert!(
        display.contains(":0:C101:"),
        "Expected :0:C101: in tracker display: {}",
        display
    );
    assert!(
        display.contains(":0:C102:"),
        "Expected :0:C102: in tracker display: {}",
        display
    );
}

// ============================================================================
// Categorization tests - verify error codes follow the scheme
// ============================================================================

#[test]
fn test_error_code_categories() {
    // Config errors (type/content) start with C10x
    let type_error = Error::TypeMismatch {
        path: "".to_string(),
        expected: "".to_string(),
        actual: "".to_string(),
    };
    assert_eq!(type_error.code(), "C101", "TypeMismatch should be C101");

    // Missing field errors are C001 (structural)
    let missing_error = Error::MissingField {
        path: "".to_string(),
    };
    assert_eq!(missing_error.code(), "C001", "MissingField should be C001");

    // Validation errors are C102 (value content)
    let validation_error = Error::InvalidValue {
        path: "".to_string(),
        message: "".to_string(),
    };
    assert_eq!(
        validation_error.code(),
        "C102",
        "InvalidValue should be C102"
    );

    // Merge conflict errors are C105
    let merge_conflict = Error::MergeConflict {
        path: "".to_string(),
        message: "".to_string(),
    };
    assert_eq!(
        merge_conflict.code(),
        "C105",
        "MergeConflict should be C105"
    );

    // ImmutableOverride is C111 (context/constraint)
    let immutable_override = Error::ImmutableOverride {
        path: "".to_string(),
        source: Source::Programmatic.display_name(),
    };
    assert_eq!(
        immutable_override.code(),
        "C111",
        "ImmutableOverride should be C111"
    );

    // Parse errors are C120
    let parse_error = Error::ParseError {
        source: Source::Programmatic.display_name(),
        message: "".to_string(),
    };
    assert_eq!(parse_error.code(), "C120", "ParseError should be C120");

    // Format not supported errors are C121 (file loading)
    let format_error = Error::FormatNotSupported {
        format: "".to_string(),
        message: "".to_string(),
    };
    assert_eq!(
        format_error.code(),
        "C121",
        "FormatNotSupported should be C121"
    );

    // I/O errors are C122 (file loading)
    let io_error = Error::IoError {
        path: "".to_string(),
        message: "".to_string(),
    };
    assert_eq!(io_error.code(), "C122", "IoError should be C122");
}

// ============================================================================
// Location method tests
// ============================================================================

#[test]
fn test_error_location() {
    // Path-based errors use the path as location
    let type_error = Error::TypeMismatch {
        path: "server.port".to_string(),
        expected: "integer".to_string(),
        actual: "string".to_string(),
    };
    assert_eq!(type_error.location(), "server.port");

    // Source-based errors use the source as location
    let parse_error = Error::ParseError {
        source: Source::File(PathBuf::from("/etc/config.yaml")).display_name(),
        message: "invalid".to_string(),
    };
    assert_eq!(parse_error.location(), "/etc/config.yaml");

    // FormatNotSupported uses the format as location
    let format_error = Error::FormatNotSupported {
        format: "xyz".to_string(),
        message: "not supported".to_string(),
    };
    assert_eq!(format_error.location(), "xyz");
}

// ============================================================================
// Optional file loading tests - missing files should be silently skipped
// ============================================================================

#[test]
fn test_missing_file_silently_skipped() {
    let mut config = Config::default();
    // Loading a non-existent file should return false (silently skip)
    let loaded = config.load_file("/nonexistent/path/to/config.yaml", Level::User);
    assert!(!loaded, "Loading non-existent file should return false");
    assert!(
        !config.has_errors(),
        "No errors should be recorded for missing file"
    );
    assert!(
        config.loaded_files().is_empty(),
        "No files should be in loaded_files list"
    );
}

#[test]
fn test_free_loader_records_unsupported_extension() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a temp file with an unsupported extension
    let mut file = NamedTempFile::with_suffix(".xyz").unwrap();
    writeln!(file, "some content").unwrap();

    let error = compote::loader::load_file::<Source, Level>(file.path(), Level::User).unwrap_err();

    match error {
        Error::FormatNotSupported { format, .. } => assert_eq!(format, "xyz"),
        error => panic!("Expected FormatNotSupported error, got: {error:?}"),
    }
}

#[test]
fn test_loader_builder_missing_file_silently_skipped() {
    use compote::loader;

    // Loading a non-existent file should silently skip (builder is returned, no error)
    let loader = loader().load_file("/nonexistent/path/to/config.yaml", Level::User);

    assert!(
        loader.loaded_files().is_empty(),
        "No files should be in loaded_files list"
    );
    assert!(
        !loader.errors().has_errors(),
        "No errors should be recorded for missing file"
    );
}

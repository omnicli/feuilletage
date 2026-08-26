//! Tests for duration transform attributes.
//!
//! This file tests `#[feuilletage(duration)]` with various unit configurations.
//! Supported units: ns, us, ms, s (default), m, h, d, w

use feuilletage::{Config, Context, Format, Level, Source};
use feuilletage_macros::Config as DeriveConfig;

// ============================================================================
// Basic duration tests (default unit: seconds)
// ============================================================================

/// Test duration parsing failure with default
#[test]
fn test_duration_parsing_error_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "60")]
        timeout_secs: u64,
    }

    // "invalid_duration" cannot be parsed
    let config_str = r#"{"timeout_secs": "invalid_duration"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.timeout_secs, 60, "should use default");

    let errors = config.errors().errors();
    assert!(
        errors.iter().any(|e| {
            let msg = e.to_string().to_lowercase();
            msg.contains("timeout") || msg.contains("duration") || msg.contains("parse")
        }),
        "Expected duration parse error, got: {:?}",
        errors
    );
}

/// Test valid duration formats
#[test]
fn test_duration_valid_formats() {
    #[derive(DeriveConfig, Debug)]
    struct DurationFormatsConfig {
        #[feuilletage(duration, default = "0")]
        seconds_only: u64,

        #[feuilletage(duration, default = "0")]
        with_unit: u64,

        #[feuilletage(duration, default = "0")]
        minutes: u64,

        #[feuilletage(duration, default = "0")]
        hours: u64,
    }

    let config_str = r#"
seconds_only: "30"
with_unit: "45s"
minutes: "5m"
hours: "2h"
"#;

    let mut loader = feuilletage::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: DurationFormatsConfig = loader.deserialize().expect("Should succeed");

    assert_eq!(result.seconds_only, 30, "plain number should be seconds");
    assert_eq!(result.with_unit, 45, "45s should be 45 seconds");
    assert_eq!(result.minutes, 300, "5m should be 300 seconds");
    assert_eq!(result.hours, 7200, "2h should be 7200 seconds");

    assert!(
        !loader.errors().has_errors(),
        "Should not have errors for valid durations"
    );
}

/// Test duration with numeric value (no unit)
#[test]
fn test_duration_numeric_value() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "0")]
        timeout: u64,
    }

    // Numeric value without unit - passed through as-is (assumed to be in target unit)
    let config_str = r#"{"timeout": 120}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.timeout, 120,
        "numeric value should be passed through"
    );
    assert!(!config.errors().has_errors());
}

/// Test duration with days
#[test]
fn test_duration_days() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "0")]
        retention: u64,
    }

    let config_str = r#"{"retention": "7d"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.retention, 604800, "7d should be 604800 seconds");
    assert!(!config.errors().has_errors());
}

/// Test duration with milliseconds input (converts to seconds, truncates)
#[test]
fn test_duration_milliseconds_truncated() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "0")]
        poll_interval: u64,
    }

    let config_str = r#"{"poll_interval": "500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed");

    // Milliseconds get converted to seconds (truncated)
    assert_eq!(result.poll_interval, 0, "500ms truncates to 0 seconds");
    assert!(!config.errors().has_errors());
}

/// Test duration on required field fails on parse error
#[test]
fn test_duration_required_field_fails_on_parse_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredDurationConfig {
        #[feuilletage(duration)]
        timeout: u64,
    }

    let config_str = r#"{"timeout": "not-a-duration"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredDurationConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required duration field fails to parse"
    );
}

/// Test zero duration
#[test]
fn test_duration_zero() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "60")]
        delay: u64,
    }

    let config_str = r#"{"delay": "0s"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.delay, 0, "0s should be 0");
    assert!(!config.errors().has_errors());
}

// ============================================================================
// duration(ms) tests (milliseconds)
// ============================================================================

/// Test basic duration with ms unit
#[test]
fn test_duration_ms_basic() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        timeout_ms: u64,
    }

    let config_str = r#"{"timeout_ms": "5s"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.timeout_ms, 5000, "5s should be 5000 milliseconds");
    assert!(!config.errors().has_errors());
}

/// Test duration_ms with milliseconds input
#[test]
fn test_duration_ms_with_ms_input() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        poll_interval: u64,
    }

    let config_str = r#"{"poll_interval": "500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.poll_interval, 500,
        "500ms should be 500 milliseconds"
    );
    assert!(!config.errors().has_errors());
}

/// Test duration_ms with combined duration
#[test]
fn test_duration_ms_combined() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        delay: u64,
    }

    let config_str = r#"{"delay": "1s500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.delay, 1500, "1s500ms should be 1500 milliseconds");
    assert!(!config.errors().has_errors());
}

/// Test duration_ms with various units
#[test]
fn test_duration_ms_various_units() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        value: u64,
    }

    // Test minutes
    let config_str = r#"{"value": "1m"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: DurationMsConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, 60000, "1m should be 60000 milliseconds");

    // Test hours
    let config_str = r#"{"value": "1h"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: DurationMsConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, 3600000, "1h should be 3600000 milliseconds");
}

/// Test duration_ms with microseconds
#[test]
fn test_duration_ms_microseconds() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        latency: u64,
    }

    // 1000 microseconds = 1 millisecond
    let config_str = r#"{"latency": "1000us"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.latency, 1, "1000us should be 1 millisecond");
    assert!(!config.errors().has_errors());
}

/// Test duration_ms with nanoseconds
#[test]
fn test_duration_ms_nanoseconds() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        latency: u64,
    }

    // 1000000 nanoseconds = 1 millisecond
    let config_str = r#"{"latency": "1000000ns"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.latency, 1, "1000000ns should be 1 millisecond");
    assert!(!config.errors().has_errors());
}

/// Test duration_ms with numeric value
#[test]
fn test_duration_ms_numeric_value() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "0")]
        timeout: u64,
    }

    // Numeric value - passed through as-is (assumed to be in target unit)
    let config_str = r#"{"timeout": 5}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    // Already an integer, should be kept as-is
    assert_eq!(result.timeout, 5, "numeric value 5 should remain 5");
    assert!(!config.errors().has_errors());
}

/// Test duration_ms parsing failure with default
#[test]
fn test_duration_ms_parsing_error_uses_default() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "1000")]
        timeout: u64,
    }

    let config_str = r#"{"timeout": "invalid_duration"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed with default");

    assert_eq!(result.timeout, 1000, "should use default of 1000ms");
    assert!(
        config.errors().has_errors(),
        "Should have recorded an error"
    );
}

/// Test duration_ms on required field fails on parse error
#[test]
fn test_duration_ms_required_field_fails_on_parse_error() {
    #[derive(DeriveConfig, Debug)]
    struct RequiredDurationMsConfig {
        #[feuilletage(duration(ms))]
        timeout: u64,
    }

    let config_str = r#"{"timeout": "not-a-duration"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: Result<RequiredDurationMsConfig, _> = config.deserialize();
    assert!(
        result.is_err(),
        "Should fail when required duration(ms) field fails to parse"
    );
}

/// Test zero duration_ms
#[test]
fn test_duration_ms_zero() {
    #[derive(DeriveConfig, Debug)]
    struct DurationMsConfig {
        #[feuilletage(duration(ms), default = "1000")]
        delay: u64,
    }

    let config_str = r#"{"delay": "0ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationMsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.delay, 0, "0ms should be 0");
    assert!(!config.errors().has_errors());
}

// ============================================================================
// Combined duration with different units tests
// ============================================================================

/// Test using both duration (seconds) and duration(ms) in same struct
#[test]
fn test_duration_and_duration_ms_combined() {
    #[derive(DeriveConfig, Debug)]
    struct MixedDurationConfig {
        #[feuilletage(duration, default = "30")]
        timeout_secs: u64,

        #[feuilletage(duration(ms), default = "500")]
        poll_interval_ms: u64,
    }

    let config_str = r#"{
        "timeout_secs": "5m",
        "poll_interval_ms": "2s"
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MixedDurationConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.timeout_secs, 300, "5m should be 300 seconds");
    assert_eq!(
        result.poll_interval_ms, 2000,
        "2s should be 2000 milliseconds"
    );
    assert!(!config.errors().has_errors());
}

// ============================================================================
// Extended unit tests
// ============================================================================

/// Test duration with extended units (ns, us, etc.)
#[test]
fn test_duration_extended_units() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "0")]
        value: u64,
    }

    // 1 billion nanoseconds = 1 second
    let config_str = r#"{"value": "1000000000ns"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: DurationConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, 1, "1000000000ns should be 1 second");

    // 1 million microseconds = 1 second
    let config_str = r#"{"value": "1000000us"}"#;
    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));
    let result: DurationConfig = config.deserialize().expect("Should succeed");
    assert_eq!(result.value, 1, "1000000us should be 1 second");
}

/// Test duration with YAML format
#[test]
fn test_duration_yaml_format() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration, default = "0")]
        timeout: u64,

        #[feuilletage(duration(ms), default = "0")]
        interval: u64,
    }

    let config_str = r#"
timeout: "1h30m"
interval: "500ms"
"#;

    let mut loader = feuilletage::loader()
        .load_str(config_str, Format::Yaml, Level::User)
        .expect("Failed to load config");

    let result: DurationConfig = loader.deserialize().expect("Should succeed");

    assert_eq!(result.timeout, 5400, "1h30m should be 5400 seconds");
    assert_eq!(result.interval, 500, "500ms should be 500 milliseconds");
}

/// Test complex combined duration
#[test]
fn test_duration_complex_combined() {
    #[derive(DeriveConfig, Debug)]
    struct DurationConfig {
        #[feuilletage(duration(ms), default = "0")]
        value: u64,
    }

    let config_str = r#"{"value": "1h30m500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationConfig = config.deserialize().expect("Should succeed");

    // 1h = 3600000ms, 30m = 1800000ms, 500ms = 500ms
    // Total = 5400500ms
    assert_eq!(
        result.value, 5400500,
        "1h30m500ms should be 5400500 milliseconds"
    );
    assert!(!config.errors().has_errors());
}

// ============================================================================
// Float precision tests
// ============================================================================

/// Test duration with float type preserves precision
#[test]
fn test_duration_float_precision() {
    #[derive(DeriveConfig, Debug)]
    struct DurationFloatConfig {
        #[feuilletage(duration, default = "0.0")]
        timeout: f64,
    }

    let config_str = r#"{"timeout": "1s500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationFloatConfig = config.deserialize().expect("Should succeed");

    assert!(
        (result.timeout - 1.5).abs() < f64::EPSILON,
        "1s500ms should be 1.5 seconds, got {}",
        result.timeout
    );
    assert!(!config.errors().has_errors());
}

/// Test duration with f32 type
#[test]
fn test_duration_f32() {
    #[derive(DeriveConfig, Debug)]
    struct DurationF32Config {
        #[feuilletage(duration(ms), default = "0.0")]
        timeout: f32,
    }

    let config_str = r#"{"timeout": "1s500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationF32Config = config.deserialize().expect("Should succeed");

    assert!(
        (result.timeout - 1500.0).abs() < 0.001,
        "1s500ms should be 1500 ms, got {}",
        result.timeout
    );
    assert!(!config.errors().has_errors());
}

// ============================================================================
// All units tests
// ============================================================================

/// Test all supported duration units
#[test]
fn test_all_duration_units() {
    #[derive(DeriveConfig, Debug)]
    struct AllUnitsConfig {
        #[feuilletage(duration(ns), default = "0")]
        nanos: u64,

        #[feuilletage(duration(us), default = "0")]
        micros: u64,

        #[feuilletage(duration(ms), default = "0")]
        millis: u64,

        #[feuilletage(duration(s), default = "0")]
        secs: u64,

        #[feuilletage(duration(m), default = "0")]
        mins: u64,

        #[feuilletage(duration(h), default = "0")]
        hours: u64,

        #[feuilletage(duration(d), default = "0")]
        days: u64,

        #[feuilletage(duration(w), default = "0")]
        weeks: u64,
    }

    let config_str = r#"{
        "nanos": "1us",
        "micros": "1ms",
        "millis": "1s",
        "secs": "1m",
        "mins": "1h",
        "hours": "1d",
        "days": "1w",
        "weeks": "2w"
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: AllUnitsConfig = config.deserialize().expect("Should succeed");

    assert_eq!(result.nanos, 1000, "1us should be 1000 ns");
    assert_eq!(result.micros, 1000, "1ms should be 1000 us");
    assert_eq!(result.millis, 1000, "1s should be 1000 ms");
    assert_eq!(result.secs, 60, "1m should be 60 s");
    assert_eq!(result.mins, 60, "1h should be 60 m");
    assert_eq!(result.hours, 24, "1d should be 24 h");
    assert_eq!(result.days, 7, "1w should be 7 d");
    assert_eq!(result.weeks, 2, "2w should be 2 w");
    assert!(!config.errors().has_errors());
}

// ============================================================================
// Explicit duration(unit = ...) syntax tests
// ============================================================================

/// Test explicit syntax: duration(unit = ms)
#[test]
fn test_duration_explicit_syntax_ms() {
    #[derive(DeriveConfig, Debug)]
    struct DurationExplicitConfig {
        #[feuilletage(duration(unit = ms), default = "0")]
        timeout_ms: u64,
    }

    let config_str = r#"{"timeout_ms": "5s"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: DurationExplicitConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.timeout_ms, 5000,
        "5s should be 5000 milliseconds with explicit syntax"
    );
    assert!(!config.errors().has_errors());
}

/// Test explicit syntax with all supported units
#[test]
fn test_duration_explicit_syntax_all_units() {
    #[derive(DeriveConfig, Debug)]
    struct AllUnitsExplicitConfig {
        #[feuilletage(duration(unit = ns), default = "0")]
        nanos: u64,

        #[feuilletage(duration(unit = us), default = "0")]
        micros: u64,

        #[feuilletage(duration(unit = ms), default = "0")]
        millis: u64,

        #[feuilletage(duration(unit = s), default = "0")]
        secs: u64,

        #[feuilletage(duration(unit = m), default = "0")]
        mins: u64,

        #[feuilletage(duration(unit = h), default = "0")]
        hours: u64,

        #[feuilletage(duration(unit = d), default = "0")]
        days: u64,

        #[feuilletage(duration(unit = w), default = "0")]
        weeks: u64,
    }

    let config_str = r#"{
        "nanos": "1us",
        "micros": "1ms",
        "millis": "1s",
        "secs": "1m",
        "mins": "1h",
        "hours": "1d",
        "days": "1w",
        "weeks": "2w"
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: AllUnitsExplicitConfig = config.deserialize().expect("Should succeed");

    assert_eq!(
        result.nanos, 1000,
        "1us should be 1000 ns (explicit syntax)"
    );
    assert_eq!(
        result.micros, 1000,
        "1ms should be 1000 us (explicit syntax)"
    );
    assert_eq!(
        result.millis, 1000,
        "1s should be 1000 ms (explicit syntax)"
    );
    assert_eq!(result.secs, 60, "1m should be 60 s (explicit syntax)");
    assert_eq!(result.mins, 60, "1h should be 60 m (explicit syntax)");
    assert_eq!(result.hours, 24, "1d should be 24 h (explicit syntax)");
    assert_eq!(result.days, 7, "1w should be 7 d (explicit syntax)");
    assert_eq!(result.weeks, 2, "2w should be 2 w (explicit syntax)");
    assert!(!config.errors().has_errors());
}

/// Test that both shorthand and explicit syntaxes can be used in the same struct
#[test]
fn test_duration_mixed_syntax() {
    #[derive(DeriveConfig, Debug)]
    struct MixedSyntaxConfig {
        // Shorthand syntax
        #[feuilletage(duration(ms), default = "0")]
        shorthand_ms: u64,

        // Explicit syntax
        #[feuilletage(duration(unit = ms), default = "0")]
        explicit_ms: u64,

        // Default (seconds)
        #[feuilletage(duration, default = "0")]
        default_secs: u64,

        // Shorthand different unit
        #[feuilletage(duration(ns), default = "0")]
        shorthand_ns: u64,

        // Explicit different unit
        #[feuilletage(duration(unit = ns), default = "0")]
        explicit_ns: u64,
    }

    let config_str = r#"{
        "shorthand_ms": "2s",
        "explicit_ms": "2s",
        "default_secs": "2s",
        "shorthand_ns": "1us",
        "explicit_ns": "1us"
    }"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: MixedSyntaxConfig = config.deserialize().expect("Should succeed");

    // Both shorthand and explicit should produce the same result
    assert_eq!(result.shorthand_ms, 2000, "shorthand 2s should be 2000 ms");
    assert_eq!(result.explicit_ms, 2000, "explicit 2s should be 2000 ms");
    assert_eq!(
        result.shorthand_ms, result.explicit_ms,
        "shorthand and explicit should produce same result"
    );

    assert_eq!(result.default_secs, 2, "default 2s should be 2 seconds");

    assert_eq!(result.shorthand_ns, 1000, "shorthand 1us should be 1000 ns");
    assert_eq!(result.explicit_ns, 1000, "explicit 1us should be 1000 ns");
    assert_eq!(
        result.shorthand_ns, result.explicit_ns,
        "shorthand and explicit ns should produce same result"
    );

    assert!(!config.errors().has_errors());
}

/// Test explicit syntax with combined duration format
#[test]
fn test_duration_explicit_syntax_combined() {
    #[derive(DeriveConfig, Debug)]
    struct CombinedExplicitConfig {
        #[feuilletage(duration(unit = ms), default = "0")]
        value: u64,
    }

    let config_str = r#"{"value": "1h30m500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: CombinedExplicitConfig = config.deserialize().expect("Should succeed");

    // 1h = 3600000ms, 30m = 1800000ms, 500ms = 500ms
    // Total = 5400500ms
    assert_eq!(
        result.value, 5400500,
        "1h30m500ms should be 5400500 milliseconds with explicit syntax"
    );
    assert!(!config.errors().has_errors());
}

/// Test explicit syntax with float type for precision
#[test]
fn test_duration_explicit_syntax_float() {
    #[derive(DeriveConfig, Debug)]
    struct FloatExplicitConfig {
        #[feuilletage(duration(unit = s), default = "0.0")]
        timeout: f64,
    }

    let config_str = r#"{"timeout": "1s500ms"}"#;

    let mut config = Config::default();
    config.load_json(config_str, Context::new(Source::Programmatic, Level::User));

    let result: FloatExplicitConfig = config.deserialize().expect("Should succeed");

    assert!(
        (result.timeout - 1.5).abs() < f64::EPSILON,
        "1s500ms should be 1.5 seconds with explicit syntax, got {}",
        result.timeout
    );
    assert!(!config.errors().has_errors());
}

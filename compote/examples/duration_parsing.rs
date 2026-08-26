//! Duration Parsing Pattern: Human-readable time strings
//!
//! Common use cases:
//! - Connection and request timeouts
//! - Cache TTLs and expiration times
//! - Rate limit windows
//! - Retry intervals and backoff delays
//!
//! Compote Solution: `#[compote(duration)]`
//!
//! Example:
//! ```yaml
//! connect_timeout: "30s"    # 30 seconds
//! read_timeout: "5m"        # 5 minutes
//! cache_ttl: "1h"           # 1 hour
//! max_age: "7d"             # 7 days
//! ```
//!
//! Supported syntax:
//! - `#[compote(duration)]` - Parse to seconds (default)
//! - `#[compote(duration(ns))]` - Parse to nanoseconds
//! - `#[compote(duration(us))]` - Parse to microseconds
//! - `#[compote(duration(ms))]` - Parse to milliseconds
//! - `#[compote(duration(s))]` - Parse to seconds (explicit)
//! - `#[compote(duration(m))]` - Parse to minutes
//! - `#[compote(duration(h))]` - Parse to hours
//! - `#[compote(duration(d))]` - Parse to days
//! - `#[compote(duration(w))]` - Parse to weeks
//!
//! Field type determines precision:
//! - Integer types (u64, i64, etc.): Values are truncated
//! - Float types (f64, f32): Preserves fractional values

use compote::{Config as ConfigContainer, Context, Error, FromContextValue, Level, Source};

/// Config with duration fields (default: seconds)
#[derive(Debug, compote::Config, PartialEq)]
struct TimeoutConfig {
    #[compote(duration)]
    connect_timeout: i64,

    #[compote(duration)]
    read_timeout: i64,

    #[compote(duration, default = "30")]
    write_timeout: i64,
}

/// Config with millisecond durations
#[derive(Debug, compote::Config, PartialEq)]
struct MillisConfig {
    #[compote(duration(ms))]
    latency: u64,

    #[compote(duration(ms), default = "100")]
    poll_interval: u64,
}

/// Config with float precision
#[derive(Debug, compote::Config)]
struct PreciseConfig {
    #[compote(duration)]
    timeout_secs: f64,

    #[compote(duration(ms))]
    latency_ms: f64,
}

/// Config with all unit types
#[derive(Debug, compote::Config, PartialEq)]
struct AllUnitsConfig {
    #[compote(duration(ns), default = "0")]
    nanos: u64,

    #[compote(duration(us), default = "0")]
    micros: u64,

    #[compote(duration(ms), default = "0")]
    millis: u64,

    #[compote(duration(s), default = "0")]
    secs: u64,

    #[compote(duration(m), default = "0")]
    mins: u64,

    #[compote(duration(h), default = "0")]
    hours: u64,

    #[compote(duration(d), default = "0")]
    days: u64,

    #[compote(duration(w), default = "0")]
    weeks: u64,
}

/// Helper to deserialize JSON
fn deserialize_json<T: FromContextValue>(json: &str) -> Result<T, Error> {
    let mut config = ConfigContainer::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    config.deserialize::<T>()
}

fn main() {
    println!("=== Duration Parsing Examples ===\n");

    // Basic seconds parsing
    println!("--- Basic Seconds (default unit) ---");
    let json = r#"{"connect_timeout": "30s", "read_timeout": "60s"}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should parse seconds");
    println!("30s -> {} seconds", config.connect_timeout);
    println!("60s -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 30);
    assert_eq!(config.read_timeout, 60);

    // Minutes
    println!("\n--- Minutes ---");
    let json = r#"{"connect_timeout": "5m", "read_timeout": "10m"}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should parse minutes");
    println!("5m -> {} seconds", config.connect_timeout);
    println!("10m -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 300); // 5 * 60
    assert_eq!(config.read_timeout, 600); // 10 * 60

    // Hours
    println!("\n--- Hours ---");
    let json = r#"{"connect_timeout": "1h", "read_timeout": "2h"}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should parse hours");
    println!("1h -> {} seconds", config.connect_timeout);
    println!("2h -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 3600); // 1 * 60 * 60
    assert_eq!(config.read_timeout, 7200); // 2 * 60 * 60

    // Days
    println!("\n--- Days ---");
    let json = r#"{"connect_timeout": "1d", "read_timeout": "7d"}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should parse days");
    println!("1d -> {} seconds", config.connect_timeout);
    println!("7d -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 86400); // 24 * 60 * 60
    assert_eq!(config.read_timeout, 604800); // 7 * 24 * 60 * 60

    // Weeks
    println!("\n--- Weeks ---");
    let json = r#"{"connect_timeout": "1w", "read_timeout": "2w"}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should parse weeks");
    println!("1w -> {} seconds", config.connect_timeout);
    println!("2w -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 604800); // 7 * 24 * 60 * 60
    assert_eq!(config.read_timeout, 1209600); // 2 * 7 * 24 * 60 * 60

    // Raw integers (seconds)
    println!("\n--- Raw Integers ---");
    let json = r#"{"connect_timeout": 120, "read_timeout": 300}"#;
    let config: TimeoutConfig = deserialize_json(json).expect("Should accept raw integers");
    println!("120 -> {} seconds", config.connect_timeout);
    println!("300 -> {} seconds", config.read_timeout);
    assert_eq!(config.connect_timeout, 120);
    assert_eq!(config.read_timeout, 300);

    // Milliseconds with duration(ms)
    println!("\n--- Milliseconds (duration(ms)) ---");
    let json = r#"{"latency": "500ms", "poll_interval": "2s"}"#;
    let config: MillisConfig = deserialize_json(json).expect("Should parse milliseconds");
    println!("500ms -> {} ms", config.latency);
    println!("2s -> {} ms", config.poll_interval);
    assert_eq!(config.latency, 500);
    assert_eq!(config.poll_interval, 2000);

    // Float precision
    println!("\n--- Float Precision ---");
    let json = r#"{"timeout_secs": "1s500ms", "latency_ms": "100us"}"#;
    let config: PreciseConfig = deserialize_json(json).expect("Should preserve precision");
    println!("1s500ms -> {} seconds (float)", config.timeout_secs);
    println!("100us -> {} ms (float)", config.latency_ms);
    assert!((config.timeout_secs - 1.5).abs() < 1e-9);
    assert!((config.latency_ms - 0.1).abs() < 1e-9);

    // All units
    println!("\n--- All Units ---");
    let json = r#"{
        "nanos": "1us",
        "micros": "1ms",
        "millis": "1s",
        "secs": "1m",
        "mins": "1h",
        "hours": "1d",
        "days": "1w",
        "weeks": "2w"
    }"#;
    let config: AllUnitsConfig = deserialize_json(json).expect("Should parse all units");
    println!("1us -> {} ns", config.nanos);
    println!("1ms -> {} us", config.micros);
    println!("1s -> {} ms", config.millis);
    println!("1m -> {} s", config.secs);
    println!("1h -> {} m", config.mins);
    println!("1d -> {} h", config.hours);
    println!("1w -> {} d", config.days);
    println!("2w -> {} w", config.weeks);
    assert_eq!(config.nanos, 1000);
    assert_eq!(config.micros, 1000);
    assert_eq!(config.millis, 1000);
    assert_eq!(config.secs, 60);
    assert_eq!(config.mins, 60);
    assert_eq!(config.hours, 24);
    assert_eq!(config.days, 7);
    assert_eq!(config.weeks, 2);

    // Real-world: Cache Config
    println!("\n--- Real-World: Cache Config ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct CacheConfig {
        #[compote(default = "true")]
        enabled: bool,

        #[compote(duration, default = "3600")] // 1 hour default
        ttl: i64,

        #[compote(duration(ms), default = "5000")] // 5 seconds default
        refresh_interval_ms: u64,

        #[compote(duration, default = "86400")] // 1 day default
        max_age: i64,
    }

    let json = r#"{
        "enabled": true,
        "ttl": "2h",
        "refresh_interval_ms": "500ms",
        "max_age": "7d"
    }"#;
    let config: CacheConfig = deserialize_json(json).expect("Cache config");
    println!("enabled: {}", config.enabled);
    println!("ttl: {} seconds (2h)", config.ttl);
    println!(
        "refresh_interval_ms: {} ms (500ms)",
        config.refresh_interval_ms
    );
    println!("max_age: {} seconds (7d)", config.max_age);
    assert!(config.enabled);
    assert_eq!(config.ttl, 7200);
    assert_eq!(config.refresh_interval_ms, 500);
    assert_eq!(config.max_age, 604800);

    // Real-world: Job Config
    println!("\n--- Real-World: Job Config ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct JobConfig {
        name: String,

        #[compote(duration)]
        interval: i64,

        #[compote(duration, default = "300")] // 5 minutes default
        timeout: i64,

        #[compote(duration(ms), default = "60000")] // 1 minute default
        retry_delay_ms: u64,

        #[compote(default = "3")]
        max_retries: i32,
    }

    let json = r#"{
        "name": "cleanup",
        "interval": "24h",
        "timeout": "30m",
        "retry_delay_ms": "5s",
        "max_retries": 5
    }"#;
    let config: JobConfig = deserialize_json(json).expect("Job config");
    println!("name: {}", config.name);
    println!("interval: {} seconds (24h)", config.interval);
    println!("timeout: {} seconds (30m)", config.timeout);
    println!("retry_delay_ms: {} ms (5s)", config.retry_delay_ms);
    println!("max_retries: {}", config.max_retries);
    assert_eq!(config.name, "cleanup");
    assert_eq!(config.interval, 86400);
    assert_eq!(config.timeout, 1800);
    assert_eq!(config.retry_delay_ms, 5000);
    assert_eq!(config.max_retries, 5);

    // Vec with duration transform
    println!("\n--- Vec with Duration Transform ---");

    #[derive(Debug, compote::Config, PartialEq)]
    struct MultiTimeoutConfig {
        #[compote(allow_single, transform_each = "parse_duration")]
        timeouts: Vec<i64>,
    }

    let json = r#"{"timeouts": ["30s", "1m", "5m", "1h"]}"#;
    let config: MultiTimeoutConfig = deserialize_json(json).expect("Vec with duration");
    println!("timeouts: {:?}", config.timeouts);
    assert_eq!(config.timeouts, vec![30, 60, 300, 3600]);

    let json = r#"{"timeouts": "5m"}"#;
    let config: MultiTimeoutConfig = deserialize_json(json).expect("Single duration value");
    println!("single timeout: {:?}", config.timeouts);
    assert_eq!(config.timeouts, vec![300]);

    println!("\n=== All duration parsing examples passed! ===");
}

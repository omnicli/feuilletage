//! Date/Time Format Validation Pattern: String format validation using strftime
//!
//! Common use cases:
//! - Date fields (YYYY-MM-DD)
//! - Timestamp fields
//! - Custom date/time formats
//! - Log rotation date patterns
//!
//! Feuilletage Solution: `#[feuilletage(datetime = "%Y-%m-%d")]`
//!
//! Example:
//! ```yaml
//! date_field: "2024-01-15"  # Valid: matches %Y-%m-%d
//! date_field: "not-a-date"  # Error: invalid format
//! ```

use feuilletage::{Config, Context, Level, Source};

#[derive(Debug, feuilletage::Config)]
struct DateConfig {
    #[feuilletage(datetime = "%Y-%m-%d")]
    date_field: String,
}

fn main() {
    // Test valid date
    let yaml = "date_field: '2024-01-15'";

    let mut config = Config::default();
    config.load_yaml(yaml, Context::new(Source::Programmatic, Level::User));

    let result = config.deserialize::<DateConfig>();
    println!("Valid date result: {:?}", result);
    assert!(result.is_ok());

    // Test invalid date
    let yaml2 = "date_field: 'not-a-date'";

    let mut config2 = Config::default();
    config2.load_yaml(yaml2, Context::new(Source::Programmatic, Level::User));

    let result2 = config2.deserialize::<DateConfig>();
    println!("Invalid date result: {:?}", result2);
    assert!(result2.is_err());

    println!("All format validation tests passed!");
}

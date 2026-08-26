//! Minimal environment variable example
//!
//! This is a minimal example showing environment variable loading.

use compote::{Config as ConfigContainer, Context, Level, Source};

/// Config with environment variable loading
#[derive(Debug, compote::Config)]
struct EnvMinimal {
    #[compote(env = "TEST_MINIMAL_KEY")]
    value: Option<String>,
}

fn main() {
    println!("=== Minimal Environment Example ===\n");

    // Set up
    std::env::set_var("TEST_MINIMAL_KEY", "hello_from_env");

    let mut config = ConfigContainer::default();
    config.load_json(r#"{}"#, Context::new(Source::Programmatic, Level::User));

    let result: EnvMinimal = config.deserialize().expect("Should deserialize");
    println!("value: {:?}", result.value);
    assert_eq!(result.value, Some("hello_from_env".to_string()));

    // Cleanup
    std::env::remove_var("TEST_MINIMAL_KEY");

    println!("\n=== Minimal environment example passed! ===");
}

//! Unit tests for ser module (serialization).
//!
//! Extracted from compote/src/ser.rs

use compote::{Context, ContextValue, Format, Level, Source};
use indexmap::IndexMap;

fn test_context(format: Format) -> Context {
    Context::new(Source::Programmatic, Level::User).with_format(format)
}

#[test]
#[cfg(feature = "json")]
fn test_json_serialization() {
    let mut map = IndexMap::new();
    map.insert(
        "name".to_string(),
        ContextValue::string("test", test_context(Format::Json)),
    );
    map.insert(
        "count".to_string(),
        ContextValue::int(42, test_context(Format::Json)),
    );

    let config = ContextValue::object(map, test_context(Format::Json));
    let json = config.to_json().unwrap();

    assert!(json.contains("\"name\""));
    assert!(json.contains("\"test\""));
    assert!(json.contains("\"count\""));
    assert!(json.contains("42"));
}

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_serialization() {
    let mut map = IndexMap::new();
    map.insert(
        "name".to_string(),
        ContextValue::string("test", test_context(Format::Yaml)),
    );
    map.insert(
        "count".to_string(),
        ContextValue::int(42, test_context(Format::Yaml)),
    );

    let config = ContextValue::object(map, test_context(Format::Yaml));
    let yaml = config.to_yaml().unwrap();

    assert!(yaml.contains("name"));
    assert!(yaml.contains("test"));
    assert!(yaml.contains("count"));
    assert!(yaml.contains("42"));
}

#[test]
#[cfg(feature = "toml")]
fn test_toml_serialization() {
    let mut map = IndexMap::new();
    map.insert(
        "name".to_string(),
        ContextValue::string("test", test_context(Format::Toml)),
    );
    map.insert(
        "count".to_string(),
        ContextValue::int(42, test_context(Format::Toml)),
    );

    let config = ContextValue::object(map, test_context(Format::Toml));
    let toml_str = config.to_toml().unwrap();

    assert!(toml_str.contains("name"));
    assert!(toml_str.contains("test"));
    assert!(toml_str.contains("count"));
    assert!(toml_str.contains("42"));
}

#[test]
#[cfg(feature = "json")]
fn test_serialize_with_format() {
    let mut map = IndexMap::new();
    map.insert(
        "key".to_string(),
        ContextValue::string("value", test_context(Format::Json)),
    );

    let config = ContextValue::object(map, test_context(Format::Json));
    let serialized = config.serialize().unwrap();

    // Should serialize to JSON since that's the format in context
    assert!(serialized.contains("\"key\""));
    assert!(serialized.contains("\"value\""));
}

// ============================================================================
// Tests for standalone serialization functions
// ============================================================================

#[test]
#[cfg(feature = "json")]
fn test_to_json_function() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestConfig {
        name: String,
        count: i32,
    }

    let config = TestConfig {
        name: "test".to_string(),
        count: 42,
    };

    let json = compote::to_json(&config).unwrap();
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"test\""));
    assert!(json.contains("\"count\""));
    assert!(json.contains("42"));
    // Verify it's pretty-printed
    assert!(json.contains('\n'));
}

#[test]
#[cfg(feature = "json")]
fn test_to_json_compact_function() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestConfig {
        name: String,
        count: i32,
    }

    let config = TestConfig {
        name: "test".to_string(),
        count: 42,
    };

    let json = compote::to_json_compact(&config).unwrap();
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"test\""));
    // Verify it's compact (no newlines)
    assert!(!json.contains('\n'));
    assert_eq!(json, r#"{"name":"test","count":42}"#);
}

#[test]
#[cfg(feature = "yaml")]
fn test_to_yaml_function() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestConfig {
        name: String,
        count: i32,
    }

    let config = TestConfig {
        name: "test".to_string(),
        count: 42,
    };

    let yaml = compote::to_yaml(&config).unwrap();
    assert!(yaml.contains("name"));
    assert!(yaml.contains("test"));
    assert!(yaml.contains("count"));
    assert!(yaml.contains("42"));
}

#[test]
#[cfg(feature = "toml")]
fn test_to_toml_function() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestConfig {
        name: String,
        count: i32,
    }

    let config = TestConfig {
        name: "test".to_string(),
        count: 42,
    };

    let toml = compote::to_toml(&config).unwrap();
    assert!(toml.contains("name"));
    assert!(toml.contains("test"));
    assert!(toml.contains("count"));
    assert!(toml.contains("42"));
}

#[test]
#[cfg(feature = "json")]
fn test_to_format_function() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestConfig {
        name: String,
    }

    let config = TestConfig {
        name: "test".to_string(),
    };

    let json = compote::to_format(&config, Format::Json).unwrap();
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"test\""));
}

// ============================================================================
// Tests for Config format-aware serialization
// ============================================================================

#[test]
#[cfg(feature = "json")]
fn test_config_loaded_format_json() {
    use compote::Config;

    let mut config = Config::default();
    assert_eq!(config.loaded_format(), Format::Unknown);

    config.load_json(
        r#"{"key": "value"}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    assert_eq!(config.loaded_format(), Format::Json);
}

#[test]
#[cfg(feature = "yaml")]
fn test_config_loaded_format_yaml() {
    use compote::Config;

    let mut config = Config::default();
    config.load_yaml(
        "key: value",
        Context::new(Source::Programmatic, Level::User),
    );

    assert_eq!(config.loaded_format(), Format::Yaml);
}

#[test]
#[cfg(feature = "toml")]
fn test_config_loaded_format_toml() {
    use compote::Config;

    let mut config = Config::default();
    config.load_toml(
        "key = \"value\"",
        Context::new(Source::Programmatic, Level::User),
    );

    assert_eq!(config.loaded_format(), Format::Toml);
}

#[test]
#[cfg(feature = "json")]
fn test_config_serialize() {
    use compote::Config;

    // compote::Config already derives Serialize, so no need to add it separately
    #[derive(Debug, compote::Config, PartialEq)]
    struct AppConfig {
        #[compote(default = "localhost")]
        host: String,
        #[compote(default = "8080")]
        port: i32,
    }

    let mut config = Config::default();
    config.load_json(
        r#"{"host": "example.com", "port": 3000}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    let app: AppConfig = config.deserialize().unwrap();
    assert_eq!(app.host, "example.com");
    assert_eq!(app.port, 3000);

    // Serialize back to JSON (the format we loaded from)
    let json_output = config.serialize(&app).unwrap();
    assert!(json_output.contains("example.com"));
    assert!(json_output.contains("3000"));
}

#[test]
#[cfg(all(feature = "json", feature = "yaml"))]
fn test_config_loaded_format_changes_with_each_load() {
    use compote::Config;

    let mut config = Config::default();

    // Load JSON first
    config.load_json(
        r#"{"a": 1}"#,
        Context::new(Source::Programmatic, Level::User),
    );
    assert_eq!(config.loaded_format(), Format::Json);

    // Then load YAML - format should change
    config.load_yaml("b: 2", Context::new(Source::Programmatic, Level::User));
    assert_eq!(config.loaded_format(), Format::Yaml);
}

#[test]
#[cfg(feature = "toml")]
fn test_config_default_format_serializes_new_config() {
    use compote::Config;

    #[derive(serde::Serialize)]
    struct AppConfig {
        name: &'static str,
    }

    let mut config = Config::default().with_default_format(Format::Toml).unwrap();
    config.at("name").set("compote").unwrap();

    assert_eq!(config.loaded_format(), Format::Unknown);
    assert_eq!(config.default_format(), Format::Toml);
    assert!(config
        .serialize(&AppConfig { name: "compote" })
        .unwrap()
        .contains("name = \"compote\""));
    assert!(config
        .serialize_raw()
        .unwrap()
        .contains("name = \"compote\""));
}

#[test]
#[cfg(all(feature = "json", feature = "toml"))]
fn test_loaded_format_takes_precedence_over_default_format() {
    use compote::Config;

    let mut config = Config::default().with_default_format(Format::Toml).unwrap();
    config.load_json(
        r#"{"name": "compote"}"#,
        Context::new(Source::Programmatic, Level::User),
    );

    assert_eq!(config.default_format(), Format::Toml);
    assert_eq!(config.loaded_format(), Format::Json);
    serde_json::from_str::<serde_json::Value>(&config.serialize_raw().unwrap()).unwrap();
}

// ============================================================================
// Tests for alphabetically sorted to_yaml()
// ============================================================================

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_keys_are_sorted_alphabetically() {
    // Insert keys in non-alphabetical order
    let mut map = IndexMap::new();
    map.insert(
        "zebra".to_string(),
        ContextValue::string("z", test_context(Format::Yaml)),
    );
    map.insert(
        "apple".to_string(),
        ContextValue::string("a", test_context(Format::Yaml)),
    );
    map.insert(
        "mango".to_string(),
        ContextValue::string("m", test_context(Format::Yaml)),
    );
    map.insert(
        "banana".to_string(),
        ContextValue::string("b", test_context(Format::Yaml)),
    );

    let config = ContextValue::object(map, test_context(Format::Yaml));
    let yaml = config.to_yaml().unwrap();

    // Find positions of keys in output
    let apple_pos = yaml.find("apple").unwrap();
    let banana_pos = yaml.find("banana").unwrap();
    let mango_pos = yaml.find("mango").unwrap();
    let zebra_pos = yaml.find("zebra").unwrap();

    // Keys should appear in alphabetical order
    assert!(apple_pos < banana_pos, "apple should come before banana");
    assert!(banana_pos < mango_pos, "banana should come before mango");
    assert!(mango_pos < zebra_pos, "mango should come before zebra");
}

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_nested_keys_are_sorted() {
    // Create nested structure with unsorted keys
    let mut inner = IndexMap::new();
    inner.insert(
        "zulu".to_string(),
        ContextValue::int(3, test_context(Format::Yaml)),
    );
    inner.insert(
        "alpha".to_string(),
        ContextValue::int(1, test_context(Format::Yaml)),
    );
    inner.insert(
        "bravo".to_string(),
        ContextValue::int(2, test_context(Format::Yaml)),
    );

    let mut outer = IndexMap::new();
    outer.insert(
        "nested".to_string(),
        ContextValue::object(inner, test_context(Format::Yaml)),
    );
    outer.insert(
        "zebra".to_string(),
        ContextValue::string("z", test_context(Format::Yaml)),
    );
    outer.insert(
        "aardvark".to_string(),
        ContextValue::string("a", test_context(Format::Yaml)),
    );

    let config = ContextValue::object(outer, test_context(Format::Yaml));
    let yaml = config.to_yaml().unwrap();

    // Outer keys should be sorted
    let aardvark_pos = yaml.find("aardvark").unwrap();
    let nested_pos = yaml.find("nested").unwrap();
    let zebra_pos = yaml.find("zebra").unwrap();

    assert!(
        aardvark_pos < nested_pos,
        "aardvark should come before nested"
    );
    assert!(nested_pos < zebra_pos, "nested should come before zebra");

    // Inner keys should also be sorted
    let alpha_pos = yaml.find("alpha").unwrap();
    let bravo_pos = yaml.find("bravo").unwrap();
    let zulu_pos = yaml.find("zulu").unwrap();

    assert!(alpha_pos < bravo_pos, "alpha should come before bravo");
    assert!(bravo_pos < zulu_pos, "bravo should come before zulu");
}

#[test]
#[cfg(feature = "yaml")]
fn test_yaml_array_with_objects_sorted() {
    // Create array with objects that have unsorted keys
    let mut obj1 = IndexMap::new();
    obj1.insert(
        "zkey".to_string(),
        ContextValue::int(1, test_context(Format::Yaml)),
    );
    obj1.insert(
        "akey".to_string(),
        ContextValue::int(2, test_context(Format::Yaml)),
    );

    let mut obj2 = IndexMap::new();
    obj2.insert(
        "ykey".to_string(),
        ContextValue::int(3, test_context(Format::Yaml)),
    );
    obj2.insert(
        "bkey".to_string(),
        ContextValue::int(4, test_context(Format::Yaml)),
    );

    let arr = vec![
        ContextValue::object(obj1, test_context(Format::Yaml)),
        ContextValue::object(obj2, test_context(Format::Yaml)),
    ];

    let mut root = IndexMap::new();
    root.insert(
        "items".to_string(),
        ContextValue::array(arr, test_context(Format::Yaml)),
    );

    let config = ContextValue::object(root, test_context(Format::Yaml));
    let yaml = config.to_yaml().unwrap();

    // In the first object, 'akey' should come before 'zkey'
    let akey_pos = yaml.find("akey").expect("akey should be in YAML");
    let zkey_pos = yaml.find("zkey").expect("zkey should be in YAML");
    assert!(
        akey_pos < zkey_pos,
        "akey should come before zkey in first object"
    );

    // In the second object, 'bkey' should come before 'ykey'
    let bkey_pos = yaml.find("bkey").expect("bkey should be in YAML");
    let ykey_pos = yaml.find("ykey").expect("ykey should be in YAML");
    assert!(
        bkey_pos < ykey_pos,
        "bkey should come before ykey in second object"
    );
}

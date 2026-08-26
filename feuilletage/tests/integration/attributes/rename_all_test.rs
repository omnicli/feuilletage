//! Tests for rename_all case conversions
//!
//! This file tests all supported case conventions:
//! - lowercase
//! - UPPERCASE
//! - snake_case
//! - camelCase
//! - PascalCase
//! - kebab-case
//! - SCREAMING_SNAKE_CASE

use feuilletage::{Config, Context, Level, Source};

// =============================================================================
// Helper structs for testing
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
struct SimpleConfig {
    #[feuilletage(default)]
    value: Option<String>,
}

// =============================================================================
// lowercase tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "lowercase")]
enum LowercaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_lowercase() {
    // FirstVariant -> firstvariant
    let json = r#"{"firstvariant": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<LowercaseEnum>();
    assert!(result.is_ok(), "Should match lowercase: {:?}", result);
    assert!(matches!(result.unwrap(), LowercaseEnum::FirstVariant(_)));

    // MyLongVariantName -> mylongvariantname
    let json2 = r#"{"mylongvariantname": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<LowercaseEnum>();
    assert!(result2.is_ok(), "Should match lowercase: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        LowercaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// UPPERCASE tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "UPPERCASE")]
enum UppercaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_uppercase() {
    // FirstVariant -> FIRSTVARIANT
    let json = r#"{"FIRSTVARIANT": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<UppercaseEnum>();
    assert!(result.is_ok(), "Should match UPPERCASE: {:?}", result);
    assert!(matches!(result.unwrap(), UppercaseEnum::FirstVariant(_)));

    // MyLongVariantName -> MYLONGVARIANTNAME
    let json2 = r#"{"MYLONGVARIANTNAME": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<UppercaseEnum>();
    assert!(result2.is_ok(), "Should match UPPERCASE: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        UppercaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// snake_case tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "snake_case")]
enum SnakeCaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_snake_case() {
    // FirstVariant -> first_variant
    let json = r#"{"first_variant": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<SnakeCaseEnum>();
    assert!(result.is_ok(), "Should match snake_case: {:?}", result);
    assert!(matches!(result.unwrap(), SnakeCaseEnum::FirstVariant(_)));

    // MyLongVariantName -> my_long_variant_name
    let json2 = r#"{"my_long_variant_name": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<SnakeCaseEnum>();
    assert!(result2.is_ok(), "Should match snake_case: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        SnakeCaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// camelCase tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "camelCase")]
enum CamelCaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_camel_case() {
    // FirstVariant -> firstVariant
    let json = r#"{"firstVariant": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<CamelCaseEnum>();
    assert!(result.is_ok(), "Should match camelCase: {:?}", result);
    assert!(matches!(result.unwrap(), CamelCaseEnum::FirstVariant(_)));

    // MyLongVariantName -> myLongVariantName
    let json2 = r#"{"myLongVariantName": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<CamelCaseEnum>();
    assert!(result2.is_ok(), "Should match camelCase: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        CamelCaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// PascalCase tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "PascalCase")]
enum PascalCaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_pascal_case() {
    // FirstVariant -> FirstVariant (no change)
    let json = r#"{"FirstVariant": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<PascalCaseEnum>();
    assert!(result.is_ok(), "Should match PascalCase: {:?}", result);
    assert!(matches!(result.unwrap(), PascalCaseEnum::FirstVariant(_)));

    // MyLongVariantName -> MyLongVariantName (no change)
    let json2 = r#"{"MyLongVariantName": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<PascalCaseEnum>();
    assert!(result2.is_ok(), "Should match PascalCase: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        PascalCaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// kebab-case tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "kebab-case")]
enum KebabCaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_kebab_case() {
    // FirstVariant -> first-variant
    let json = r#"{"first-variant": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<KebabCaseEnum>();
    assert!(result.is_ok(), "Should match kebab-case: {:?}", result);
    assert!(matches!(result.unwrap(), KebabCaseEnum::FirstVariant(_)));

    // MyLongVariantName -> my-long-variant-name
    let json2 = r#"{"my-long-variant-name": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<KebabCaseEnum>();
    assert!(result2.is_ok(), "Should match kebab-case: {:?}", result2);
    assert!(matches!(
        result2.unwrap(),
        KebabCaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// SCREAMING_SNAKE_CASE tests
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(external_tag, rename_all = "SCREAMING_SNAKE_CASE")]
enum ScreamingSnakeCaseEnum {
    FirstVariant(SimpleConfig),
    SecondVariant(SimpleConfig),
    MyLongVariantName(SimpleConfig),
}

#[test]
fn test_rename_all_screaming_snake_case() {
    // FirstVariant -> FIRST_VARIANT
    let json = r#"{"FIRST_VARIANT": {}}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<ScreamingSnakeCaseEnum>();
    assert!(
        result.is_ok(),
        "Should match SCREAMING_SNAKE_CASE: {:?}",
        result
    );
    assert!(matches!(
        result.unwrap(),
        ScreamingSnakeCaseEnum::FirstVariant(_)
    ));

    // MyLongVariantName -> MY_LONG_VARIANT_NAME
    let json2 = r#"{"MY_LONG_VARIANT_NAME": {}}"#;
    let mut config2 = Config::default();
    config2.load_json(json2, Context::new(Source::Programmatic, Level::User));
    let result2 = config2.deserialize::<ScreamingSnakeCaseEnum>();
    assert!(
        result2.is_ok(),
        "Should match SCREAMING_SNAKE_CASE: {:?}",
        result2
    );
    assert!(matches!(
        result2.unwrap(),
        ScreamingSnakeCaseEnum::MyLongVariantName(_)
    ));
}

// =============================================================================
// rename_all with internal tag
// =============================================================================

#[derive(Debug, feuilletage::Config, PartialEq)]
#[feuilletage(tag = "type", rename_all = "snake_case")]
enum InternalTagSnakeCase {
    FirstVariant { value: String },
    SecondVariant { count: i32 },
}

#[test]
fn test_rename_all_internal_tag() {
    let json = r#"{"type": "first_variant", "value": "hello"}"#;
    let mut config = Config::default();
    config.load_json(json, Context::new(Source::Programmatic, Level::User));
    let result = config.deserialize::<InternalTagSnakeCase>();
    assert!(
        result.is_ok(),
        "Should match internal tag with snake_case: {:?}",
        result
    );

    if let InternalTagSnakeCase::FirstVariant { value } = result.unwrap() {
        assert_eq!(value, "hello");
    } else {
        panic!("Expected FirstVariant");
    }
}

// =============================================================================
// rename_all with serialization
// =============================================================================

#[test]
fn test_rename_all_serialization_snake_case() {
    let value = SnakeCaseEnum::MyLongVariantName(SimpleConfig {
        value: Some("test".to_string()),
    });
    let json = feuilletage::to_json_compact(&value).unwrap();

    // Should serialize with snake_case key
    assert!(
        json.contains("my_long_variant_name"),
        "Should serialize with snake_case: {}",
        json
    );
}

#[test]
fn test_rename_all_serialization_kebab_case() {
    let value = KebabCaseEnum::MyLongVariantName(SimpleConfig {
        value: Some("test".to_string()),
    });
    let json = feuilletage::to_json_compact(&value).unwrap();

    // Should serialize with kebab-case key
    assert!(
        json.contains("my-long-variant-name"),
        "Should serialize with kebab-case: {}",
        json
    );
}

#[test]
fn test_rename_all_serialization_screaming_snake_case() {
    let value = ScreamingSnakeCaseEnum::MyLongVariantName(SimpleConfig {
        value: Some("test".to_string()),
    });
    let json = feuilletage::to_json_compact(&value).unwrap();

    // Should serialize with SCREAMING_SNAKE_CASE key
    assert!(
        json.contains("MY_LONG_VARIANT_NAME"),
        "Should serialize with SCREAMING_SNAKE_CASE: {}",
        json
    );
}

// =============================================================================
// rename_all round-trip tests
// =============================================================================

#[test]
fn test_rename_all_round_trip() {
    let original = KebabCaseEnum::MyLongVariantName(SimpleConfig {
        value: Some("round-trip".to_string()),
    });
    let json = feuilletage::to_json_compact(&original).unwrap();

    let mut config = Config::default();
    config.load_json(&json, Context::new(Source::Programmatic, Level::User));
    let restored: KebabCaseEnum = config.deserialize().unwrap();

    assert_eq!(original, restored);
}

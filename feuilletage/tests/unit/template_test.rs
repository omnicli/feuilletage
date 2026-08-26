//! Unit tests for template module (template interpolation).
//!
//! Extracted from feuilletage/src/template.rs

use feuilletage::template::{
    extract_field_references, interpolate_template, topological_sort, value_to_string,
};
use feuilletage::{Context, ContextValue, Level, Source};
use std::collections::{HashMap, HashSet};

#[test]
fn test_extract_field_references() {
    assert_eq!(
        extract_field_references("http://%{host}:%{port}/api"),
        vec!["host", "port"]
    );

    assert_eq!(
        extract_field_references("no references here"),
        Vec::<String>::new()
    );

    assert_eq!(
        extract_field_references("%%{escaped}"),
        Vec::<String>::new()
    );

    assert_eq!(
        extract_field_references("%{field1} and %{field2} and %{field1}"),
        vec!["field1", "field2"] // No duplicates
    );
}

#[test]
fn test_value_to_string() {
    let ctx = Context::new(Source::Default, Level::User);

    assert_eq!(
        value_to_string(&ContextValue::string("hello".to_string(), ctx.clone()), ","),
        "hello"
    );
    assert_eq!(
        value_to_string(&ContextValue::int(42, ctx.clone()), ","),
        "42"
    );
    assert_eq!(
        value_to_string(&ContextValue::float(3.14, ctx.clone()), ","),
        "3.14"
    );
    assert_eq!(
        value_to_string(&ContextValue::bool(true, ctx.clone()), ","),
        "true"
    );
    assert_eq!(
        value_to_string(&ContextValue::bool(false, ctx.clone()), ","),
        "false"
    );
    assert_eq!(value_to_string(&ContextValue::null(ctx.clone()), ","), "");

    let arr = ContextValue::array(
        vec![
            ContextValue::string("a".to_string(), ctx.clone()),
            ContextValue::string("b".to_string(), ctx.clone()),
            ContextValue::string("c".to_string(), ctx.clone()),
        ],
        ctx.clone(),
    );
    assert_eq!(value_to_string(&arr, ","), "a,b,c");
    assert_eq!(value_to_string(&arr, ";"), "a;b;c");
}

#[test]
fn test_interpolate_template() {
    let mut values = HashMap::new();
    values.insert("host".to_string(), "localhost".to_string());
    values.insert("port".to_string(), "8080".to_string());

    assert_eq!(
        interpolate_template("http://%{host}:%{port}/api", &values, ",").unwrap(),
        "http://localhost:8080/api"
    );

    // Test escape sequence
    assert_eq!(
        interpolate_template("echo %%{HOME}", &values, ",").unwrap(),
        "echo %{HOME}"
    );

    // Test missing field (left as-is)
    assert_eq!(
        interpolate_template("%{missing}", &values, ",").unwrap(),
        "%{missing}"
    );
}

#[test]
fn test_topological_sort_simple() {
    let mut template_fields = HashMap::new();
    let mut all_fields = HashSet::new();

    all_fields.insert("host".to_string());
    all_fields.insert("port".to_string());
    all_fields.insert("url".to_string());

    template_fields.insert(
        "url".to_string(),
        vec!["host".to_string(), "port".to_string()],
    );

    let sorted = topological_sort(&template_fields, &all_fields).unwrap();

    // url should come after host and port
    let url_idx = sorted.iter().position(|x| x == "url").unwrap();
    let host_idx = sorted.iter().position(|x| x == "host").unwrap();
    let port_idx = sorted.iter().position(|x| x == "port").unwrap();

    assert!(url_idx > host_idx);
    assert!(url_idx > port_idx);
}

#[test]
fn test_topological_sort_chain() {
    let mut template_fields = HashMap::new();
    let mut all_fields = HashSet::new();

    all_fields.insert("a".to_string());
    all_fields.insert("b".to_string());
    all_fields.insert("c".to_string());

    template_fields.insert("b".to_string(), vec!["a".to_string()]);
    template_fields.insert("c".to_string(), vec!["b".to_string()]);

    let sorted = topological_sort(&template_fields, &all_fields).unwrap();

    // Order should be a, b, c
    let a_idx = sorted.iter().position(|x| x == "a").unwrap();
    let b_idx = sorted.iter().position(|x| x == "b").unwrap();
    let c_idx = sorted.iter().position(|x| x == "c").unwrap();

    assert!(a_idx < b_idx);
    assert!(b_idx < c_idx);
}

#[test]
fn test_topological_sort_cycle() {
    use feuilletage::template::TemplateError;

    let mut template_fields = HashMap::new();
    let mut all_fields = HashSet::new();

    all_fields.insert("a".to_string());
    all_fields.insert("b".to_string());

    template_fields.insert("a".to_string(), vec!["b".to_string()]);
    template_fields.insert("b".to_string(), vec!["a".to_string()]);

    let result = topological_sort(&template_fields, &all_fields);
    assert!(matches!(
        result,
        Err(TemplateError::CircularDependency { .. })
    ));
}

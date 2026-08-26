//! Template interpolation examples
//!
//! These examples demonstrate the #[feuilletage(template)] attribute for field reference interpolation.
//!
//! Demonstrated patterns:
//! - Basic template interpolation with %{field} syntax
//! - Multiple field references in a single template
//! - Type coercion (non-string fields converted to string)
//! - Escape sequences (%%{ for literal %{)
//! - Template chains (field A depends on field B which depends on field C)
//! - Vec fields with custom delimiters
//! - Option fields (None -> empty string)

#![allow(clippy::approx_constant)]

use feuilletage::{Context, Level, Source};

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

fn main() {
    println!("=== Template Interpolation Examples ===\n");

    // Basic template interpolation
    println!("--- Basic Template Interpolation ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ServerConfig {
        host: String,
        #[feuilletage(default = "8080")]
        port: i32,
        #[feuilletage(template)]
        url: String,
    }

    let mut config = feuilletage::Config::default();
    config.load_json(
        r#"{
            "host": "localhost",
            "port": 3000,
            "url": "http://%{host}:%{port}/api"
        }"#,
        test_context(),
    );

    let server: ServerConfig = config.deserialize().unwrap();
    println!("host: {}", server.host);
    println!("port: {}", server.port);
    println!("url: {} (interpolated)", server.url);
    assert_eq!(server.host, "localhost");
    assert_eq!(server.port, 3000);
    assert_eq!(server.url, "http://localhost:3000/api");

    // Template with default
    println!("\n--- Template with Default ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ApiConfig {
        base_url: String,
        api_version: String,
        #[feuilletage(template, default = "%{base_url}/api/v%{api_version}")]
        api_endpoint: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "base_url": "https://example.com",
            "api_version": "2"
        }"#,
        test_context(),
    );

    let result: ApiConfig = cfg.deserialize().unwrap();
    println!("base_url: {}", result.base_url);
    println!("api_version: {}", result.api_version);
    println!(
        "api_endpoint: {} (from default template)",
        result.api_endpoint
    );
    assert_eq!(result.api_endpoint, "https://example.com/api/v2");

    // Type coercion in templates
    println!("\n--- Type Coercion in Templates ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct VersionConfig {
        major: i32,
        minor: i32,
        patch: i32,
        #[feuilletage(template)]
        version: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "major": 1,
            "minor": 2,
            "patch": 3,
            "version": "v%{major}.%{minor}.%{patch}"
        }"#,
        test_context(),
    );

    let result: VersionConfig = cfg.deserialize().unwrap();
    println!("version: {} (integers coerced to string)", result.version);
    assert_eq!(result.version, "v1.2.3");

    // Boolean field
    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct StatusConfig {
        name: String,
        enabled: bool,
        #[feuilletage(template)]
        status: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "name": "feature_x",
            "enabled": true,
            "status": "Feature %{name} is %{enabled}"
        }"#,
        test_context(),
    );

    let result: StatusConfig = cfg.deserialize().unwrap();
    println!("status: {} (bool coerced)", result.status);
    assert_eq!(result.status, "Feature feature_x is true");

    // Float field
    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct PiConfig {
        value: f64,
        #[feuilletage(template)]
        message: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "value": 3.14,
            "message": "Pi is approximately %{value}"
        }"#,
        test_context(),
    );

    let result: PiConfig = cfg.deserialize().unwrap();
    println!("message: {} (float coerced)", result.message);
    assert_eq!(result.message, "Pi is approximately 3.14");

    // Escape sequences
    println!("\n--- Escape Sequences ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct EscapeConfig {
        name: String,
        #[feuilletage(template)]
        shell_command: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "name": "test",
            "shell_command": "echo %%{HOME} with %{name}"
        }"#,
        test_context(),
    );

    let result: EscapeConfig = cfg.deserialize().unwrap();
    println!("shell_command: {}", result.shell_command);
    println!("Note: %%{{ becomes %{{ (escaped)");
    assert_eq!(result.shell_command, "echo %{HOME} with test");

    // Template chains
    println!("\n--- Template Chains ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct ChainConfig {
        scheme: String,
        host: String,
        port: i32,
        #[feuilletage(template)]
        base_url: String,
        #[feuilletage(template)]
        api_url: String,
        #[feuilletage(template)]
        health_url: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "scheme": "https",
            "host": "api.example.com",
            "port": 443,
            "base_url": "%{scheme}://%{host}:%{port}",
            "api_url": "%{base_url}/v1",
            "health_url": "%{api_url}/health"
        }"#,
        test_context(),
    );

    let result: ChainConfig = cfg.deserialize().unwrap();
    println!("base_url: {}", result.base_url);
    println!("api_url: {} (depends on base_url)", result.api_url);
    println!("health_url: {} (depends on api_url)", result.health_url);
    assert_eq!(result.base_url, "https://api.example.com:443");
    assert_eq!(result.api_url, "https://api.example.com:443/v1");
    assert_eq!(result.health_url, "https://api.example.com:443/v1/health");

    // Option fields
    println!("\n--- Option Fields ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct OptionalConfig {
        prefix: Option<String>,
        #[feuilletage(template)]
        message: String,
    }

    // With None
    let mut cfg = feuilletage::Config::default();
    cfg.load_json(r#"{"message": "[%{prefix}] Hello"}"#, test_context());

    let result: OptionalConfig = cfg.deserialize().unwrap();
    println!("prefix: {:?}", result.prefix);
    println!("message: {} (None -> empty string)", result.message);
    assert_eq!(result.prefix, None);
    assert_eq!(result.message, "[] Hello");

    // With Some
    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{"prefix": "INFO", "message": "[%{prefix}] Hello"}"#,
        test_context(),
    );

    let result: OptionalConfig = cfg.deserialize().unwrap();
    println!("prefix: {:?}", result.prefix);
    println!("message: {}", result.message);
    assert_eq!(result.prefix, Some("INFO".to_string()));
    assert_eq!(result.message, "[INFO] Hello");

    // Vec fields
    println!("\n--- Vec Fields ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct TagsConfig {
        #[feuilletage(allow_single)]
        tags: Vec<String>,
        #[feuilletage(template)]
        tag_string: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "tags": ["web", "api", "v2"],
            "tag_string": "Tags: %{tags}"
        }"#,
        test_context(),
    );

    let result: TagsConfig = cfg.deserialize().unwrap();
    println!("tags: {:?}", result.tags);
    println!("tag_string: {} (joined with ',')", result.tag_string);
    assert_eq!(result.tags, vec!["web", "api", "v2"]);
    assert_eq!(result.tag_string, "Tags: web,api,v2");

    // No template references
    println!("\n--- No Template References ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct StaticConfig {
        name: String,
        #[feuilletage(template)]
        static_value: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "name": "test",
            "static_value": "just a static string"
        }"#,
        test_context(),
    );

    let result: StaticConfig = cfg.deserialize().unwrap();
    println!("static_value: {} (no references)", result.static_value);
    assert_eq!(result.static_value, "just a static string");

    // Real-world: Database connection string
    println!("\n--- Real-World: Database Connection String ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct DatabaseConfig {
        driver: String,
        host: String,
        port: i32,
        database: String,
        username: String,
        #[feuilletage(secret)]
        password: String,
        #[feuilletage(template)]
        connection_string: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "driver": "postgres",
            "host": "db.example.com",
            "port": 5432,
            "database": "myapp",
            "username": "admin",
            "password": "secret123",
            "connection_string": "%{driver}://%{username}:%{password}@%{host}:%{port}/%{database}"
        }"#,
        test_context(),
    );

    let result: DatabaseConfig = cfg.deserialize().unwrap();
    println!("connection_string: {}", result.connection_string);
    assert_eq!(
        result.connection_string,
        "postgres://admin:secret123@db.example.com:5432/myapp"
    );

    // Real-world: Path construction
    println!("\n--- Real-World: Path Construction ---");

    #[derive(Debug, feuilletage::Config, PartialEq)]
    struct PathConfig {
        home_dir: String,
        #[feuilletage(default = ".config/myapp")]
        config_subdir: String,
        #[feuilletage(template)]
        config_dir: String,
        #[feuilletage(template)]
        cache_dir: String,
    }

    let mut cfg = feuilletage::Config::default();
    cfg.load_json(
        r#"{
            "home_dir": "/home/user",
            "config_dir": "%{home_dir}/%{config_subdir}",
            "cache_dir": "%{home_dir}/.cache/myapp"
        }"#,
        test_context(),
    );

    let result: PathConfig = cfg.deserialize().unwrap();
    println!("config_dir: {}", result.config_dir);
    println!("cache_dir: {}", result.cache_dir);
    assert_eq!(result.config_dir, "/home/user/.config/myapp");
    assert_eq!(result.cache_dir, "/home/user/.cache/myapp");

    // Template library functions
    println!("\n--- Template Library Functions ---");

    use feuilletage::extract_field_references;

    let refs = extract_field_references("http://%{host}:%{port}/api");
    println!(
        "extract_field_references(\"http://%{{host}}:%{{port}}/api\"): {:?}",
        refs
    );
    assert_eq!(refs, vec!["host", "port"]);

    let refs = extract_field_references("no references here");
    println!(
        "extract_field_references(\"no references here\"): {:?}",
        refs
    );
    assert_eq!(refs, Vec::<String>::new());

    let refs = extract_field_references("%%{escaped}");
    println!(
        "extract_field_references(\"%%{{escaped}}\"): {:?} (escaped)",
        refs
    );
    assert_eq!(refs, Vec::<String>::new());

    let refs = extract_field_references("%{field} and %{other} and %{field}");
    println!(
        "extract_field_references(\"%{{field}} and %{{other}} and %{{field}}\"): {:?} (deduped)",
        refs
    );
    assert_eq!(refs, vec!["field", "other"]);

    // value_to_string function
    use feuilletage::{value_to_string, ContextValue};

    let ctx = test_context();

    println!("\nvalue_to_string examples:");
    println!(
        "  string \"hello\": {}",
        value_to_string(&ContextValue::string("hello".to_string(), ctx.clone()), ",")
    );
    println!(
        "  int 42: {}",
        value_to_string(&ContextValue::int(42, ctx.clone()), ",")
    );
    println!(
        "  float 3.14: {}",
        value_to_string(&ContextValue::float(3.14, ctx.clone()), ",")
    );
    println!(
        "  bool true: {}",
        value_to_string(&ContextValue::bool(true, ctx.clone()), ",")
    );
    println!(
        "  null: \"{}\"",
        value_to_string(&ContextValue::null(ctx.clone()), ",")
    );

    let arr = ContextValue::array(
        vec![
            ContextValue::string("a".to_string(), ctx.clone()),
            ContextValue::string("b".to_string(), ctx.clone()),
        ],
        ctx.clone(),
    );
    println!(
        "  array [\"a\", \"b\"] with \",\": {}",
        value_to_string(&arr, ",")
    );
    println!(
        "  array [\"a\", \"b\"] with \";\": {}",
        value_to_string(&arr, ";")
    );

    println!("\n=== All template interpolation examples passed! ===");
}

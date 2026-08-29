//! Reference for every `#[derive(feuilletage::Config)]` attribute, with a
//! runnable example for each.
//!
//! This module contains no code — it exists solely so that `cargo doc`
//! surfaces a single browsable attribute index and `cargo test --doc`
//! exercises every documented example.
//!
//! # Attribute Reference
//!
//! ## Default Values
//!
//! Fields are required by default. Use `default`, `default = "value"`, or `default_fn` to make them optional.
//!
//! ### `default` - Use type's Default implementation
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct AppConfig {
//!     #[feuilletage(default)]
//!     count: i32,        // Defaults to 0
//!     #[feuilletage(default)]
//!     enabled: bool,     // Defaults to false
//!     #[feuilletage(default)]
//!     items: Vec<String>, // Defaults to empty vec
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(r#"{}"#, Context::new(Source::Programmatic, Level::User));
//!
//! let app: AppConfig = config.deserialize().unwrap();
//! assert_eq!(app.count, 0);
//! assert_eq!(app.enabled, false);
//! assert!(app.items.is_empty());
//! # }
//! ```
//!
//! ### `default = "value"` - Explicit default value
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct ServerConfig {
//!     #[feuilletage(default = "localhost")]
//!     host: String,
//!     #[feuilletage(default = "8080")]
//!     port: i32,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(r#"{}"#, Context::new(Source::Programmatic, Level::User));
//!
//! let server: ServerConfig = config.deserialize().unwrap();
//! assert_eq!(server.host, "localhost");
//! assert_eq!(server.port, 8080);
//! # }
//! ```
//!
//! ### `default_fn = "function"` - Call function for default
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! fn default_timeout() -> i64 { 30 }
//! fn generate_id() -> String { "auto-generated".to_string() }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct TimeoutConfig {
//!     #[feuilletage(default_fn = "default_timeout")]
//!     timeout: i64,
//!     #[feuilletage(default_fn = "generate_id")]
//!     request_id: String,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(r#"{}"#, Context::new(Source::Programmatic, Level::User));
//!
//! let tc: TimeoutConfig = config.deserialize().unwrap();
//! assert_eq!(tc.timeout, 30);
//! assert_eq!(tc.request_id, "auto-generated");
//! # }
//! ```
//!
//! ## Field Naming
//!
//! ### `rename = "key"` - Use different key name
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct ApiConfig {
//!     #[feuilletage(rename = "userName")]
//!     user_name: String,
//!     #[feuilletage(rename = "apiKey")]
//!     api_key: String,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"userName": "john", "apiKey": "secret123"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let api: ApiConfig = config.deserialize().unwrap();
//! assert_eq!(api.user_name, "john");
//! assert_eq!(api.api_key, "secret123");
//! # }
//! ```
//!
//! ### `aliases = ["alt1", "alt2"]` - Accept alternative key names
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct CountConfig {
//!     #[feuilletage(aliases = ["count", "n", "num"])]
//!     item_count: i32,
//! }
//!
//! // All these keys work:
//! for key in &["item_count", "count", "n", "num"] {
//!     let mut config = Config::default();
//!     config.load_yaml(
//!         &format!(r#"{{"{key}": 42}}"#),
//!         Context::new(Source::Programmatic, Level::User),
//!     );
//!     let c: CountConfig = config.deserialize().unwrap();
//!     assert_eq!(c.item_count, 42);
//! }
//! # }
//! ```
//!
//! ## Validation
//!
//! ### `range(min, max)` - Numeric range validation
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct RangeConfig {
//!     #[feuilletage(range(0, 100))]
//!     percentage: i32,
//!     #[feuilletage(range(1, 65535))]
//!     port: i32,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"percentage": 75, "port": 8080}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let rc: RangeConfig = config.deserialize().unwrap();
//! assert_eq!(rc.percentage, 75);
//! assert_eq!(rc.port, 8080);
//!
//! // Out of range values cause errors
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"percentage": 150, "port": 8080}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! assert!(config2.deserialize::<RangeConfig>().is_err());
//! # }
//! ```
//!
//! ### `length(min, max)` - String/array length validation
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct LengthConfig {
//!     #[feuilletage(length(3, 20))]
//!     username: String,
//!     #[feuilletage(length(1, 10))]
//!     tags: Vec<String>,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"username": "john_doe", "tags": ["rust", "config"]}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let lc: LengthConfig = config.deserialize().unwrap();
//! assert_eq!(lc.username, "john_doe");
//! assert_eq!(lc.tags, vec!["rust", "config"]);
//!
//! // Too short username causes error
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"username": "ab", "tags": ["ok"]}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! assert!(config2.deserialize::<LengthConfig>().is_err());
//! # }
//! ```
//!
//! ### `validate = "function"` - Custom validation function
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! fn validate_even(value: &i32) -> Result<(), String> {
//!     if value % 2 == 0 {
//!         Ok(())
//!     } else {
//!         Err(format!("{} is not even", value))
//!     }
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct EvenConfig {
//!     #[feuilletage(validate = "validate_even")]
//!     count: i32,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"count": 42}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let ec: EvenConfig = config.deserialize().unwrap();
//! assert_eq!(ec.count, 42);
//!
//! // Odd number causes validation error
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"count": 7}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! assert!(config2.deserialize::<EvenConfig>().is_err());
//! # }
//! ```
//!
//! ## Type Coercion
//!
//! ### `coerce` - Enable liberal type conversion
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct CoerceConfig {
//!     #[feuilletage(coerce)]
//!     port: i32,      // Accepts "8080" as string
//!     #[feuilletage(coerce)]
//!     enabled: bool,  // Accepts "true", 1, "yes"
//!     #[feuilletage(coerce)]
//!     ratio: f64,     // Accepts "3.14" as string
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"port": "8080", "enabled": "true", "ratio": "3.14"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let cc: CoerceConfig = config.deserialize().unwrap();
//! assert_eq!(cc.port, 8080);
//! assert_eq!(cc.enabled, true);
//! assert!((cc.ratio - 3.14).abs() < 0.001);
//! # }
//! ```
//!
//! ## Flexible Input Handling
//!
//! ### `allow_single` - Accept single value as array
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct TagConfig {
//!     #[feuilletage(allow_single)]
//!     tags: Vec<String>,
//! }
//!
//! // Single value is wrapped in array
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"tags": "single-tag"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let tc: TagConfig = config.deserialize().unwrap();
//! assert_eq!(tc.tags, vec!["single-tag"]);
//!
//! // Array still works
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"tags": ["tag1", "tag2"]}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let tc2: TagConfig = config2.deserialize().unwrap();
//! assert_eq!(tc2.tags, vec!["tag1", "tag2"]);
//! # }
//! ```
//!
//! ### `allow_map` - Accept map notation for Vec
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! struct Package {
//!     name: String,
//!     #[feuilletage(default = "latest")]
//!     version: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct DepsConfig {
//!     #[feuilletage(allow_map(key = "name", scalar_as = "version"))]
//!     packages: Vec<Package>,
//! }
//!
//! // Map notation: {"curl": "7.0"} -> [{name: "curl", version: "7.0"}]
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"packages": {"curl": "7.0", "jq": "1.6"}}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let dc: DepsConfig = config.deserialize().unwrap();
//! assert_eq!(dc.packages.len(), 2);
//! assert_eq!(dc.packages[0].name, "curl");
//! assert_eq!(dc.packages[0].version, "7.0");
//! # }
//! ```
//!
//! ## Struct-Level Attributes
//!
//! ### `scalar_as` and `array_as` - Wrap input as object
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! #[feuilletage(scalar_as = "file", array_as = "packages")]
//! struct NixSpec {
//!     file: Option<String>,
//!     packages: Option<Vec<String>>,
//! }
//!
//! // Scalar -> {file: value}
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#""shell.nix""#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let ns: NixSpec = config.deserialize().unwrap();
//! assert_eq!(ns.file, Some("shell.nix".to_string()));
//! assert_eq!(ns.packages, None);
//!
//! // Array -> {packages: value}
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"["pkg1", "pkg2"]"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let ns2: NixSpec = config2.deserialize().unwrap();
//! assert_eq!(ns2.file, None);
//! assert_eq!(ns2.packages, Some(vec!["pkg1".to_string(), "pkg2".to_string()]));
//! # }
//! ```
//!
//! ## Enum Support
//!
//! ### `tag = "field"` - Internally tagged enums
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(tag = "type")]
//! enum Message {
//!     Text { content: String },
//!     Image { url: String },
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"type": "text", "content": "Hello!"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let msg: Message = config.deserialize().unwrap();
//! assert!(matches!(msg, Message::Text { content } if content == "Hello!"));
//!
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"type": "image", "url": "https://example.com/img.png"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let msg2: Message = config2.deserialize().unwrap();
//! assert!(matches!(msg2, Message::Image { url } if url == "https://example.com/img.png"));
//! # }
//! ```
//!
//! ### `untagged` - Untagged enums (try each variant)
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(untagged)]
//! enum FlexibleValue {
//!     Simple(String),
//!     Detailed { name: String, value: i32 },
//! }
//!
//! // String input -> Simple variant
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#""hello""#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let val: FlexibleValue = config.deserialize().unwrap();
//! assert!(matches!(val, FlexibleValue::Simple(s) if s == "hello"));
//!
//! // Object input -> Detailed variant
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"name": "test", "value": 42}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let val2: FlexibleValue = config2.deserialize().unwrap();
//! assert!(matches!(val2, FlexibleValue::Detailed { name, value } if name == "test" && value == 42));
//! # }
//! ```
//!
//! ### `external_tag` - Externally tagged enums (map key determines variant)
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(scalar_as = "version")]
//! struct ToolVersion {
//!     version: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(external_tag)]
//! enum Tool {
//!     Python(ToolVersion),
//!     Node(ToolVersion),
//! }
//!
//! // Map key determines the variant: {"python": "3.11"} -> Tool::Python
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"python": "3.11"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let tool: Tool = config.deserialize().unwrap();
//! assert!(matches!(tool, Tool::Python(tv) if tv.version == "3.11"));
//!
//! // Another variant: {"node": "20.0"}
//! let mut config2 = Config::default();
//! config2.load_yaml(
//!     r#"{"node": "20.0"}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let tool2: Tool = config2.deserialize().unwrap();
//! assert!(matches!(tool2, Tool::Node(tv) if tv.version == "20.0"));
//! # }
//! ```
//!
//! ## Flatten
//!
//! ### `flatten` - Flatten nested struct fields
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, Level, Source, FromContextValue};
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct DatabaseConfig {
//!     host: String,
//!     port: i32,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct AppConfig {
//!     name: String,
//!     #[feuilletage(flatten)]
//!     database: DatabaseConfig,
//! }
//!
//! // Database fields are at the top level
//! let mut config = Config::default();
//! config.load_yaml(
//!     r#"{"name": "myapp", "host": "localhost", "port": 5432}"#,
//!     Context::new(Source::Programmatic, Level::User),
//! );
//!
//! let app: AppConfig = config.deserialize().unwrap();
//! assert_eq!(app.name, "myapp");
//! assert_eq!(app.database.host, "localhost");
//! assert_eq!(app.database.port, 5432);
//! # }
//! ```
//!
//! ## Complete Field Attribute Example
//!
//! This example exercises the remaining field-level input, transform,
//! validation, metadata, context, fallback, and error-handling attributes.
//! `alias` is the singular form of `aliases`; both are accepted.
//!
//! ```
//! # #[cfg(all(feature = "std", feature = "yaml"))] {
//! use std::collections::HashMap;
//! use std::path::PathBuf;
//! use feuilletage::{Config, Context, Error, Level, Source};
//!
//! fn double(value: &mut i32) -> Result<(), Error> {
//!     *value *= 2;
//!     Ok(())
//! }
//! fn uppercase(value: &mut String) -> Result<(), Error> {
//!     *value = value.to_uppercase();
//!     Ok(())
//! }
//! fn level_name<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
//!     context: &Context<S, L>,
//! ) -> String {
//!     context.level.name().to_string()
//! }
//! fn absolute_path() -> PathBuf { PathBuf::from("/tmp/app") }
//! fn relative_path() -> PathBuf { PathBuf::from(".") }
//! fn normalized_path() -> PathBuf { PathBuf::from("a/../b") }
//!
//! #[derive(Debug, Default, feuilletage::Config)]
//! #[feuilletage(mutable_by = ["user"])]
//! struct NestedOptions {
//!     #[feuilletage(default)]
//!     value: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct FieldOptions {
//!     #[feuilletage(alias = "old_name", aliases = ["legacy_name"])]
//!     name: String,
//!     #[feuilletage(default, allow_list)]
//!     labels: HashMap<String, String>,
//!     #[feuilletage(default, transform = "trim", transform_after = "uppercase")]
//!     title: String,
//!     #[feuilletage(default, allow_single, transform_each = "trim", transform_each_after = "uppercase")]
//!     tags: Vec<String>,
//!     #[feuilletage(default, on_error = default, transform_after = "double")]
//!     retries: i32,
//!     #[feuilletage(duration)]
//!     timeout: u64,
//!     #[feuilletage(default, mutable_by = ["user"])]
//!     mutable: String,
//!     #[feuilletage(default, nested)]
//!     nested_options: NestedOptions,
//!     #[feuilletage(default_fn = "absolute_path", absolute_path)]
//!     absolute: PathBuf,
//!     #[feuilletage(default_fn = "relative_path", relative_path)]
//!     relative: PathBuf,
//!     #[feuilletage(default_fn = "normalized_path", normalize_path)]
//!     normalized: PathBuf,
//!     #[feuilletage(default, env = "FEUILLETAGE_DOCS_MISSING_ENV")]
//!     env_value: String,
//!     #[feuilletage(default, deprecated = "use name", secret)]
//!     old_secret: String,
//!     #[feuilletage(from_context = "level.name")]
//!     scope: String,
//!     #[feuilletage(from_context_fn = "level_name")]
//!     computed_scope: String,
//!     #[feuilletage(default = "stable")]
//!     channel: String,
//!     #[feuilletage(fallback = "channel")]
//!     mirror: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct TemplateOptions {
//!     name: String,
//!     #[feuilletage(template(refs = ["name"]))]
//!     greeting: String,
//! }
//!
//! let mut config = Config::default();
//! config.load_yaml(
//!     "old_name: demo\ntitle: ' hello '\ntags: [' one ', two]\nretries: 2\ntimeout: 2s\nmutable: editable\n",
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let value: FieldOptions = config.deserialize().unwrap();
//! assert_eq!(value.name, "demo");
//! assert_eq!(value.title, "HELLO");
//! assert_eq!(value.tags, ["ONE", "TWO"]);
//! assert_eq!(value.retries, 4);
//! assert_eq!(value.timeout, 2);
//! assert_eq!(value.mutable, "editable");
//! assert_eq!(value.scope, "user");
//! assert_eq!(value.computed_scope, "user");
//! assert_eq!(value.mirror, "stable");
//!
//! let mut templates = Config::default();
//! templates.load_yaml(
//!     "name: demo\ngreeting: 'hello %{name}'\n",
//!     Context::new(Source::Programmatic, Level::User),
//! );
//! let templates: TemplateOptions = templates.deserialize().unwrap();
//! assert_eq!(templates.greeting, "hello demo");
//! # }
//! ```
//!
//! Feature-backed validators are executable when their corresponding feature
//! is enabled:
//!
//! ```
//! #[cfg(feature = "regex")]
//! #[derive(feuilletage::Config)]
//! struct RegexConfig {
//!     #[feuilletage(regex = "^[a-z]+$")]
//!     name: String,
//! }
//!
//! #[cfg(feature = "chrono")]
//! #[derive(feuilletage::Config)]
//! struct DateConfig {
//!     #[feuilletage(datetime = "%Y-%m-%d")]
//!     date: String,
//! }
//! ```
//!
//! ## Serialization Attributes
//!
//! `skip` omits a field in both directions. The `skip_if_empty`,
//! `skip_if_empty_recursive`, `skip_if_default`, and `skip_if` forms only
//! control serialization. `serialize_single_as_value` unwraps a one-item
//! vector.
//!
//! ```
//! fn is_zero(value: &i32) -> bool { *value == 0 }
//!
//! #[derive(feuilletage::Config)]
//! struct OutputOptions {
//!     name: String,
//!     #[feuilletage(skip, default)]
//!     internal: bool,
//!     #[feuilletage(skip_if_empty)]
//!     empty: Vec<String>,
//!     #[feuilletage(skip_if_empty_recursive)]
//!     nested_empty: Option<String>,
//!     #[feuilletage(default, skip_if_default)]
//!     enabled: bool,
//!     #[feuilletage(default, skip_if = "is_zero")]
//!     count: i32,
//!     #[feuilletage(serialize_single_as_value)]
//!     tags: Vec<String>,
//! }
//!
//! let value = OutputOptions {
//!     name: "demo".into(),
//!     internal: true,
//!     empty: vec![],
//!     nested_empty: Some(String::new()),
//!     enabled: false,
//!     count: 0,
//!     tags: vec!["one".into()],
//! };
//! let json = serde_json::to_value(value).unwrap();
//! assert_eq!(json, serde_json::json!({"name": "demo", "tags": "one"}));
//! ```
//!
//! ## Container Attributes
//!
//! Containers support raw `transform`, `allow_map`, `transparent`,
//! `post_process`, `parse_as`, `skip_serialize`, and `skip_deserialize`. The
//! two skip attributes suppress only the named generated serde implementation;
//! the core `FromContextValue` implementation remains available.
//!
//! `parse_as = "WireType"` replaces the target's normal deserialization path.
//! Feuilletage parses `WireType` through its `FromContextValue`
//! implementation, then calls `FromParsed<WireType, S, L>` on the target. The
//! projection receives the parsed wire value, the untouched original
//! `ContextValue`, and the same path-aware `ErrorTracker`. Serialization is
//! still generated from the target type and is independent of the wire type.
//!
//! ```
//! use feuilletage::{Context, ContextValue, CustomLevel, CustomSource, Error, ErrorTracker, FromParsed};
//!
//! fn finish<S: CustomSource, L: CustomLevel>(
//!     value: &mut Wrapped,
//!     _source: &ContextValue<S, L>,
//!     _tracker: &mut ErrorTracker,
//! ) -> Result<(), Error> {
//!     value.0.push('!');
//!     Ok(())
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! #[feuilletage(transparent, transform = "trim", post_process = "finish", skip_serialize, skip_deserialize)]
//! struct Wrapped(String);
//!
//! #[derive(Debug, feuilletage::Config)]
//! #[feuilletage(allow_map(key = "name", scalar_as = "version"))]
//! struct Package {
//!     name: String,
//!     version: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! struct PackageWire {
//!     name: String,
//!     version: String,
//! }
//!
//! #[derive(Debug, feuilletage::Config)]
//! #[feuilletage(parse_as = "PackageWire")]
//! struct PackageLabel {
//!     label: String,
//! }
//!
//! impl<S: CustomSource, L: CustomLevel> FromParsed<PackageWire, S, L> for PackageLabel {
//!     fn from_parsed(
//!         parsed: PackageWire,
//!         _original: &ContextValue<S, L>,
//!         _tracker: &mut ErrorTracker,
//!     ) -> Result<Self, Error> {
//!         Ok(Self { label: format!("{}@{}", parsed.name, parsed.version) })
//!     }
//! }
//!
//! let source = ContextValue::string(" demo ", Context::default());
//! let mut tracker = ErrorTracker::new();
//! let wrapped = <Wrapped as feuilletage::FromContextValue>::from_context_value(
//!     &source,
//!     &mut tracker,
//! ).unwrap();
//! assert_eq!(wrapped.0, "demo!");
//! assert_eq!(<Package as feuilletage::AllowMapKeys>::map_key_fields(), ["name"]);
//! let mut fields = feuilletage::OrderedMap::default();
//! fields.insert(
//!     "name".into(),
//!     ContextValue::string("demo", Context::default()),
//! );
//! fields.insert(
//!     "version".into(),
//!     ContextValue::string("1.0", Context::default()),
//! );
//! let source = ContextValue::object(fields, Context::default());
//! let label = <PackageLabel as feuilletage::FromContextValue>::from_context_value(
//!     &source,
//!     &mut ErrorTracker::new(),
//! ).unwrap();
//! assert_eq!(label.label, "demo@1.0");
//! ```
//!
//! A projection can accept multiple input shapes by using an untagged enum as
//! its wire type. Variants are attempted in declaration order.
//!
//! ```
//! use feuilletage::{
//!     Context, ContextValue, CustomLevel, CustomSource, Error, ErrorTracker,
//!     FromContextValue, FromParsed, OrderedMap,
//! };
//!
//! #[derive(Debug, feuilletage::Config)]
//! #[feuilletage(untagged)]
//! enum EndpointWire {
//!     Url(String),
//!     Parts { host: String, port: u16 },
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(parse_as = "EndpointWire")]
//! struct Endpoint {
//!     url: String,
//! }
//!
//! impl<S: CustomSource, L: CustomLevel> FromParsed<EndpointWire, S, L> for Endpoint {
//!     fn from_parsed(
//!         parsed: EndpointWire,
//!         _original: &ContextValue<S, L>,
//!         _tracker: &mut ErrorTracker,
//!     ) -> Result<Self, Error> {
//!         let url = match parsed {
//!             EndpointWire::Url(url) => url,
//!             EndpointWire::Parts { host, port } => format!("http://{host}:{port}"),
//!         };
//!         Ok(Self { url })
//!     }
//! }
//!
//! let scalar: ContextValue = ContextValue::string(
//!     "https://example.com",
//!     Context::default(),
//! );
//! let endpoint = Endpoint::from_context_value(&scalar, &mut ErrorTracker::new()).unwrap();
//! assert_eq!(endpoint.url, "https://example.com");
//!
//! let mut fields = OrderedMap::default();
//! fields.insert(
//!     "host".into(),
//!     ContextValue::string("localhost", Context::default()),
//! );
//! fields.insert(
//!     "port".into(),
//!     ContextValue::int(8080, Context::default()),
//! );
//! let object: ContextValue = ContextValue::object(fields, Context::default());
//! let endpoint = Endpoint::from_context_value(&object, &mut ErrorTracker::new()).unwrap();
//! assert_eq!(endpoint.url, "http://localhost:8080");
//! ```
//!
//! ## Enum and Variant Attributes
//!
//! Enum containers choose one of `tag`, `untagged`, `external_tag`, or
//! `value_matched`. `rename_all` applies a case convention. Variants accept
//! `rename`, `alias`, `aliases`, `fallback`, `from_tag`, `allow_single`,
//! legacy `null_variant`, legacy `scalar_variant` (also spelled `scalar`),
//! unified `variant` predicates, `variant_fn`, `variant_value`, and
//! `variant_default`.
//!
//! ```
//! # #[cfg(feature = "yaml")] {
//! use feuilletage::{Config, Context, ContextValue, Level, Source};
//!
//! fn is_auto<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>(
//!     value: &ContextValue<S, L>,
//! ) -> bool {
//!     matches!(value, ContextValue::String(text, _) if text == "automatic")
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(value_matched, rename_all = "snake_case")]
//! enum Mode {
//!     #[feuilletage(variant = "on" | true | 1)]
//!     On,
//!     #[feuilletage(variant = "auto", variant_fn = "is_auto")]
//!     Auto,
//!     #[feuilletage(fallback)]
//!     Ask,
//! }
//!
//! #[derive(Debug, Default, feuilletage::Config, PartialEq)]
//! #[feuilletage(scalar_as = "name")]
//! struct Named { name: String }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(external_tag)]
//! enum External {
//!     #[feuilletage(rename = "known", alias = "old", aliases = ["legacy"])]
//!     Known(Named),
//!     #[feuilletage(fallback, from_tag = "name")]
//!     Other(Named),
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(external_tag)]
//! enum LegacyScalar {
//!     #[feuilletage(null_variant)]
//!     Empty,
//!     #[feuilletage(scalar_variant)]
//!     Text(String),
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(external_tag)]
//! enum ScalarAlias {
//!     #[feuilletage(scalar)]
//!     Text(String),
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(external_tag)]
//! enum Constructed {
//!     #[feuilletage(variant = "enabled", variant_value = true)]
//!     Enabled(bool),
//!     #[feuilletage(variant = "default", variant_default)]
//!     Defaulted(Named),
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(untagged)]
//! enum OneOrMany {
//!     One(String),
//!     #[feuilletage(allow_single)]
//!     Many(Vec<String>),
//! }
//!
//! #[derive(Debug, feuilletage::Config, PartialEq)]
//! #[feuilletage(tag = "kind")]
//! enum Tagged { Text { value: String } }
//!
//! let mut config = Config::default();
//! config.load_yaml("automatic", Context::new(Source::Programmatic, Level::User));
//! assert_eq!(config.deserialize::<Mode>().unwrap(), Mode::Auto);
//!
//! let mut external = Config::default();
//! external.load_yaml("tool: rust", Context::new(Source::Programmatic, Level::User));
//! assert_eq!(
//!     external.deserialize::<External>().unwrap(),
//!     External::Other(Named { name: "tool".into() }),
//! );
//! # }
//! ```
//!
//! ## Compile-time Rejection
//!
//! Unknown attributes are rejected in the main crate, where these tests prove
//! the parser diagnostic rather than merely failing to resolve a dependency.
//!
//! ```compile_fail
//! #[derive(feuilletage::Config)]
//! #[feuilletage(unknown_container)]
//! struct InvalidContainer { value: String }
//! ```
//!
//! ```compile_fail
//! #[derive(feuilletage::Config)]
//! struct InvalidField {
//!     #[feuilletage(unknown_field)]
//!     value: String,
//! }
//! ```
//!
//! ```compile_fail
//! #[derive(feuilletage::Config)]
//! enum InvalidVariant {
//!     #[feuilletage(unknown_variant)]
//!     Value,
//! }
//! ```
//!
//! Projection targets cannot declare mutability constraints because parsing
//! and mutability enforcement belong to the wire type.
//!
//! ```compile_fail
//! #[derive(feuilletage::Config)]
//! struct Wire { value: String }
//!
//! #[derive(feuilletage::Config)]
//! #[feuilletage(
//!     parse_as = "Wire",
//!     mutable_by = ["user"],
//!     skip_serialize,
//!     skip_deserialize
//! )]
//! struct InvalidProjection { value: String }
//!
//! impl<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>
//!     feuilletage::FromParsed<Wire, S, L> for InvalidProjection
//! {
//!     fn from_parsed(
//!         parsed: Wire,
//!         _original: &feuilletage::ContextValue<S, L>,
//!         _tracker: &mut feuilletage::ErrorTracker,
//!     ) -> Result<Self, feuilletage::Error> {
//!         Ok(Self { value: parsed.value })
//!     }
//! }
//! ```
//!
//! ```compile_fail
//! #[derive(feuilletage::Config)]
//! struct Wire { value: String }
//!
//! #[derive(feuilletage::Config)]
//! #[feuilletage(parse_as = "Wire", skip_serialize, skip_deserialize)]
//! struct InvalidProjection {
//!     #[feuilletage(mutable_by = ["user"])]
//!     value: String,
//! }
//!
//! impl<S: feuilletage::CustomSource, L: feuilletage::CustomLevel>
//!     feuilletage::FromParsed<Wire, S, L> for InvalidProjection
//! {
//!     fn from_parsed(
//!         parsed: Wire,
//!         _original: &feuilletage::ContextValue<S, L>,
//!         _tracker: &mut feuilletage::ErrorTracker,
//!     ) -> Result<Self, feuilletage::Error> {
//!         Ok(Self { value: parsed.value })
//!     }
//! }
//! ```

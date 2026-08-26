# Feuilletage

[![docs.rs](https://img.shields.io/docsrs/feuilletage)](https://docs.rs/feuilletage)
[![Crates.io](https://img.shields.io/crates/v/feuilletage.svg)](https://crates.io/crates/feuilletage)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A flexible Rust configuration library with layered merging, source/level
provenance, and a derive-based deserializer.

- **Multi-format**: YAML, JSON, TOML (feature-gated)
- **Layered merging**: Recursive merge across system → user → local levels with
  per-field mutability constraints
- **Powerful derive**: `#[derive(feuilletage::Config)]` with 30+ attributes for
  defaults, validation, environment fallbacks, transforms, templates, enums
- **Provenance tracking**: every value remembers which source and level set it
- **`no_std` friendly**: core merging and deserialization work without `std`

Full reference docs: [docs.rs/feuilletage](https://docs.rs/feuilletage)

## Install

```toml
[dependencies]
feuilletage = { version = "0.1", features = ["yaml", "json"] }
```

## A complete example

Load a YAML system default, override with a user YAML snippet, validate, and
deserialize — in one pass:

```rust
# #[cfg(feature = "yaml")] {
use feuilletage::{Config, Context, Level, Source};

#[derive(Debug, feuilletage::Config)]
struct ServerConfig {
    #[feuilletage(default = "localhost")]
    host: String,

    #[feuilletage(range(1, 65535), default = "8080")]
    port: u16,

    #[feuilletage(default)]
    debug: bool,
}

let mut config = Config::default();

// System-level YAML (lowest priority).
config.load_yaml(
    "host: 0.0.0.0\nport: 8080\n",
    Context::new(Source::Programmatic, Level::System),
);

// User-level YAML override (higher priority).
config.load_yaml(
    "port: 3000\ndebug: true\n",
    Context::new(Source::Programmatic, Level::User),
);

let server: ServerConfig = config.deserialize().unwrap();
assert_eq!(server.host, "0.0.0.0");      // from system YAML
assert_eq!(server.port, 3000);            // overridden by user YAML
assert_eq!(server.debug, true);           // overridden by user YAML
# }
```

## Feature flags

| Feature | Default | Description |
|---|---|---|
| `std` | yes | File I/O, environment expansion, `PathBuf` |
| `yaml` | yes | YAML loading (requires `std`) |
| `json` | no | JSON loading |
| `toml` | no | TOML loading |
| `regex` | no | `#[feuilletage(regex = "...")]` validation |
| `chrono` | no | `#[feuilletage(datetime = "...")]` date/time validation |

## Layered configuration

Feuilletage was built around the system/user/local triad. Mutability rules let you
lock down which levels can set each field:

```rust
#[derive(feuilletage::Config)]
struct SecureConfig {
    /// Only the system level can set this — user/local overrides are ignored
    /// and recorded as warnings.
    #[feuilletage(mutable_by = ["system"])]
    admin_key: String,

    /// System or user; local cannot change it.
    #[feuilletage(mutable_by = ["system", "user"])]
    api_endpoint: String,
}
```

Loading multiple files with the builder:

```rust,no_run
# #[cfg(feature = "std")] {
use feuilletage::loader::loader;
use feuilletage::Level;

#[derive(feuilletage::Config)]
struct AppConfig {
    #[feuilletage(default = "myapp")]
    name: String,
}

let config: AppConfig = loader()
    .load_file("/etc/myapp/config.yaml", Level::System)
    .load_file("~/.config/myapp/config.yaml", Level::User)
    .load_file("./.myapp.yaml", Level::Local)
    .deserialize()
    .unwrap();
# }
```

Recognized `.yaml`, `.yml`, `.json`, and `.toml` extensions determine the input
format. Extensionless and nonstandard paths require an explicit format or an
explicit opt-in to best-effort content detection:

```rust,no_run
# #[cfg(all(feature = "std", feature = "toml"))] {
use feuilletage::{loader, Format, Level};

let config = loader()
    .load_file_with_format("config", Format::Toml, Level::User)
    .build()
    .unwrap();

let detected = loader()
    .load_file_auto("config", Level::User)
    .build()
    .unwrap();
# }
```

Auto loading tries only enabled parsers in `json > toml > yaml` order. It is
best effort, not format identification: JSON is valid YAML, and YAML may accept
TOML-looking input as a scalar. Failed parser attempts are discarded if a later
parser succeeds; if all attempts fail, one aggregate error is reported.

`default_format` is output-only. It controls serialization when no loaded
format exists and file writes whose destination has no recognized extension.
Output resolution uses a recognized destination extension or the most recently
loaded format first, then the configured preference, then the enabled-format
fallback (`yaml > toml > json`). Successfully loaded formats are preserved for
`loaded_format()`, `serialize()`, and `serialize_raw()`.

Container-level mutability rules provide defaults that fields can override:

```rust
#[derive(feuilletage::Config)]
#[feuilletage(mutable_by = ["system"])]
struct SecureDefaults {
    api_key: String,

    #[feuilletage(mutable_by = ["system", "user"])]
    timeout: u32,
}
```

Use `nested` when a field is another derived config whose mutability rules
must be enforced while parent objects are merged. Every containing field from
the deserialized root to the constrained nested field must opt in. Nested paths
compose through `rename` and `flatten`, and warnings report the complete
serialized path:

```rust
#[derive(Default, feuilletage::Config)]
#[feuilletage(mutable_by = ["system", "user"])]
struct OperationPolicy {
    #[feuilletage(default)]
    allowed: Vec<String>,
}

#[derive(feuilletage::Config)]
struct AppConfig {
    #[feuilletage(default, nested)]
    operations: OperationPolicy,
}
```

## Merge modifiers

Suffix any key to control how that value combines with what was already there:

| Suffix | Behavior |
|---|---|
| `key__tokeep` | Only set if not already present |
| `key__toappend` | Append to existing array |
| `key__toprepend` | Prepend to existing array |
| `key__toreplace` | Replace entirely (even for objects) |

```yaml
# System
plugins: [auth, logging]

# User overlay
plugins__toappend: [metrics]
# → [auth, logging, metrics]
```

## The derive macro

### Defaults, validation, environment

```rust
#[derive(feuilletage::Config)]
struct AppConfig {
    #[feuilletage(default = "localhost")]
    host: String,

    #[feuilletage(env = "PORT", range(1, 65535), default = "8080")]
    port: u16,

    #[feuilletage(length(8, 64))]
    password: String,

    #[feuilletage(validate = "is_even")]
    workers: i32,
}

fn is_even(n: &i32) -> Result<(), String> {
    if n % 2 == 0 { Ok(()) } else { Err("must be even".into()) }
}
```

### Duration parsing

Parse human-readable durations (`"5m"`, `"2h30m"`, `"500ms"`, units `ns` /
`us` / `µs` / `ms` / `s` / `m` / `h` / `d` / `w`):

```rust
#[derive(feuilletage::Config)]
struct Timeouts {
    #[feuilletage(duration)]
    request: u64,          // "5m" → 300 (seconds)

    #[feuilletage(duration(ms))]
    poll: u64,             // "500ms" → 500

    #[feuilletage(duration(ns))]
    tick: u64,             // "1ms" → 1_000_000
}
```

### Template interpolation

Fields can reference each other with `%{field}` (resolution order is detected
automatically):

```rust
# #[cfg(feature = "std")] {
#[derive(feuilletage::Config)]
struct Api {
    #[feuilletage(default = "localhost")]
    host: String,

    #[feuilletage(default = "8080")]
    port: u16,

    #[feuilletage(template)]
    base_url: String,      // "http://%{host}:%{port}" → "http://localhost:8080"
}
# }
```

### Flexible input shape

Accept multiple input shapes for the same field:

```rust
#[derive(feuilletage::Config)]
#[feuilletage(scalar_as = "version")]    // String input becomes {version: "..."}
struct PythonConfig {
    version: Option<String>,
}

#[derive(feuilletage::Config)]
#[feuilletage(array_as = "packages")]    // Array input becomes {packages: [...]}
struct HomebrewConfig {
    #[feuilletage(allow_single)]         // Also accepts a single value as array
    packages: Vec<String>,
}
```

### Enums

Internally tagged:

```rust
#[derive(feuilletage::Config)]
#[feuilletage(tag = "type")]
enum Action {
    Start { delay: u32 },
    Stop,
}
// Input: {"type": "start", "delay": 5}
```

External tag (factory pattern — the map key picks the variant):

```rust
# #[derive(feuilletage::Config)] struct HomebrewConfig { #[feuilletage(default)] pkgs: Vec<String> }
# #[derive(feuilletage::Config)] struct PythonConfig { #[feuilletage(default)] version: Option<String> }
# #[derive(feuilletage::Config)]
# #[feuilletage(scalar_as = "version")]
# struct MiseConfig {
#     #[feuilletage(default = "")] name: String,
#     version: Option<String>,
# }
#[derive(feuilletage::Config)]
#[feuilletage(external_tag, rename_all = "kebab-case")]
enum Tool {
    #[feuilletage(alias = "brew")]
    Homebrew(HomebrewConfig),
    Python(PythonConfig),
    #[feuilletage(fallback, from_tag = "name")] // Unknown keys land here
    Mise(MiseConfig),
}
// Input: {"rust": "1.75"}      → Mise   { name: "rust",      version: Some("1.75") }
// Input: {"homebrew": [...]}   → Homebrew { ... }
```

Untagged (tries variants in order):

```rust
#[derive(feuilletage::Config)]
#[feuilletage(untagged)]
enum StringOrNumber {
    Text(String),
    Number(i64),
}
```

## Edit API

Navigate and modify a loaded config without reserializing:

```rust
use feuilletage::{Config, ContextValue};

let mut config = Config::default();
config.at("database.host").set("localhost").unwrap();
config.at("database.port").set(5432_i64).unwrap();

assert!(config.at("database.host").exists());
assert!(matches!(
    config.at("database.port").get(),
    Some(ContextValue::Int(5432, _))
));
```

## Examples

Each feature has a runnable example:

```bash
cargo run -p feuilletage --example simple_validation --features "json yaml"
```

Available: `simple_validation`, `duration_parsing`, `external_tag`,
`template_interpolation`, `layered_config`, `merge_modifiers`,
`flexible_input`, `tagged_enum`, `untagged_enum`, `mutable_by`,
`env_loading`, `format_validation`, `path_resolution`, `type_coercion`,
`aliases_and_flatten`, `map_to_vec`, `env_validation`, `env_minimal`.

## Testing

```bash
cargo test --all-features        # 1325 tests passing, 0 ignored
cargo doc --all-features --open  # full API reference
./scripts/check-rustdoc-example-coverage.sh
```

The coverage gate measures public API items containing rustdoc examples, not
the raw number of doctest blocks. It permits improvements but fails if either
the example-covered item count or percentage drops below the checked-in
baseline.

Maintainers: see [RELEASING.md](RELEASING.md) for the release-PR workflow and
independent `feuilletage` / `feuilletage-macros` versioning.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

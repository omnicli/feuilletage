mod attrs;
mod gen_enum;
mod gen_serialize;
mod gen_validation;
mod gen_vec;
mod helpers;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

use crate::attrs::{
    parse_container_attributes, parse_default_fn_value, parse_field_config_attributes,
    validate_feuilletage_attributes, ContainerAttributes, DefaultValue, DurationConfig,
    FieldConfigAttributes, OnErrorMode,
};
use crate::gen_enum::{generate_deserialize_via_from_context_value, generate_enum_impl};
use crate::gen_serialize::{
    generate_serialize_impl, get_skip_check_for_default, get_skip_check_for_type,
    get_skip_check_recursive_for_type,
};
use crate::gen_validation::generate_validation_code;
use crate::gen_vec::generate_vec_deserialization;
use crate::helpers::{
    convert_string_default, extract_map_value_type, get_inner_type, get_map_kind, get_type_name,
    is_bool_type, is_float_type, is_option_type, is_signed_int_type, is_string_type,
    is_unsigned_int_type, parse_transform_path,
};

fn generate_projection_impl(
    name: &syn::Ident,
    parsed_type: &syn::Type,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let projection_where_clause = if let Some(where_clause) = where_clause {
        let predicates = &where_clause.predicates;
        quote! {
            where
                #predicates,
                #name #ty_generics: feuilletage::FromParsed<#parsed_type, __FeuilletageS, __FeuilletageL>
        }
    } else {
        quote! {
            where
                #name #ty_generics: feuilletage::FromParsed<#parsed_type, __FeuilletageS, __FeuilletageL>
        }
    };

    quote! {
        impl #extended_impl_generics feuilletage::FromContextValue<__FeuilletageS, __FeuilletageL> for #name #ty_generics #projection_where_clause {
            fn from_context_value(
                value: &feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>,
                tracker: &mut feuilletage::ErrorTracker,
            ) -> Result<Self, feuilletage::Error> {
                let __feuilletage_parsed = <#parsed_type as feuilletage::FromContextValue<__FeuilletageS, __FeuilletageL>>::from_context_value(
                    value,
                    tracker,
                )?;
                <Self as feuilletage::FromParsed<#parsed_type, __FeuilletageS, __FeuilletageL>>::from_parsed(
                    __feuilletage_parsed,
                    value,
                    tracker,
                )
            }
        }
    }
}

fn generate_projection_mutability_info_impl(
    name: &syn::Ident,
    parsed_type: &syn::Type,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let projection_where_clause = if let Some(where_clause) = where_clause {
        let predicates = &where_clause.predicates;
        quote! {
            where
                #predicates,
                #parsed_type: feuilletage::MutabilityInfo
        }
    } else {
        quote! {
            where
                #parsed_type: feuilletage::MutabilityInfo
        }
    };

    quote! {
        impl #impl_generics feuilletage::MutabilityInfo for #name #ty_generics #projection_where_clause {
            fn mutability_constraints() -> feuilletage::MutabilityConstraints {
                <#parsed_type as feuilletage::MutabilityInfo>::mutability_constraints()
            }
        }
    }
}

/// Derive macro for configuration deserialization.
///
/// The `Config` derive macro generates implementations of `FromContextValue`
/// and `serde::Serialize` for your configuration structs and enums.
///
/// All attributes use the `#[feuilletage(...)]` namespace. Unknown container,
/// field, and variant attributes are rejected at compile time. The main
/// `feuilletage` crate owns executable rejection tests because generated code
/// depends on its runtime types.
///
/// # Runnable Examples
///
/// For runnable doctest examples of all attributes, see the main
/// [`feuilletage`](https://docs.rs/feuilletage) library documentation, specifically
/// the "Attribute Reference" section which includes examples that compile
/// and run as part of the test suite.
///
/// # Required by Default
///
/// **Fields are required by default.** Non-Option fields without a default will error if missing.
/// - `Option<T>` fields automatically default to `None` (always optional)
/// - Use `default`, `default = "xx"`, or `default_fn = "xx"` to make a field optional
///
/// # Basic Usage
///
/// ```text
/// use feuilletage::{Config, Context, Level, Source, FromContextValue};
///
/// #[derive(Debug, Config)]
/// struct AppConfig {
///     // Required (no default)
///     api_key: String,
///
///     #[feuilletage(default = "localhost")]
///     host: String,
///
///     #[feuilletage(default = "8080")]
///     port: i32,
///
///     #[feuilletage(default)]  // Uses Default::default()
///     debug: bool,
///
///     // Optional (None if missing)
///     override_url: Option<String>,
/// }
/// ```
///
/// # Field Attributes
///
/// ## Default Values
///
/// - `#[feuilletage(default = "value")]` - Use explicit default value (makes field optional)
/// - `#[feuilletage(default)]` - Use `Default::default()` (makes field optional)
/// - `#[feuilletage(default_fn = "function_name")]` - Call function to get default (makes field optional)
///
/// ```text
/// #[derive(Config)]
/// struct Config {
///     #[feuilletage(default = "localhost")]
///     host: String,  // Defaults to "localhost"
///
///     #[feuilletage(default)]
///     count: i32,  // Defaults to 0 (i32::default())
///
///     #[feuilletage(default_fn = "default_timeout")]
///     timeout: i64,  // Calls default_timeout() for default
///
///     api_key: String,  // Required - error if not provided
/// }
///
/// fn default_timeout() -> i64 { 30 }
/// ```
///
/// ## Flexible Input Handling
///
/// - `#[feuilletage(allow_single)]` - Accept single value as array (for Vec fields)
/// - `#[feuilletage(allow_single = "field")]` - Wrap scalar as object (for struct fields)
/// - `#[feuilletage(allow_map = "key")]` - Accept map notation for Vec
/// - `#[feuilletage(allow_map(key = "field", scalar_as = "value_field"))]` - Full syntax
/// - `#[feuilletage(allow_map(key = "field", order_by = "sort_field"))]` - Sort after conversion
/// - `#[feuilletage(allow_list)]` - Accept array input for HashMap/BTreeMap fields
///
/// ```text
/// #[derive(Config)]
/// struct Config {
///     #[feuilletage(allow_single)]
///     tags: Vec<String>,  // "foo" -> ["foo"], ["foo", "bar"] -> ["foo", "bar"]
///
///     #[feuilletage(allow_map(key = "name", scalar_as = "version"))]
///     packages: Vec<Package>,  // {"curl": "1.0"} -> [{name: "curl", version: "1.0"}]
/// }
/// ```
///
/// ## Transforms
///
/// - `#[feuilletage(transform = "function_name")]` - Apply transform function
/// - `#[feuilletage(transform_each = "function_name")]` - Transform each item in Vec
/// - `#[feuilletage(relative_path)]` - Shorthand for `transform = "relative_path"`
/// - `#[feuilletage(normalize_path)]` - Shorthand for `transform = "normalize_path"` (resolves `.` and `..`)
/// - `#[feuilletage(duration)]` - Parse duration strings to seconds (default)
/// - `#[feuilletage(duration(ms))]` - Parse duration strings to specified unit (shorthand)
/// - `#[feuilletage(duration(unit = ms))]` - Parse duration strings to specified unit (explicit)
///
/// ### Duration Attribute Syntax
///
/// The `duration` attribute supports three syntaxes:
///
/// | Syntax | Output Unit | Description |
/// |--------|-------------|-------------|
/// | `#[feuilletage(duration)]` | seconds | Default - converts to seconds |
/// | `#[feuilletage(duration(ms))]` | milliseconds | Shorthand - unit identifier only |
/// | `#[feuilletage(duration(unit = ns))]` | nanoseconds | Explicit - named parameter |
///
/// ### Duration Units
///
/// | Unit | Name | Example |
/// |------|------|---------|
/// | `ns` | nanoseconds | `"1000ns"` -> 1000 |
/// | `us` | microseconds | `"100us"` -> 100 |
/// | `ms` | milliseconds | `"500ms"` -> 500 |
/// | `s` | seconds (default) | `"30s"` -> 30 |
/// | `m` | minutes | `"5m"` -> 5 (minutes) or 300 (seconds) |
/// | `h` | hours | `"2h"` -> 2 (hours) or 7200 (seconds) |
/// | `d` | days | `"1d"` -> 1 (days) or 86400 (seconds) |
/// | `w` | weeks | `"1w"` -> 1 (weeks) or 604800 (seconds) |
///
/// Combined formats are supported: `"1h30m"`, `"2d12h"`, `"1s500ms"`
///
/// ### Duration Examples
///
/// ```text
/// #[derive(Config)]
/// struct TimeoutConfig {
///     #[feuilletage(transform = "to_uppercase")]
///     name: String,
///
///     // Default: converts to seconds
///     #[feuilletage(duration)]
///     timeout_secs: u64,  // "5m" -> 300 (seconds)
///
///     // Shorthand syntax: unit identifier in parentheses
///     #[feuilletage(duration(ms))]
///     poll_interval: u64,  // "5s" -> 5000 (milliseconds)
///
///     // Explicit syntax: named parameter
///     #[feuilletage(duration(unit = ns))]
///     latency_threshold: u64,  // "1ms" -> 1_000_000 (nanoseconds)
///
///     // Float fields preserve fractional precision
///     #[feuilletage(duration)]
///     precise_timeout: f64,  // "1s500ms" -> 1.5 (seconds as float)
///
///     #[feuilletage(relative_path)]
///     config_path: String,  // Resolved relative to config file
/// }
/// ```
///
/// ## Field Naming
///
/// - `#[feuilletage(rename = "key")]` - Use different key for serialization/deserialization
/// - `#[feuilletage(aliases = ["alt1", "alt2"])]` - Accept alternative key names
///
/// ```text
/// #[derive(Config)]
/// struct Config {
///     #[feuilletage(rename = "userName")]
///     user_name: String,
///
///     #[feuilletage(aliases = ["count", "n"])]
///     item_count: i32,
/// }
/// ```
///
/// ## Validation
///
/// - `#[feuilletage(range(min, max))]` - Numeric range validation
/// - `#[feuilletage(regex = "pattern")]` - String pattern validation (requires `regex` feature)
/// - `#[feuilletage(length(min, max))]` - String/list length validation
/// - `#[feuilletage(validate = "function_name")]` - Custom validation function
/// - `#[feuilletage(datetime = "fmt")]` - Date/time format validation (requires `chrono` feature)
/// - `#[feuilletage(absolute_path)]` - Validate path is absolute
///
/// ```text
/// #[derive(Config)]
/// struct Config {
///     #[feuilletage(range(0, 100))]
///     percentage: i32,
///
///     #[feuilletage(regex = r"^[a-z0-9_]+$")]
///     username: String,
///
///     #[feuilletage(length(8, 128))]
///     password: String,
///
///     #[feuilletage(validate = "validate_port")]
///     port: i32,
/// }
///
/// fn validate_port(value: &i32) -> Result<(), String> {
///     if *value > 0 && *value < 65536 { Ok(()) }
///     else { Err("port must be between 1 and 65535".to_string()) }
/// }
/// ```
///
/// ## Environment Variables
///
/// - `#[feuilletage(env = "VAR_NAME")]` - Load from environment variable as fallback
///
/// ```text
/// #[derive(Config)]
/// struct Config {
///     #[feuilletage(env = "DATABASE_URL", default = "localhost")]
///     db_url: String,  // Config value > env var > default
/// }
/// ```
///
/// ## Metadata
///
/// - `#[feuilletage(deprecated = "message")]` - Print deprecation warning
/// - `#[feuilletage(secret)]` - Hide value in error messages
/// - `#[feuilletage(mutable_by = ["level1", "level2"])]` - Restrict which levels can set this field
/// - `#[feuilletage(nested)]` - Compose a nested Config type's mutability rules into the parent
///
/// ## Type Coercion
///
/// - `#[feuilletage(coerce)]` - Enable liberal type coercion (string<->int<->float<->bool)
///
/// ## Struct Flattening
///
/// - `#[feuilletage(flatten)]` - Flatten nested struct fields into parent
///
/// ## Serialization Control
///
/// - `#[feuilletage(skip)]` - Always skip serializing this field
/// - `#[feuilletage(skip_if_empty)]` - Skip if field is empty
/// - `#[feuilletage(skip_if_empty_recursive)]` - Skip if field is empty recursively
/// - `#[feuilletage(skip_if_default)]` - Skip if field equals default value
/// - `#[feuilletage(skip_if = "function_name")]` - Custom skip function
/// - `#[feuilletage(serialize_single_as_value)]` - Serialize single-item Vec as value
///
/// # Struct-Level Attributes
///
/// - `#[feuilletage(parse_as = "WireType")]` - Parse `WireType` with its
///   `FromContextValue` implementation, then construct the target through
///   `FromParsed<WireType, S, L>`. The wire type owns deserialization and
///   `mutable_by` constraints; projected targets cannot define `mutable_by`.
/// - `#[feuilletage(scalar_as = "field")]` - Wrap scalar input as `{field: value}`
/// - `#[feuilletage(array_as = "field")]` - Wrap array input as `{field: value}`
/// - `#[feuilletage(transform = "fn_name")]` - Run a normalizer function on the raw
///   input `ContextValue` before any field deserialization or `scalar_as` /
///   `array_as` wrapping. Same signature as field-level `transform`. Lets a
///   struct accept multiple input shapes and normalize them to a canonical
///   form declaratively.
///
/// ```text
/// #[derive(Config)]
/// #[feuilletage(scalar_as = "file", array_as = "packages")]
/// struct NixSpec {
///     file: Option<String>,
///     packages: Option<Vec<String>>,
/// }
/// // "shell.nix" -> {file: "shell.nix"}
/// // ["pkg1", "pkg2"] -> {packages: ["pkg1", "pkg2"]}
/// ```
///
/// # Enum Support
///
/// ## Tagged Enums
///
/// Use `#[feuilletage(tag = "field")]` for internally tagged enums:
///
/// ```text
/// #[derive(Config)]
/// #[feuilletage(tag = "type")]
/// enum Message {
///     Text { content: String },
///     #[feuilletage(rename = "image")]
///     Image { url: String },
/// }
/// // {"type": "text", "content": "hello"} -> Message::Text { content: "hello" }
/// ```
///
/// ## Untagged Enums
///
/// Use `#[feuilletage(untagged)]` for untagged enums (tries each variant in order):
///
/// ```text
/// #[derive(Config)]
/// #[feuilletage(untagged)]
/// enum Value {
///     Simple(String),
///     Complex { name: String, value: i32 },
/// }
/// // "hello" -> Value::Simple("hello")
/// // {"name": "foo", "value": 42} -> Value::Complex { name: "foo", value: 42 }
/// ```
///
/// ## Enum Attributes
///
/// - `#[feuilletage(rename = "tag")]` - Variant tag name
/// - `#[feuilletage(alias = "alt")]` - Alternative tag value
/// - `#[feuilletage(rename_all = "case")]` - Case conversion for all variants
///   (snake_case, camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE)
#[proc_macro_derive(Config, attributes(feuilletage))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    if let Err(error) = validate_feuilletage_attributes(&input) {
        return error.into_compile_error().into();
    }

    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Create extended generics with __FeuilletageS and __FeuilletageL for FromContextValue impl.
    // This is needed because:
    // 1. The user's struct may have its own generics (e.g., MyConfig<T>)
    // 2. We need to add our S, L type params for FromContextValue<S, L>
    // 3. So for MyConfig<T>, we generate: impl<T, __FeuilletageS, __FeuilletageL> FromContextValue<__FeuilletageS, __FeuilletageL> for MyConfig<T>
    let mut extended_generics = generics.clone();
    extended_generics
        .params
        .push(syn::parse_quote!(__FeuilletageS: feuilletage::CustomSource));
    extended_generics
        .params
        .push(syn::parse_quote!(__FeuilletageL: feuilletage::CustomLevel));
    let (extended_impl_generics, _, _) = extended_generics.split_for_impl();

    // Parse container-level attributes
    let container_attrs = parse_container_attributes(&input.attrs);

    let implementation: proc_macro2::TokenStream = match &input.data {
        Data::Struct(data) => generate_struct_impl(
            name,
            generics,
            &data.fields,
            &container_attrs,
            impl_generics,
            extended_impl_generics,
            ty_generics,
            where_clause,
        ),
        Data::Enum(data) => generate_enum_impl(
            name,
            generics,
            &data.variants,
            &container_attrs,
            impl_generics,
            extended_impl_generics,
            ty_generics,
            where_clause,
        ),
        Data::Union(_) => {
            panic!("Config cannot be derived for unions")
        }
    }
    .into();

    quote! {
        const _: () = {
            use ::feuilletage as feuilletage;
            #[allow(unused_imports)]
            use ::feuilletage::__private::{
                format, vec, Cow as __FeuilletageCow, HashSet as __FeuilletageHashSet,
                IndexMap as __FeuilletageIndexMap, ToString as __FeuilletageToString, serde, serde_json,
            };

            #implementation
        };
    }
    .into()
}

/// Generate FromContextValue implementation for structs
#[allow(clippy::too_many_arguments)]
fn generate_struct_impl(
    name: &syn::Ident,
    _generics: &syn::Generics,
    fields: &Fields,
    container_attrs: &ContainerAttributes,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    let fields = match fields {
        Fields::Named(fields) => &fields.named,
        Fields::Unnamed(fields) if container_attrs.transparent && fields.unnamed.len() == 1 => {
            &fields.unnamed
        }
        Fields::Unnamed(_) => panic!(
            "Config can only be derived for tuple structs when `#[feuilletage(transparent)]` is set and there is exactly one field"
        ),
        Fields::Unit => panic!("Config cannot be derived for unit structs"),
    };

    // Handle transparent structs - serialize/deserialize as the single inner field
    if container_attrs.transparent {
        return generate_transparent_struct_impl(
            name,
            fields,
            container_attrs,
            impl_generics,
            extended_impl_generics,
            ty_generics,
            where_clause,
        );
    }

    // Collect field info including flatten status
    let field_infos: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();
            let field_type = &field.ty;
            let mut attrs = parse_field_config_attributes(&field.attrs);
            if attrs.mutable_by.is_none() {
                attrs.mutable_by = container_attrs.mutable_by.clone();
            }
            (
                field_name.clone(),
                field_name_str,
                field_type.clone(),
                attrs,
            )
        })
        .collect();

    // Collect non-flattened field names for exclusion when building remaining_obj
    // This includes renamed keys and aliases
    // Fields with from_context are also excluded since they don't come from the input object
    let non_flattened_field_names: Vec<_> = field_infos
        .iter()
        .filter(|(_, _, _, attrs)| {
            !attrs.flatten && attrs.from_context.is_none() && attrs.from_context_fn.is_none()
        })
        .flat_map(|(_, field_name_str, _, attrs)| {
            // Primary key is rename if specified, otherwise field name
            let primary_key = attrs.rename.as_ref().unwrap_or(field_name_str).clone();
            let mut keys = vec![primary_key];
            // Also include aliases
            keys.extend(attrs.aliases.iter().cloned());
            keys
        })
        .collect();

    // Check if we have any flattened fields
    let has_flattened_fields = field_infos.iter().any(|(_, _, _, attrs)| attrs.flatten);

    // Generate the remaining_obj creation if needed
    let remaining_obj_creation = if has_flattened_fields {
        let excluded_keys: Vec<proc_macro2::TokenStream> = non_flattened_field_names
            .iter()
            .map(|k| quote! { #k })
            .collect();
        quote! {
            // Create a filtered object containing only keys not consumed by non-flattened fields
            let __excluded_keys: __FeuilletageHashSet<&str> = [#(#excluded_keys),*].into_iter().collect();
            let __remaining_obj: __FeuilletageIndexMap<String, feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>> = obj.iter()
                .filter(|(k, _)| !__excluded_keys.contains(k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let __remaining_value = feuilletage::ContextValue::object(
                __remaining_obj,
                value.context().clone(),
            );
        }
    } else {
        quote! {}
    };

    let field_names: Vec<_> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

    // Check if any fields have fallback attributes or default_fn with params
    let has_fallback_fields = field_infos
        .iter()
        .any(|(_, _, _, attrs)| attrs.fallback.is_some());

    // Check if any fields have default_fn with field parameters
    let has_default_fn_with_params = field_infos.iter().any(|(_, _, _, attrs)| {
        if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
            let (_, params) = parse_default_fn_value(fn_str);
            !params.is_empty()
        } else {
            false
        }
    });

    // Build set of fields that are fallback targets (they need raw extraction too)
    let fallback_targets: std::collections::HashSet<String> = field_infos
        .iter()
        .filter_map(|(_, _, _, attrs)| attrs.fallback.clone())
        .collect();

    // Build set of fields that are default_fn param targets (they need to be resolved before use)
    let default_fn_targets: std::collections::HashSet<String> = field_infos
        .iter()
        .flat_map(|(_, _, _, attrs)| {
            if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
                let (_, params) = parse_default_fn_value(fn_str);
                params
            } else {
                vec![]
            }
        })
        .collect();

    // Check if a field is involved in dependencies (fallback, default_fn params, or is a target)
    let is_dependency_involved = |field_name_str: &str, attrs: &FieldConfigAttributes| -> bool {
        // Has fallback attribute
        if attrs.fallback.is_some() {
            return true;
        }
        // Is a fallback target
        if fallback_targets.contains(field_name_str) {
            return true;
        }
        // Has default_fn with params
        if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
            let (_, params) = parse_default_fn_value(fn_str);
            if !params.is_empty() {
                return true;
            }
        }
        // Is a default_fn param target
        if default_fn_targets.contains(field_name_str) {
            return true;
        }
        false
    };

    // Helper to check if a field has default_fn with params
    let has_default_fn_params = |attrs: &FieldConfigAttributes| -> bool {
        if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
            let (_, params) = parse_default_fn_value(fn_str);
            !params.is_empty()
        } else {
            false
        }
    };

    let field_deserializations =
        field_infos
            .iter()
            .map(|(field_name, field_name_str, field_type, attrs)| {
                if attrs.skip {
                    // For skipped fields, use the default value directly (no deserialization)
                    generate_skip_field_default(field_name, field_type, attrs)
                } else if let Some(ref context_path) = attrs.from_context {
                    // For from_context fields, extract value from context metadata
                    generate_from_context_deserialization(field_name, field_type, context_path)
                } else if let Some(ref fn_name) = attrs.from_context_fn {
                    // For from_context_fn fields, call user function with context
                    generate_from_context_fn_deserialization(field_name, field_type, fn_name)
                } else if attrs.flatten {
                    // For flattened fields, pass the remaining object
                    generate_flatten_deserialization(field_name, field_type)
                } else if attrs.fallback.is_some() {
                    // This field has a fallback - generate raw extraction only
                    // The actual assignment happens in dependency resolution
                    generate_raw_field_extraction(field_name, field_name_str, field_type, attrs)
                } else if has_default_fn_params(attrs) {
                    // This field has default_fn with params - generate raw extraction only
                    // The actual assignment happens in dependency resolution
                    generate_raw_field_extraction(field_name, field_name_str, field_type, attrs)
                } else if is_dependency_involved(field_name_str, attrs) {
                    // This field is a dependency target (fallback or default_fn param) but doesn't need deferred resolution
                    // Generate raw extraction AND assign it directly if it has a value
                    generate_raw_field_with_assignment(
                        field_name,
                        field_name_str,
                        field_type,
                        attrs,
                    )
                } else {
                    generate_field_deserialization(field_name, field_name_str, field_type, attrs)
                }
            });

    // Generate dependency resolution code (handles both fallback and default_fn with params)
    let dependency_resolution = if has_fallback_fields || has_default_fn_with_params {
        generate_dependency_resolution(&field_infos)
    } else {
        quote! {}
    };

    // Generate skip helper methods for serde serialization (kept for backward compatibility)
    let skip_helpers = generate_skip_helpers(name, fields);

    // Generate Serialize impl (unless skip_serialize is set)
    let serialize_impl = if container_attrs.skip_serialize {
        quote! {}
    } else {
        generate_serialize_impl(
            name,
            fields,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    // Generate Deserialize impl (unless skip_deserialize is set)
    let deserialize_impl = if container_attrs.skip_deserialize {
        quote! {}
    } else {
        generate_deserialize_via_from_context_value(name, ty_generics.clone(), where_clause)
    };

    // Generate container-level transform call (runs BEFORE scalar_as/array_as wrapping
    // and field deserialization). The transform takes &mut ContextValue and may
    // arbitrarily reshape the raw input.
    let container_transform_call = if let Some(ref fn_name) = container_attrs.transform {
        let path = parse_transform_path(fn_name);
        quote! {
            // Container-level transform: normalize raw input shape.
            // We own a mutable clone, run the transform on it, then re-shadow `value`
            // with a borrow of the owned clone. All downstream code (scalar_as wrapping,
            // field deserialization, etc.) sees the transformed value.
            let __feuilletage_transformed_value = {
                let mut __feuilletage_v = value.clone();
                let __feuilletage_ctx = __feuilletage_v.context().clone();
                #path(&mut __feuilletage_v, &__feuilletage_ctx)?;
                __feuilletage_v
            };
            let value = &__feuilletage_transformed_value;
        }
    } else {
        quote! {}
    };

    // Generate value wrapping logic for scalar_as, array_as, and struct allow_map attributes
    let value_wrapping = generate_value_wrapping(container_attrs, &field_infos);

    // Generate template resolution code if any fields have template attribute
    let template_resolution = generate_template_resolution(&field_infos);

    // Generate post_process call if attribute is present
    // We save a reference to the original value before field deserialization
    // to avoid issues with field names that shadow the `value` parameter
    let (post_process_value_save, post_process_call) =
        if let Some(ref fn_name) = container_attrs.post_process {
            let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
            (
                quote! {
                    let __post_process_source = value;
                },
                quote! {
                    #fn_ident(&mut __result, __post_process_source, tracker)?;
                },
            )
        } else {
            (quote! {}, quote! {})
        };

    // Generate MutabilityInfo implementation
    let mutability_info_impl = if let Some(parsed_type) = &container_attrs.parse_as {
        generate_projection_mutability_info_impl(
            name,
            parsed_type,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    } else {
        generate_mutability_info_impl(
            &field_infos,
            name,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    // Generate AllowMapKeys implementation if struct has allow_map(key = ...)
    let allow_map_keys_impl = generate_allow_map_keys_impl(
        &field_infos,
        container_attrs,
        name,
        impl_generics.clone(),
        ty_generics.clone(),
        where_clause,
    );

    let from_context_value_impl = if let Some(parsed_type) = &container_attrs.parse_as {
        generate_projection_impl(
            name,
            parsed_type,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else {
        quote! {
            impl #extended_impl_generics feuilletage::FromContextValue<__FeuilletageS, __FeuilletageL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>,
                tracker: &mut feuilletage::ErrorTracker,
            ) -> Result<Self, feuilletage::Error> {
                #container_transform_call

                #value_wrapping

                // Save reference to value for post_process before fields may shadow it
                #post_process_value_save

                // Expect an object at the root
                let obj = match value {
                    feuilletage::ContextValue::Object(obj, _) => obj,
                    _ => {
                        tracker.record_type_mismatch("object", value.type_name());
                        return Err(feuilletage::Error::TypeMismatch {
                            path: tracker.current_path(),
                            expected: "object".to_string(),
                            actual: value.type_name().to_string(),
                        });
                    }
                };

                // Save reference to value before field deserialization can shadow it
                // (e.g., a field named `value` would shadow the function parameter)
                #[allow(unused_variables)]
                let __feuilletage_value = value;

                #remaining_obj_creation

                #(#field_deserializations)*

                // Resolve dependencies (fallback chains and default_fn with params)
                #dependency_resolution

                // Note: Errors are recorded in the tracker but deserialization continues
                // with default values. The caller can check tracker.has_errors() after
                // deserialization to decide whether to fail.

                #template_resolution

                let mut __result = Self {
                    #(#field_names: #field_names),*
                };

                #post_process_call

                Ok(__result)
            }
        }
        }
    };

    let expanded = quote! {
        #from_context_value_impl

        #serialize_impl

        #deserialize_impl

        #skip_helpers

        #mutability_info_impl

        #allow_map_keys_impl
    };

    TokenStream::from(expanded)
}

/// Generate implementations for transparent structs.
/// A transparent struct has exactly one field and serializes/deserializes as that field directly.
/// Field attributes like allow_single, allow_map, transform, etc. are applied to the input value.
fn generate_transparent_struct_impl(
    name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
    container_attrs: &ContainerAttributes,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    // Transparent requires exactly one field
    if fields.len() != 1 {
        panic!(
            "transparent can only be used on structs with exactly one field, found {}",
            fields.len()
        );
    }

    let field = fields.first().unwrap();
    let field_type = &field.ty;
    let attrs = parse_field_config_attributes(&field.attrs);

    // For named fields use the field's Ident. For tuple struct fields, synthesize
    // a local binding name and use syn::Index for `self.0` style access.
    let is_tuple = field.ident.is_none();
    let field_access: proc_macro2::TokenStream = if let Some(ident) = field.ident.as_ref() {
        quote! { #ident }
    } else {
        let idx = syn::Index::from(0);
        quote! { #idx }
    };
    let local_var = field
        .ident
        .clone()
        .unwrap_or_else(|| syn::Ident::new("__feuilletage_inner", proc_macro2::Span::call_site()));
    // Helper to construct a `Self { field: expr }` (named) or `Self(expr)` (tuple).
    let construct = |inner: &proc_macro2::TokenStream| -> proc_macro2::TokenStream {
        if is_tuple {
            quote! { Self(#inner) }
        } else {
            quote! { Self { #field_access: #inner } }
        }
    };

    // Generate Serialize impl (unless skip_serialize is set)
    let serialize_impl = if container_attrs.skip_serialize {
        quote! {}
    } else {
        quote! {
            impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
                fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
                where
                    __S: serde::Serializer,
                {
                    self.#field_access.serialize(serializer)
                }
            }
        }
    };

    // Generate Deserialize impl (unless skip_deserialize is set)
    let deserialize_impl = if container_attrs.skip_deserialize {
        quote! {}
    } else {
        generate_deserialize_via_from_context_value(name, ty_generics.clone(), where_clause)
    };

    // Generate field deserialization code that applies field attributes.
    // This binds the inner value to `local_var`.
    let field_deser_code =
        generate_transparent_field_deserialization(&local_var, field_type, &attrs);

    // Generate container-level transform call for the transparent struct.
    // Runs on the raw input BEFORE null-handling and field deserialization,
    // letting the transform reshape the input even if it was originally a scalar
    // that the inner field would otherwise reject.
    let container_transform_call = if let Some(ref fn_name) = container_attrs.transform {
        let path = parse_transform_path(fn_name);
        quote! {
            let __feuilletage_transformed_value = {
                let mut __feuilletage_v = value.clone();
                let __feuilletage_ctx = __feuilletage_v.context().clone();
                #path(&mut __feuilletage_v, &__feuilletage_ctx)?;
                __feuilletage_v
            };
            let value = &__feuilletage_transformed_value;
        }
    } else {
        quote! {}
    };

    // Generate post_process call if specified
    let post_process_code = if let Some(ref fn_name) = container_attrs.post_process {
        let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
        quote! {
            #fn_ident(&mut __result, value, tracker)?;
        }
    } else {
        quote! {}
    };

    // Generate null handling based on whether field has a default
    let null_handling = if let Some(default_value) = attrs.default_value.as_ref() {
        // Field has explicit default - use it for null
        let default_expr = match default_value {
            DefaultValue::Explicit(expr) => {
                let converted = convert_string_default(expr, field_type);
                converted.parse::<proc_macro2::TokenStream>().unwrap()
            }
            DefaultValue::UseDefault => quote! { Default::default() },
            DefaultValue::Function(fn_name) => {
                let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
                quote! { #fn_ident() }
            }
        };
        let ctor = construct(&default_expr);
        quote! {
            if value.is_null() {
                return Ok(#ctor);
            }
        }
    } else if is_option_type(field_type) {
        // Option<T> defaults to None for null
        let none_expr = quote! { None };
        let ctor = construct(&none_expr);
        quote! {
            if value.is_null() {
                return Ok(#ctor);
            }
        }
    } else {
        // No default - null is an error (field is required)
        quote! {
            if value.is_null() {
                return Err(feuilletage::Error::MissingField {
                    path: tracker.current_path(),
                });
            }
        }
    };

    let local_var_tokens = quote! { #local_var };
    let result_construct = construct(&local_var_tokens);

    let from_context_value_impl = if let Some(parsed_type) = &container_attrs.parse_as {
        generate_projection_impl(
            name,
            parsed_type,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else {
        quote! {
            impl #extended_impl_generics feuilletage::FromContextValue<__FeuilletageS, __FeuilletageL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>,
                tracker: &mut feuilletage::ErrorTracker,
            ) -> Result<Self, feuilletage::Error> {
                // Container-level transform runs before null handling and field deserialization
                #container_transform_call

                // Handle null based on field default
                #null_handling

                // Transparent: deserialize with field attribute processing
                #field_deser_code

                let mut __result = #result_construct;
                #post_process_code
                Ok(__result)
            }
        }
        }
    };

    let mutability_info_impl = if let Some(parsed_type) = &container_attrs.parse_as {
        generate_projection_mutability_info_impl(
            name,
            parsed_type,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    } else {
        quote! {
            impl #impl_generics feuilletage::MutabilityInfo for #name #ty_generics #where_clause {
                fn mutability_constraints() -> feuilletage::MutabilityConstraints {
                    feuilletage::MutabilityHashMap::default()
                }
            }
        }
    };

    let expanded = quote! {
        #from_context_value_impl

        #serialize_impl

        #deserialize_impl

        #mutability_info_impl
    };

    TokenStream::from(expanded)
}

/// Generate field deserialization code for transparent structs.
/// Uses the input value directly (no field lookup) and applies all field attributes.
fn generate_transparent_field_deserialization(
    field_name: &syn::Ident,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Check if this is a Vec type with allow_single or allow_map
    let is_vec_allow_single = attrs.allow_single.as_ref().is_some_and(|s| s.is_empty());
    let has_allow_map = attrs.allow_map.is_some() || attrs.allow_map_flag;

    if is_vec_allow_single || has_allow_map {
        generate_transparent_vec_deserialization(field_name, field_type, attrs)
    } else {
        // Simple case: just deserialize with optional transform
        if let Some(ref transform_fn) = attrs.transform {
            let transform_ident: proc_macro2::TokenStream = parse_transform_path(transform_fn);
            quote! {
                let mut __transformed = value.clone();
                let ctx = __transformed.context().clone();
                #transform_ident(&mut __transformed, &ctx)?;
                let #field_name = feuilletage::FromContextValue::from_context_value(&__transformed, tracker)?;
            }
        } else {
            quote! {
                let #field_name = feuilletage::FromContextValue::from_context_value(value, tracker)?;
            }
        }
    }
}

/// Generate Vec deserialization code for transparent structs with allow_single/allow_map.
fn generate_transparent_vec_deserialization(
    field_name: &syn::Ident,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Extract element type for sorting
    let element_type = get_inner_type(field_type);

    // Generate normalization code (allow_single/allow_map conversion)
    let has_allow_single = attrs.allow_single.as_ref().is_some_and(|s| s.is_empty());
    let normalization_code = if let Some(ref allow_map_config) = attrs.allow_map {
        let key_field = &allow_map_config.key_field;

        let map_to_array_body = if let Some(ref scalar_as_field) = allow_map_config.scalar_as_field
        {
            quote! {
                was_from_map = true;
                let mut items = Vec::new();
                for (map_key, item) in map.iter() {
                    let augmented_item = match item {
                        feuilletage::ContextValue::Object(inner_obj, _) => {
                            let mut new_obj = inner_obj.clone();
                            new_obj.insert(
                                #key_field.to_string(),
                                feuilletage::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            feuilletage::ContextValue::object(new_obj, item.context().clone())
                        }
                        _ => {
                            let mut new_obj = __FeuilletageIndexMap::default();
                            new_obj.insert(
                                #key_field.to_string(),
                                feuilletage::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            new_obj.insert(
                                #scalar_as_field.to_string(),
                                item.clone(),
                            );
                            feuilletage::ContextValue::object(new_obj, item.context().clone())
                        }
                    };
                    items.push(augmented_item);
                }
                feuilletage::ContextValue::array(items, value.context().clone())
            }
        } else {
            quote! {
                was_from_map = true;
                let mut items = Vec::new();
                for (map_key, item) in map.iter() {
                    let augmented_item = match item {
                        feuilletage::ContextValue::Object(inner_obj, _) => {
                            let mut new_obj = inner_obj.clone();
                            new_obj.insert(
                                #key_field.to_string(),
                                feuilletage::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            feuilletage::ContextValue::object(new_obj, item.context().clone())
                        }
                        _ => {
                            tracker.record(feuilletage::Error::TypeMismatch {
                                path: tracker.current_path(),
                                expected: "object".to_string(),
                                actual: item.type_name().to_string(),
                            });
                            continue;
                        }
                    };
                    items.push(augmented_item);
                }
                feuilletage::ContextValue::array(items, value.context().clone())
            }
        };

        // Wrap with single-item detection using AllowMapKeys trait
        let map_to_array = quote! {
            feuilletage::ContextValue::Object(map, _) => {
                // Use AllowMapKeys trait to detect if this object is a single item
                let key_fields = <#element_type as feuilletage::AllowMapKeys>::map_key_fields();
                let is_single_item = key_fields.iter().any(|k| map.contains_key(*k));

                if is_single_item {
                    // Object contains the key field — treat as a single item
                    feuilletage::ContextValue::array(vec![value.clone()], value.context().clone())
                } else {
                    // Map notation — convert to array with key injection
                    #map_to_array_body
                }
            }
        };

        let single_value_arm = if has_allow_single {
            quote! {
                _ => {
                    feuilletage::ContextValue::array(vec![value.clone()], value.context().clone())
                }
            }
        } else {
            quote! {
                other => {
                    return Err(feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: "array".to_string(),
                        actual: other.type_name().to_string(),
                    });
                }
            }
        };

        quote! {
            let mut was_from_map = false;
            let array_value = match value {
                feuilletage::ContextValue::Array(_, _) => value.clone(),
                #map_to_array
                #single_value_arm
            };
        }
    } else if attrs.allow_map_flag {
        // allow_map flag form: use inner type's AllowMapKeys trait
        let single_value_arm = if has_allow_single {
            quote! {
                _ => {
                    feuilletage::ContextValue::array(vec![value.clone()], value.context().clone())
                }
            }
        } else {
            quote! {
                other => {
                    return Err(feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: "array".to_string(),
                        actual: other.type_name().to_string(),
                    });
                }
            }
        };

        let elem_type = element_type.expect("Vec should have inner type");
        quote! {
            let mut was_from_map = false;
            let array_value = match value {
                feuilletage::ContextValue::Array(_, _) => value.clone(),
                feuilletage::ContextValue::Object(map, _) => {
                    // Use AllowMapKeys trait for detection
                    let key_fields = <#elem_type as feuilletage::AllowMapKeys>::map_key_fields();
                    let is_single_item = key_fields.iter().any(|k| map.contains_key(*k));

                    if is_single_item {
                        // Map has a key matching a field name - treat as single item
                        feuilletage::ContextValue::array(vec![value.clone()], value.context().clone())
                    } else {
                        // No key matches - split into single-key maps
                        was_from_map = true;
                        let items: Vec<feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>> = map.iter()
                            .map(|(k, v)| {
                                let mut single_map = __FeuilletageIndexMap::default();
                                single_map.insert(k.clone(), v.clone());
                                feuilletage::ContextValue::object(single_map, v.context().clone())
                            })
                            .collect();
                        feuilletage::ContextValue::array(items, value.context().clone())
                    }
                }
                #single_value_arm
            };
        }
    } else if has_allow_single {
        // Only allow_single (no allow_map)
        quote! {
            let was_from_map = false;
            let array_value = match value {
                feuilletage::ContextValue::Array(_, _) => value.clone(),
                _ => feuilletage::ContextValue::array(vec![value.clone()], value.context().clone()),
            };
        }
    } else {
        quote! {
            let was_from_map = false;
            let array_value = value.clone();
        }
    };

    // Determine if sorting will be applied
    let has_sorting = attrs
        .allow_map
        .as_ref()
        .is_some_and(|c| c.order_by.is_some() || c.order_by_fn.is_some())
        || attrs.order_by.is_some()
        || attrs.order_by_fn.is_some();

    // Get on_error mode
    let on_error = attrs.on_error.unwrap_or_default();

    // Generate error handling based on on_error mode
    let (error_init, error_handle_deser, post_loop_check) = match on_error {
        OnErrorMode::Fail => (
            quote! {},
            quote! {
                tracker.pop();
                return Err(e);
            },
            quote! {},
        ),
        OnErrorMode::Default => (
            quote! { let mut __vec_had_error = false; },
            quote! {
                tracker.record(e.clone());
                __vec_had_error = true;
            },
            quote! {
                if __vec_had_error {
                    result = Vec::new();
                }
            },
        ),
        OnErrorMode::Skip => (
            quote! {},
            quote! {
                tracker.record(e.clone());
            },
            quote! {},
        ),
    };

    // Generate deserialization code
    let deserialization_code = if has_sorting {
        let elem_ty = element_type.expect("Vec should have inner type");
        quote! {
            #error_init
            #[allow(unused_mut)]
            let mut result: Vec<#elem_ty> = match array_value {
                feuilletage::ContextValue::Array(arr, _) => {
                    let mut vec = Vec::new();
                    for (i, item) in arr.iter().enumerate() {
                        tracker.push_index(i);
                        match feuilletage::FromContextValue::from_context_value(item, tracker) {
                            Ok(v) => vec.push(v),
                            Err(e) => {
                                #error_handle_deser
                            }
                        }
                        tracker.pop();
                    }
                    vec
                }
                _ => Vec::new()
            };
            #post_loop_check
        }
    } else {
        quote! {
            #error_init
            #[allow(unused_mut)]
            let mut result: Vec<_> = match array_value {
                feuilletage::ContextValue::Array(arr, _) => {
                    let mut vec = Vec::new();
                    for (i, item) in arr.iter().enumerate() {
                        tracker.push_index(i);
                        match feuilletage::FromContextValue::from_context_value(item, tracker) {
                            Ok(v) => vec.push(v),
                            Err(e) => {
                                #error_handle_deser
                            }
                        }
                        tracker.pop();
                    }
                    vec
                }
                _ => Vec::new()
            };
            #post_loop_check
        }
    };

    // Generate sorting code (for allow_map with order_by or order_by_fn, or standalone order_by/order_by_fn)
    let sorting_code = if let Some(ref allow_map_config) = attrs.allow_map {
        // Explicit allow_map config - check its order_by/order_by_fn
        if let Some(ref order_by_field) = allow_map_config.order_by {
            let field_ident: proc_macro2::TokenStream = order_by_field.parse().unwrap();
            quote! {
                if was_from_map {
                    result.sort_by(|a, b| a.#field_ident.cmp(&b.#field_ident));
                }
            }
        } else if let Some(ref order_by_fn_name) = allow_map_config.order_by_fn {
            let fn_ident: proc_macro2::TokenStream = order_by_fn_name.parse().unwrap();
            quote! {
                if was_from_map {
                    result.sort_by(#fn_ident);
                }
            }
        } else {
            quote! {}
        }
    } else if let Some(ref order_by_field) = attrs.order_by {
        // Standalone order_by (for flag form allow_map)
        let field_ident: proc_macro2::TokenStream = order_by_field.parse().unwrap();
        quote! {
            if was_from_map {
                result.sort_by(|a, b| a.#field_ident.cmp(&b.#field_ident));
            }
        }
    } else if let Some(ref order_by_fn_name) = attrs.order_by_fn {
        // Standalone order_by_fn (for flag form allow_map)
        let fn_ident: proc_macro2::TokenStream = order_by_fn_name.parse().unwrap();
        quote! {
            if was_from_map {
                result.sort_by(#fn_ident);
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #normalization_code
        #deserialization_code
        #sorting_code
        let #field_name = result.into_iter().collect();
    }
}

/// Represents a fallback chain for resolution
struct FallbackChain {
    /// Fields in this chain (in resolution order)
    fields: Vec<String>,
    /// Whether this chain contains a cycle
    has_cycle: bool,
    /// Field that has a default (if any in the chain)
    default_field: Option<String>,
}

/// Build fallback graph and find connected components
/// Returns a map of field_name -> FallbackChain for fields involved in fallback relationships
fn analyze_fallback_graph(
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
) -> std::collections::HashMap<String, FallbackChain> {
    use std::collections::{HashMap, HashSet};

    // Build the fallback graph: field -> fallback_target
    let mut fallback_map: HashMap<String, String> = HashMap::new();
    let mut reverse_map: HashMap<String, Vec<String>> = HashMap::new(); // target -> sources
    let mut has_default: HashMap<String, bool> = HashMap::new();

    for (_, field_name_str, _, attrs) in field_infos {
        has_default.insert(field_name_str.clone(), attrs.default_value.is_some());

        if let Some(ref target) = attrs.fallback {
            fallback_map.insert(field_name_str.clone(), target.clone());
            reverse_map
                .entry(target.clone())
                .or_default()
                .push(field_name_str.clone());
        }
    }

    // Find all fields involved in fallback relationships (either as source or target)
    let mut involved_fields: HashSet<String> = HashSet::new();
    for (source, target) in &fallback_map {
        involved_fields.insert(source.clone());
        involved_fields.insert(target.clone());
    }

    // For each field with a fallback, trace the chain and detect cycles
    let mut result: HashMap<String, FallbackChain> = HashMap::new();

    for field in &involved_fields {
        if result.contains_key(field) {
            continue; // Already processed as part of another chain
        }

        // Trace the fallback chain starting from this field
        let mut chain: Vec<String> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut current = field.clone();
        let mut has_cycle = false;
        let mut default_field: Option<String> = None;

        loop {
            if visited.contains(&current) {
                // Cycle detected
                has_cycle = true;
                break;
            }

            chain.push(current.clone());
            visited.insert(current.clone());

            // Check if this field has a default
            if has_default.get(&current).copied().unwrap_or(false) && default_field.is_none() {
                default_field = Some(current.clone());
            }

            // Follow the fallback chain
            if let Some(next) = fallback_map.get(&current) {
                current = next.clone();
            } else {
                // End of chain
                break;
            }
        }

        // Store the chain info for all fields in this chain
        let chain_info = FallbackChain {
            fields: chain.clone(),
            has_cycle,
            default_field,
        };

        for f in &chain {
            result.insert(
                f.clone(),
                FallbackChain {
                    fields: chain_info.fields.clone(),
                    has_cycle: chain_info.has_cycle,
                    default_field: chain_info.default_field.clone(),
                },
            );
        }
    }

    result
}

/// Generate dependency resolution code for fields with fallback or default_fn with params
/// This unified function handles both types of dependencies
fn generate_dependency_resolution(
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
) -> proc_macro2::TokenStream {
    use std::collections::{HashMap, HashSet};

    // Build a map of field_name -> (ident, type, attrs)
    let field_map: HashMap<String, (&syn::Ident, &syn::Type, &FieldConfigAttributes)> = field_infos
        .iter()
        .map(|(ident, name_str, ty, attrs)| (name_str.clone(), (ident, ty, attrs)))
        .collect();

    // Build fallback graph for cycle detection
    let fallback_chains = analyze_fallback_graph(field_infos);

    // Find all fields that need deferred resolution
    // These are fields with fallback OR default_fn with params
    let fields_needing_resolution: Vec<_> = field_infos
        .iter()
        .filter(|(_, _, _, attrs)| {
            if attrs.fallback.is_some() {
                return true;
            }
            if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
                let (_, params) = parse_default_fn_value(fn_str);
                if !params.is_empty() {
                    return true;
                }
            }
            false
        })
        .collect();

    if fields_needing_resolution.is_empty() {
        return quote! {};
    }

    // Build set of field names that need resolution
    let needs_resolution: HashSet<String> = fields_needing_resolution
        .iter()
        .map(|(_, name, _, _)| name.clone())
        .collect();

    // Build dependency map: field -> list of fields it depends on
    let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
    for (_, field_name_str, _, attrs) in &fields_needing_resolution {
        let mut deps = Vec::new();

        // Add fallback target as dependency
        if let Some(ref target) = attrs.fallback {
            deps.push(target.clone());
        }

        // Add default_fn params as dependencies
        if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
            let (_, params) = parse_default_fn_value(fn_str);
            for param in params {
                if !deps.contains(&param) {
                    deps.push(param);
                }
            }
        }

        dependencies.insert(field_name_str.clone(), deps);
    }

    // Topological sort: process fields whose dependencies are satisfied first
    // Simple approach: sort by number of dependencies that also need resolution
    let empty_deps: Vec<String> = vec![];
    let mut sorted_fields = fields_needing_resolution.clone();
    sorted_fields.sort_by(|a, b| {
        let a_deps = dependencies.get(&a.1).unwrap_or(&empty_deps);
        let b_deps = dependencies.get(&b.1).unwrap_or(&empty_deps);

        // Count how many dependencies also need resolution
        let a_unresolved = a_deps
            .iter()
            .filter(|d| needs_resolution.contains(*d))
            .count();
        let b_unresolved = b_deps
            .iter()
            .filter(|d| needs_resolution.contains(*d))
            .count();

        a_unresolved.cmp(&b_unresolved)
    });

    // Helper to convert default value with proper string handling
    let convert_default = |dv: &DefaultValue, ty: &syn::Type| -> proc_macro2::TokenStream {
        match dv {
            DefaultValue::Explicit(expr) => {
                let converted = convert_string_default(expr, ty);
                converted.parse::<proc_macro2::TokenStream>().unwrap()
            }
            DefaultValue::UseDefault => quote! { Default::default() },
            DefaultValue::Function(fn_str) => {
                // Check if it has params - if so, this should be handled specially
                let (fn_name, params) = parse_default_fn_value(fn_str);
                if params.is_empty() {
                    let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
                    quote! { #fn_ident() }
                } else {
                    // This case is handled in the resolution code below
                    // Just return a placeholder that will be replaced
                    quote! { unreachable!() }
                }
            }
        }
    };

    // Generate resolution code for each field
    let resolution_code: Vec<proc_macro2::TokenStream> = sorted_fields.iter().map(|(field_ident, field_name_str, field_type, attrs)| {
        let raw_field_name = syn::Ident::new(&format!("__raw_{}", field_name_str), field_ident.span());
        let field_is_option = is_option_type(field_type);

        // Check if this field has default_fn with params
        let default_fn_params = if let Some(DefaultValue::Function(fn_str)) = &attrs.default_value {
            let (fn_name, params) = parse_default_fn_value(fn_str);
            if !params.is_empty() {
                Some((fn_name, params))
            } else {
                None
            }
        } else {
            None
        };

        // If field has fallback, use fallback logic
        if let Some(ref fallback_target) = attrs.fallback {
            let fallback_target_ident = syn::Ident::new(fallback_target, field_ident.span());

            // Get chain info for cycle detection
            let chain_info = fallback_chains.get(field_name_str);
            let has_cycle = chain_info.map(|c| c.has_cycle).unwrap_or(false);

            // Check if fallback target also needs resolution
            let target_needs_resolution = needs_resolution.contains(fallback_target);

            // Generate the default expression if this field has one (without params)
            let field_default = attrs.default_value.as_ref().and_then(|dv| {
                if let DefaultValue::Function(fn_str) = dv {
                    let (_, params) = parse_default_fn_value(fn_str);
                    if !params.is_empty() {
                        return None; // Has params, handled separately
                    }
                }
                Some(convert_default(dv, field_type))
            });

            // Walk the fallback chain to find any default
            let chain_default = {
                let mut current_field = fallback_target.clone();
                let mut found_default: Option<proc_macro2::TokenStream> = None;
                let mut visited = std::collections::HashSet::new();

                while let Some((_, target_ty, target_attrs)) = field_map.get(&current_field) {
                    if visited.contains(&current_field) {
                        break;
                    }
                    visited.insert(current_field.clone());

                    if let Some(ref dv) = target_attrs.default_value {
                        // Skip default_fn with params in chain
                        if let DefaultValue::Function(fn_str) = dv {
                            let (_, params) = parse_default_fn_value(fn_str);
                            if params.is_empty() {
                                found_default = Some(convert_default(dv, target_ty));
                                break;
                            }
                        } else {
                            found_default = Some(convert_default(dv, target_ty));
                            break;
                        }
                    }

                    if let Some(ref next_target) = target_attrs.fallback {
                        current_field = next_target.clone();
                    } else {
                        break;
                    }
                }
                found_default
            };

            // Check if the fallback target type is Option
            let target_is_option = field_map.get(fallback_target)
                .map(|(_, ty, _)| is_option_type(ty))
                .unwrap_or(false);

            // Determine what to use when both field and fallback are None
            let final_fallback = if let Some(ref own_default) = field_default {
                quote! { #own_default }
            } else if let Some((ref fn_name, ref params)) = default_fn_params {
                // Use default_fn with params as final fallback
                let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
                let param_idents: Vec<syn::Ident> = params.iter()
                    .map(|p| syn::Ident::new(p, field_ident.span()))
                    .collect();
                quote! { #fn_ident(#(&#param_idents),*) }
            } else if let Some(ref ch_default) = chain_default {
                quote! { #ch_default }
            } else if field_is_option {
                quote! { None }
            } else {
                quote! {
                    {
                        let error = feuilletage::Error::InvalidValue {
                            path: tracker.current_path(),
                            message: format!(
                                "Field '{}' is missing and its fallback chain (to '{}') has no values or defaults",
                                #field_name_str,
                                #fallback_target
                            ),
                        };
                        tracker.record(error.clone());
                        return Err(error);
                    }
                }
            };

            // Generate raw fallback name for accessing raw values
            let raw_fallback_name = syn::Ident::new(&format!("__raw_{}", fallback_target), field_ident.span());

            // Generate resolution code based on whether target needs resolution and type differences
            if target_needs_resolution && has_cycle {
                // Cycle case - use raw values
                if target_is_option && !field_is_option {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => match #raw_fallback_name.as_ref() {
                                Some(Some(__val)) => __val.clone(),
                                _ => #final_fallback,
                            },
                        };
                    }
                } else if !target_is_option && field_is_option {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => #raw_fallback_name.as_ref().map(|v| v.clone()),
                        };
                    }
                } else {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => match #raw_fallback_name.as_ref() {
                                Some(__val) => __val.clone(),
                                None => #final_fallback,
                            },
                        };
                    }
                }
            } else {
                // Non-cycle case - use resolved variable
                if target_is_option && !field_is_option {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => match #fallback_target_ident.as_ref() {
                                Some(__val) => __val.clone(),
                                None => #final_fallback,
                            },
                        };
                    }
                } else if !target_is_option && field_is_option {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => Some(#fallback_target_ident.clone()),
                        };
                    }
                } else {
                    quote! {
                        let #field_ident: #field_type = match #raw_field_name.as_ref() {
                            Some(__val) => __val.clone(),
                            None => #fallback_target_ident.clone(),
                        };
                    }
                }
            }
        } else if let Some((ref fn_name, ref params)) = default_fn_params {
            // Field has default_fn with params (no fallback)
            let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
            let param_idents: Vec<syn::Ident> = params.iter()
                .map(|p| syn::Ident::new(p, field_ident.span()))
                .collect();

            // Generate the default_fn call
            let default_fn_call = quote! { #fn_ident(#(&#param_idents),*) };

            // Check if field has a static default as well (default_fn with params takes precedence)
            // Actually, for default_fn with params, that IS the default
            if field_is_option {
                quote! {
                    let #field_ident: #field_type = match #raw_field_name {
                        Some(__val) => __val,
                        None => Some(#default_fn_call),
                    };
                }
            } else {
                quote! {
                    let #field_ident: #field_type = match #raw_field_name {
                        Some(__val) => __val,
                        None => #default_fn_call,
                    };
                }
            }
        } else {
            // This shouldn't happen - field is in the list but has neither fallback nor default_fn with params
            quote! {}
        }
    }).collect();

    quote! {
        // Dependency resolution phase (fallback and default_fn with params)
        #(#resolution_code)*
    }
}

/// Generate raw value extraction for a field (returns Option<T>)
/// This is used for fields involved in fallback relationships
fn generate_raw_field_extraction(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    let raw_var_name = syn::Ident::new(&format!("__raw_{}", field_name_str), field_name.span());

    // Get the primary key (rename or field name)
    let primary_key = attrs.rename.as_deref().unwrap_or(field_name_str);

    // Build the key lookup chain with aliases
    // Build the lookup chain: obj.get("key1").or_else(|| obj.get("key2")).or_else(|| ...)
    let lookup_chain = {
        let mut chain = quote! { obj.get(#primary_key) };
        for alias in &attrs.aliases {
            chain = quote! { #chain.or_else(|| obj.get(#alias)) };
        }
        chain
    };

    // For Option<T> fields, we need to handle the inner type
    let is_option = is_option_type(field_type);

    if is_option {
        quote! {
            let mut #raw_var_name: Option<#field_type> = {
                tracker.push_field(#field_name_str);
                let result = match #lookup_chain {
                    Some(field_value) => {
                        match feuilletage::FromContextValue::from_context_value(field_value, tracker) {
                            Ok(v) => Some(Some(v)),
                            Err(_) => Some(None),
                        }
                    }
                    None => None, // Field not present - might use fallback
                };
                tracker.pop();
                result
            };
        }
    } else {
        quote! {
            let mut #raw_var_name: Option<#field_type> = {
                tracker.push_field(#field_name_str);
                let result = match #lookup_chain {
                    Some(field_value) => {
                        match feuilletage::FromContextValue::from_context_value(field_value, tracker) {
                            Ok(v) => Some(v),
                            Err(_) => None,
                        }
                    }
                    None => None, // Field not present - might use fallback
                };
                tracker.pop();
                result
            };
        }
    }
}

/// Generate raw value extraction AND assignment for a fallback target field
/// This is for fields that are referenced by other fields' fallback but don't have their own fallback
fn generate_raw_field_with_assignment(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &syn::Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    let raw_var_name = syn::Ident::new(&format!("__raw_{}", field_name_str), field_name.span());

    // Get the primary key (rename or field name)
    let primary_key = attrs.rename.as_deref().unwrap_or(field_name_str);

    // Build the key lookup chain with aliases
    // Build the lookup chain: obj.get("key1").or_else(|| obj.get("key2")).or_else(|| ...)
    let lookup_chain = {
        let mut chain = quote! { obj.get(#primary_key) };
        for alias in &attrs.aliases {
            chain = quote! { #chain.or_else(|| obj.get(#alias)) };
        }
        chain
    };

    // Generate default value tokens if present (with proper string conversion)
    let default_tokens = attrs.default_value.as_ref().map(|dv| match dv {
        DefaultValue::Explicit(expr) => {
            let converted = convert_string_default(expr, field_type);
            converted.parse::<proc_macro2::TokenStream>().unwrap()
        }
        DefaultValue::UseDefault => quote! { Default::default() },
        DefaultValue::Function(fn_name) => {
            let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
            quote! { #fn_ident() }
        }
    });

    // For Option<T> fields, we need to handle the inner type
    let is_option = is_option_type(field_type);

    // Generate the missing field handling
    let missing_field_handling = if let Some(ref default_expr) = default_tokens {
        quote! { #default_expr }
    } else if is_option {
        quote! { None }
    } else {
        quote! {
            {
                let error = feuilletage::Error::InvalidValue {
                    path: format!("{}.{}", tracker.current_path(), #field_name_str),
                    message: format!("required field '{}' was not provided", #field_name_str),
                };
                tracker.record(error.clone());
                return Err(error);
            }
        }
    };

    if is_option {
        quote! {
            let mut #raw_var_name: Option<#field_type> = {
                tracker.push_field(#field_name_str);
                let result = match #lookup_chain {
                    Some(field_value) => {
                        match feuilletage::FromContextValue::from_context_value(field_value, tracker) {
                            Ok(v) => Some(Some(v)),
                            Err(_) => Some(None),
                        }
                    }
                    None => None,
                };
                tracker.pop();
                result
            };
            // Assign the field (this is a fallback target, but doesn't have its own fallback)
            let #field_name: #field_type = #raw_var_name.clone().unwrap_or_else(|| #missing_field_handling);
        }
    } else {
        quote! {
            let mut #raw_var_name: Option<#field_type> = {
                tracker.push_field(#field_name_str);
                let result = match #lookup_chain {
                    Some(field_value) => {
                        match feuilletage::FromContextValue::from_context_value(field_value, tracker) {
                            Ok(v) => Some(v),
                            Err(_) => None,
                        }
                    }
                    None => None,
                };
                tracker.pop();
                result
            };
            // Assign the field (this is a fallback target, but doesn't have its own fallback)
            let #field_name: #field_type = match #raw_var_name.clone() {
                Some(v) => v,
                None => #missing_field_handling,
            };
        }
    }
}

/// Generate MutabilityInfo implementation for a struct.
///
/// This generates a function that returns a map of field names (using renamed keys if applicable)
/// to their allowed Levels. Fields without mutable_by constraints are not included.
fn generate_mutability_info_impl(
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
    name: &syn::Ident,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    // Collect fields with mutable_by constraints, including aliases because
    // aliases are also accepted input paths.
    let constrained_fields: Vec<_> = field_infos
        .iter()
        .flat_map(|(_, field_name_str, _, attrs)| {
            let Some(levels) = attrs.mutable_by.as_ref() else {
                return Vec::new();
            };
            let mut keys = vec![attrs.rename.as_ref().unwrap_or(field_name_str).clone()];
            keys.extend(attrs.aliases.iter().cloned());
            keys.into_iter()
                .map(|key| (key, levels.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    let nested_fields: Vec<_> = field_infos
        .iter()
        .filter(|(_, _, _, attrs)| attrs.nested)
        .map(|(_, field_name_str, field_type, attrs)| {
            let prefixes = if attrs.flatten {
                vec![String::new()]
            } else {
                let mut prefixes = vec![attrs.rename.as_ref().unwrap_or(field_name_str).clone()];
                prefixes.extend(attrs.aliases.iter().cloned());
                prefixes
            };
            (field_type, prefixes)
        })
        .collect();

    // Generate static arrays for each constrained field's allowed level names
    let static_arrays: Vec<proc_macro2::TokenStream> = constrained_fields
        .iter()
        .enumerate()
        .map(|(idx, (_, levels))| {
            let array_name = syn::Ident::new(
                &format!("__MUTABLE_BY_LEVELS_{}", idx),
                proc_macro2::Span::call_site(),
            );
            // levels are already strings, just quote them directly
            quote! {
                static #array_name: &[&'static str] = &[#(#levels),*];
            }
        })
        .collect();

    // Generate the map insertions
    let map_insertions: Vec<proc_macro2::TokenStream> = constrained_fields
        .iter()
        .enumerate()
        .map(|(idx, (key, _))| {
            let array_name = syn::Ident::new(
                &format!("__MUTABLE_BY_LEVELS_{}", idx),
                proc_macro2::Span::call_site(),
            );
            quote! {
                map.insert(#key.into(), #array_name);
            }
        })
        .collect();

    let nested_insertions = nested_fields.iter().map(|(field_type, prefixes)| {
        let insertions = prefixes.iter().map(|prefix| {
            if prefix.is_empty() {
                quote! {
                    map.extend(<#field_type as feuilletage::MutabilityInfo>::mutability_constraints());
                }
            } else {
                quote! {
                    for (mut path, allowed_levels) in
                        <#field_type as feuilletage::MutabilityInfo>::mutability_constraints()
                    {
                        path.insert(0, '.');
                        path.insert_str(0, #prefix);
                        map.insert(path, allowed_levels);
                    }
                }
            }
        });
        quote! { #(#insertions)* }
    });

    // Use feuilletage's re-exported HashMap which works in both std and no_std
    quote! {
        impl #impl_generics feuilletage::MutabilityInfo for #name #ty_generics #where_clause {
            fn mutability_constraints() -> feuilletage::MutabilityConstraints {
                #(#static_arrays)*
                let mut map = feuilletage::MutabilityHashMap::new();
                #(#map_insertions)*
                #(#nested_insertions)*
                map
            }
        }
    }
}

/// Generate AllowMapKeys implementation for a struct deriving `feuilletage::Config`.
///
/// This generates a function that returns the list of field names (including aliases)
/// for the key field. Used by `Vec<T>` with `allow_map` to detect whether a map input
/// should be treated as a single struct instance or split into multiple instances.
///
/// - If the struct has `#[feuilletage(allow_map(key = field))]`, returns that field's names
///   (including rename and aliases).
/// - Otherwise, returns `&[]` (no key detection — objects are always treated as maps).
fn generate_allow_map_keys_impl(
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
    container_attrs: &ContainerAttributes,
    name: &syn::Ident,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    // Check if struct has allow_map(key = ...) attribute
    let unique_names: Vec<String> = match &container_attrs.struct_allow_map {
        Some(config) if config.key_field.is_some() => {
            let key_field_name = config.key_field.as_ref().unwrap();

            // Find the field info for the key field
            let key_field_info = field_infos.iter().find(|(_, field_name_str, _, attrs)| {
                // Check if this field matches the key field name (considering rename and aliases)
                let primary_key = attrs.rename.as_ref().unwrap_or(field_name_str);
                primary_key == key_field_name
                    || field_name_str == key_field_name
                    || attrs.aliases.contains(&key_field_name.to_string())
            });

            // Collect all names for the key field (primary name + aliases)
            let key_names: Vec<String> = if let Some((_, field_name_str, _, attrs)) = key_field_info
            {
                let mut names = Vec::new();
                // Add the primary key (rename or field name)
                let primary_key = attrs.rename.as_ref().unwrap_or(field_name_str).clone();
                names.push(primary_key);
                // Add the original field name if different from primary
                if attrs.rename.is_some() && attrs.rename.as_ref() != Some(field_name_str) {
                    names.push(field_name_str.clone());
                }
                // Add aliases
                names.extend(attrs.aliases.iter().cloned());
                names
            } else {
                // Field not found, just use the key_field_name
                vec![key_field_name.clone()]
            };

            // Deduplicate
            let mut deduped: Vec<String> = Vec::new();
            for name_str in key_names {
                if !deduped.contains(&name_str) {
                    deduped.push(name_str);
                }
            }
            deduped
        }
        // No allow_map key — return empty (no single-item detection)
        _ => Vec::new(),
    };

    quote! {
        impl #impl_generics feuilletage::AllowMapKeys for #name #ty_generics #where_clause {
            fn map_key_fields() -> &'static [&'static str] {
                &[#(#unique_names),*]
            }
        }
    }
}

/// Generate template resolution code for fields with #[feuilletage(template)] attribute.
///
/// Template resolution happens after all fields are deserialized. For each template field:
/// 1. Build a map of all field names to their string values
/// 2. Extract %{field} references from the template string
/// 3. Interpolate the references with the resolved field values
/// 4. Shadow the original variable with the interpolated value
fn generate_template_resolution(
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
) -> proc_macro2::TokenStream {
    // Collect template fields and their info
    let template_fields: Vec<_> = field_infos
        .iter()
        .filter(|(_, _, _, attrs)| attrs.template)
        .collect();

    if template_fields.is_empty() {
        return quote! {};
    }

    // Generate code to build the field values map
    // We convert all fields to string representation for template interpolation
    let field_value_insertions: Vec<proc_macro2::TokenStream> = field_infos
        .iter()
        .map(|(field_name, field_name_str, field_type, _attrs)| {
            // Generate the string conversion based on field type
            let string_conversion = generate_field_to_string(field_name, field_type);
            quote! {
                __template_map.insert(#field_name_str.to_string(), #string_conversion);
            }
        })
        .collect();

    // Generate code to interpolate each template field
    // We use variable shadowing (let #field_name = ...) instead of reassignment
    // After interpolation, we update the map so that subsequent template fields can reference the resolved value
    let template_interpolations: Vec<proc_macro2::TokenStream> = template_fields
        .iter()
        .map(|(field_name, field_name_str, _field_type, attrs)| {
            let vec_delimiter = attrs.template_vec_delimiter.as_deref().unwrap_or(",");
            quote! {
                // Interpolate template for field and shadow the original binding
                let #field_name = {
                    let template_str = &#field_name;
                    match feuilletage::interpolate_template(template_str, &__template_field_values, #vec_delimiter) {
                        Ok(interpolated) => {
                            // Update the map so that subsequent template fields can reference the resolved value
                            __template_field_values.insert(#field_name_str.to_string(), interpolated.clone());
                            interpolated
                        }
                        Err(e) => {
                            tracker.record(feuilletage::Error::InvalidValue {
                                path: format!("{}.{}", tracker.current_path(), #field_name_str),
                                message: format!("Template interpolation error: {}", e),
                            });
                            // On error, keep the original value
                            template_str.clone()
                        }
                    }
                };
            }
        })
        .collect();

    quote! {
        // Template resolution phase
        use std::collections::HashMap;

        // Build map of all field values as strings (mutable so we can update after each template resolves)
        let mut __template_field_values: HashMap<String, String> = {
            let mut __template_map = HashMap::new();
            #(#field_value_insertions)*
            __template_map
        };

        // Interpolate each template field (shadowing the original bindings)
        // After each interpolation, the map is updated so subsequent templates can reference resolved values
        #(#template_interpolations)*
    }
}

/// Generate code to convert a field value to its string representation for template interpolation.
fn generate_field_to_string(
    field_name: &syn::Ident,
    field_type: &syn::Type,
) -> proc_macro2::TokenStream {
    let type_name = get_type_name(field_type);

    match type_name.as_str() {
        "String" => quote! { #field_name.clone() },
        "Option" => {
            quote! {
                match &#field_name {
                    Some(v) => format!("{}", v),
                    None => String::new(),
                }
            }
        }
        "Vec" => {
            quote! {
                #field_name.iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(",")
            }
        }
        "PathBuf" => quote! { #field_name.to_string_lossy().to_string() },
        // Default: use Display trait (works for i32, i64, f64, bool, etc.)
        _ => quote! { format!("{}", #field_name) },
    }
}

/// Generate value wrapping code for scalar_as, array_as, and struct-level allow_map attributes
fn generate_value_wrapping(
    container_attrs: &ContainerAttributes,
    field_infos: &[(syn::Ident, String, syn::Type, FieldConfigAttributes)],
) -> proc_macro2::TokenStream {
    let has_scalar_as = container_attrs.scalar_as.is_some();
    let has_array_as = container_attrs.array_as.is_some();
    let has_struct_allow_map = container_attrs
        .struct_allow_map
        .as_ref()
        .map(|c| c.key_field.is_some())
        .unwrap_or(false);

    if !has_scalar_as && !has_array_as && !has_struct_allow_map {
        return quote! {};
    }

    // Collect all field names and aliases for detection (used by struct allow_map)
    let all_field_keys: Vec<String> = field_infos
        .iter()
        .flat_map(|(_, field_name_str, _, attrs)| {
            let mut keys = Vec::new();
            // Primary key is rename if specified, otherwise field name
            let primary_key = attrs.rename.as_ref().unwrap_or(field_name_str).clone();
            keys.push(primary_key);
            // Also include aliases
            keys.extend(attrs.aliases.iter().cloned());
            // Include original field name if renamed
            if attrs.rename.is_some() {
                keys.push(field_name_str.clone());
            }
            keys
        })
        .collect();

    // Generate scalar wrapping code
    let scalar_wrap = if has_scalar_as {
        let scalar_field = container_attrs.scalar_as.as_ref().unwrap();
        quote! {
            feuilletage::ContextValue::String(_, _) | feuilletage::ContextValue::Int(_, _) | feuilletage::ContextValue::Float(_, _) | feuilletage::ContextValue::Bool(_, _) => {
                // Scalar input - wrap as {scalar_as_field: value}
                let mut wrapped_obj = __FeuilletageIndexMap::default();
                wrapped_obj.insert(#scalar_field.to_string(), value.clone());
                __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
            }
        }
    } else {
        quote! {}
    };

    // Generate array wrapping code
    let array_wrap = if has_array_as {
        let array_field = container_attrs.array_as.as_ref().unwrap();
        quote! {
            feuilletage::ContextValue::Array(_, _) => {
                // Array input - wrap as {array_as_field: value}
                let mut wrapped_obj = __FeuilletageIndexMap::default();
                wrapped_obj.insert(#array_field.to_string(), value.clone());
                __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
            }
        }
    } else {
        quote! {}
    };

    // Generate object wrapping code for struct allow_map
    let object_wrap = if has_struct_allow_map {
        let allow_map_config = container_attrs.struct_allow_map.as_ref().unwrap();
        let key_field = allow_map_config.key_field.as_ref().unwrap();

        // If struct allow_map has scalar_as, use it; otherwise, try container's scalar_as
        let value_field = allow_map_config
            .scalar_as_field
            .as_ref()
            .or(container_attrs.scalar_as.as_ref());

        let value_wrap_code = if let Some(scalar_as_field) = value_field {
            quote! {
                // Check if the single value is a scalar - wrap it
                match single_value {
                    feuilletage::ContextValue::String(_, _) | feuilletage::ContextValue::Int(_, _) | feuilletage::ContextValue::Float(_, _) | feuilletage::ContextValue::Bool(_, _) => {
                        // Scalar value - create {key_field: key, scalar_as_field: value}
                        let mut wrapped_obj = __FeuilletageIndexMap::default();
                        wrapped_obj.insert(#key_field.to_string(),
                            feuilletage::ContextValue::string(single_key.clone(), single_value.context().clone()));
                        wrapped_obj.insert(#scalar_as_field.to_string(), single_value.clone());
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
                    }
                    feuilletage::ContextValue::Null(_) => {
                        // Null value - create {key_field: key} without scalar_as field
                        let mut wrapped_obj = __FeuilletageIndexMap::default();
                        wrapped_obj.insert(#key_field.to_string(),
                            feuilletage::ContextValue::string(single_key.clone(), single_value.context().clone()));
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
                    }
                    feuilletage::ContextValue::Object(inner_obj, _) => {
                        // Object value - inject key into the object
                        let mut wrapped_obj = inner_obj.clone();
                        wrapped_obj.insert(#key_field.to_string(),
                            feuilletage::ContextValue::string(single_key.clone(), single_value.context().clone()));
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
                    }
                    _ => __FeuilletageCow::Borrowed(value),
                }
            }
        } else {
            quote! {
                // No scalar_as specified, only transform objects and nulls
                match single_value {
                    feuilletage::ContextValue::Object(inner_obj, _) => {
                        // Object value - inject key into the object
                        let mut wrapped_obj = inner_obj.clone();
                        wrapped_obj.insert(#key_field.to_string(),
                            feuilletage::ContextValue::string(single_key.clone(), single_value.context().clone()));
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
                    }
                    feuilletage::ContextValue::Null(_) => {
                        // Null value - create {key_field: key}
                        let mut wrapped_obj = __FeuilletageIndexMap::default();
                        wrapped_obj.insert(#key_field.to_string(),
                            feuilletage::ContextValue::string(single_key.clone(), single_value.context().clone()));
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, value.context().clone()))
                    }
                    _ => __FeuilletageCow::Borrowed(value),
                }
            }
        };

        quote! {
            feuilletage::ContextValue::Object(map, _) if map.len() == 1 => {
                // Single-key map - check if key matches any field name
                let (single_key, single_value) = map.iter().next().unwrap();
                let known_fields: &[&str] = &[#(#all_field_keys),*];
                let is_field_key = known_fields.iter().any(|f| *f == single_key);

                if is_field_key {
                    // Key matches a field name - this is a regular object, don't transform
                    __FeuilletageCow::Borrowed(value)
                } else {
                    // Key is NOT a field name - apply allow_map transformation
                    #value_wrap_code
                }
            }
        }
    } else {
        quote! {}
    };

    // Combine all wrapping code
    let has_any_wrap = has_scalar_as || has_array_as || has_struct_allow_map;

    if has_any_wrap {
        quote! {
            // Wrap inputs based on container attributes
            let value: __FeuilletageCow<'_, feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>> = match value {
                #scalar_wrap
                #array_wrap
                #object_wrap
                _ => __FeuilletageCow::Borrowed(value),
            };
            let value = value.as_ref();
        }
    } else {
        quote! {}
    }
}

/// Generate default value code for a skipped field.
/// Skipped fields are not deserialized from input - they use the default value directly.
fn generate_skip_field_default(
    field_name: &syn::Ident,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Determine the default value:
    // 1. Use explicit default if provided
    // 2. Use default_fn if provided
    // 3. Use Default::default() (from #[feuilletage(default)] or implicit)
    let default_tokens = if let Some(ref dv) = attrs.default_value {
        match dv {
            DefaultValue::Explicit(expr) => {
                let converted = convert_string_default(expr, field_type);
                converted.parse::<proc_macro2::TokenStream>().unwrap()
            }
            DefaultValue::UseDefault => quote! { Default::default() },
            DefaultValue::Function(fn_name) => {
                let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                quote! { #fn_ident() }
            }
        }
    } else {
        // No default specified, use Default::default() (may cause compile error if type doesn't impl Default)
        quote! { Default::default() }
    };

    quote! {
        let #field_name: #field_type = #default_tokens;
    }
}

/// Generate deserialization code for a flattened field
fn generate_flatten_deserialization(
    field_name: &syn::Ident,
    field_type: &Type,
) -> proc_macro2::TokenStream {
    quote! {
        let #field_name: #field_type = feuilletage::FromContextValue::from_context_value(&__remaining_value, tracker)?;
    }
}

/// Generate deserialization code for a from_context field
///
/// These fields are populated from context metadata rather than from the input object.
/// Supported paths:
/// - "source.file_path" - PathBuf from source's file_path()
/// - "source.display_name" - String from source's display_name()
/// - "level.name" - String from level's name()
///
/// Note: We get the context from any field in the object (via `obj`) rather than from the
/// root object's context, because after merging configs, the root object may have the
/// context from Config::default() while individual fields have the context from the
/// actual loaded source.
fn generate_from_context_deserialization(
    field_name: &syn::Ident,
    field_type: &Type,
    context_path: &str,
) -> proc_macro2::TokenStream {
    let type_str = quote!(#field_type).to_string();
    let is_option = type_str.starts_with("Option <") || type_str.starts_with("Option<");

    // Get context from any field in the object (they all came from the same source)
    // Fall back to __feuilletage_value.context() if no fields exist
    // NOTE: We reference __feuilletage_value (saved before field deserialization)
    // instead of `value` because field names like `value` can shadow the
    // function parameter. We also avoid calling .context() on Option/Result
    // to prevent conflicts with miette's Context trait.
    let get_context = quote! {
        match obj.values().next() {
            Some(__cv) => __cv.context(),
            None => __feuilletage_value.context(),
        }
    };

    match context_path {
        "source.file_path" => {
            // source.file_path returns Option<&Path>, convert to field type
            if is_option {
                // Option<PathBuf> - use the Option directly
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        ctx.source.file_path().map(|p| p.to_path_buf())
                    };
                }
            } else {
                // PathBuf - require file_path to exist
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        ctx.source.file_path()
                            .map(|p| p.to_path_buf())
                            .ok_or_else(|| feuilletage::Error::MissingField {
                                path: tracker.current_path(),
                                field: stringify!(#field_name).to_string(),
                            })?
                    };
                }
            }
        }
        "source.display_name" => {
            // source.display_name returns String
            if is_option {
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        Some(ctx.source.display_name())
                    };
                }
            } else {
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        ctx.source.display_name()
                    };
                }
            }
        }
        "level.name" => {
            // level.name returns &str
            if is_option {
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        Some(ctx.level.name().to_string())
                    };
                }
            } else {
                quote! {
                    let #field_name: #field_type = {
                        let ctx = #get_context;
                        ctx.level.name().to_string()
                    };
                }
            }
        }
        _ => {
            // This shouldn't happen due to validation in parse_field_config_attributes
            panic!("Unsupported from_context path: {}", context_path);
        }
    }
}

/// Generate deserialization code for a from_context_fn field
///
/// These fields are populated by calling a user-provided function with the context.
/// The function receives &feuilletage::Context<S, L> and returns the field type.
fn generate_from_context_fn_deserialization(
    field_name: &syn::Ident,
    field_type: &Type,
    fn_name: &str,
) -> proc_macro2::TokenStream {
    let fn_ident = syn::Ident::new(fn_name, field_name.span());

    // Get context from any field in the object (they all came from the same source)
    // Fall back to value.context() if no fields exist
    quote! {
        let #field_name: #field_type = {
            let __ctx = obj.values().next().map(|v| v.context()).unwrap_or_else(|| value.context());
            #fn_ident(__ctx)
        };
    }
}

/// Generate skip helper methods for serde serialization (kept for backward compatibility)
fn generate_skip_helpers(
    struct_name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
) -> proc_macro2::TokenStream {
    let mut helpers = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_config_attributes(&field.attrs);

        if attrs.skip_if_empty {
            let helper_name = syn::Ident::new(
                &format!("__feuilletage_skip_if_empty_{}", field_name),
                field_name.span(),
            );

            // Determine the appropriate is_empty check based on type
            let skip_check = get_skip_check_for_type(field_type);

            helpers.push(quote! {
                #[doc(hidden)]
                #[allow(dead_code, clippy::ptr_arg)]
                pub fn #helper_name(value: &#field_type) -> bool {
                    #skip_check
                }
            });
        }

        if attrs.skip_if_empty_recursive {
            let helper_name = syn::Ident::new(
                &format!("__feuilletage_skip_if_empty_recursive_{}", field_name),
                field_name.span(),
            );

            // Generate recursive empty check
            let skip_check = get_skip_check_recursive_for_type(field_type);

            helpers.push(quote! {
                #[doc(hidden)]
                #[allow(dead_code, clippy::ptr_arg)]
                pub fn #helper_name(value: &#field_type) -> bool {
                    #skip_check
                }
            });
        }

        if attrs.skip_if_default {
            let helper_name = syn::Ident::new(
                &format!("__feuilletage_skip_if_default_{}", field_name),
                field_name.span(),
            );

            // Generate default value check
            let skip_check = get_skip_check_for_default(field_type, &attrs);

            helpers.push(quote! {
                #[doc(hidden)]
                #[allow(dead_code, clippy::ptr_arg)]
                pub fn #helper_name(value: &#field_type) -> bool {
                    #skip_check
                }
            });
        }

        if let Some(ref custom_fn) = attrs.skip_if {
            // For custom skip functions, we generate a wrapper that calls the users function
            let helper_name = syn::Ident::new(
                &format!("__feuilletage_skip_if_{}", field_name),
                field_name.span(),
            );
            let custom_fn_ident: proc_macro2::TokenStream = custom_fn.parse().unwrap();

            helpers.push(quote! {
                #[doc(hidden)]
                #[allow(dead_code, clippy::ptr_arg)]
                pub fn #helper_name(value: &#field_type) -> bool {
                    #custom_fn_ident(value)
                }
            });
        }
    }

    if helpers.is_empty() {
        quote! {}
    } else {
        quote! {
            impl #struct_name {
                #(#helpers)*
            }
        }
    }
}

/// Determine the coercion function to use based on the target type
/// Returns (coercion_function_name, optional_cast_type)
fn get_coercion_info(ty: &Type) -> Option<(&'static str, Option<&'static str>)> {
    if is_string_type(ty) {
        Some(("coerce_to_string", None))
    } else if is_bool_type(ty) {
        Some(("coerce_to_bool", None))
    } else if is_signed_int_type(ty) {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                let cast = match name.as_str() {
                    "i64" => None,
                    "i8" => Some("i8"),
                    "i16" => Some("i16"),
                    "i32" => Some("i32"),
                    "isize" => Some("isize"),
                    _ => None,
                };
                return Some(("coerce_to_i64", cast));
            }
        }
        Some(("coerce_to_i64", None))
    } else if is_unsigned_int_type(ty) {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                let cast = match name.as_str() {
                    "u64" => None,
                    "u8" => Some("u8"),
                    "u16" => Some("u16"),
                    "u32" => Some("u32"),
                    "usize" => Some("usize"),
                    _ => None,
                };
                return Some(("coerce_to_u64", cast));
            }
        }
        Some(("coerce_to_u64", None))
    } else if is_float_type(ty) {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                let name = segment.ident.to_string();
                let cast = match name.as_str() {
                    "f64" => None,
                    "f32" => Some("f32"),
                    _ => None,
                };
                return Some(("coerce_to_f64", cast));
            }
        }
        Some(("coerce_to_f64", None))
    } else {
        None
    }
}

/// Generate coercion code for a specific type
/// Returns None if the type is not supported for coercion
fn generate_coercion_code(
    field_type: &Type,
    field_name_str: &str,
) -> Option<proc_macro2::TokenStream> {
    let (coerce_fn, cast) = get_coercion_info(field_type)?;
    let coerce_fn_ident: proc_macro2::TokenStream = format!("feuilletage::coerce::{}", coerce_fn)
        .parse()
        .unwrap();

    let coercion = if let Some(cast_type) = cast {
        let cast_ident: proc_macro2::TokenStream = cast_type.parse().unwrap();
        quote! {
            match #coerce_fn_ident(field_value) {
                Some(v) => v as #cast_ident,
                None => {
                    let error = feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: field_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                }
            }
        }
    } else {
        quote! {
            match #coerce_fn_ident(field_value) {
                Some(v) => v,
                None => {
                    let error = feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: field_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                }
            }
        }
    };

    Some(coercion)
}

/// Generate inline duration deserialization code
/// This handles parsing duration strings to the specified unit with appropriate type conversion
fn generate_duration_deserialization(
    field_type: &Type,
    field_name_str: &str,
    duration_config: &DurationConfig,
    default_tokens: &Option<proc_macro2::TokenStream>,
    validation_code: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    // Nanosecond divisors for each unit
    let ns_divisor: f64 = match duration_config.unit.as_str() {
        "ns" => 1.0,
        "us" => 1_000.0,
        "ms" => 1_000_000.0,
        "s" => 1_000_000_000.0,
        "m" => 60.0 * 1_000_000_000.0,
        "h" => 60.0 * 60.0 * 1_000_000_000.0,
        "d" => 24.0 * 60.0 * 60.0 * 1_000_000_000.0,
        "w" => 7.0 * 24.0 * 60.0 * 60.0 * 1_000_000_000.0,
        _ => 1_000_000_000.0, // Default to seconds
    };

    let ns_divisor_lit = ns_divisor;
    let is_float = is_float_type(field_type);

    // Determine the conversion to the final type
    let final_conversion = if is_float {
        // For float types, return the f64 result directly (may need cast for f32)
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "f32" {
                    quote! { result as f32 }
                } else {
                    quote! { result }
                }
            } else {
                quote! { result }
            }
        } else {
            quote! { result }
        }
    } else {
        // For integer types, truncate and cast
        if is_unsigned_int_type(field_type) {
            if let Type::Path(type_path) = field_type {
                if let Some(segment) = type_path.path.segments.last() {
                    let type_name = segment.ident.to_string();
                    match type_name.as_str() {
                        "u64" => quote! { result as u64 },
                        "u32" => quote! { result as u32 },
                        "u16" => quote! { result as u16 },
                        "u8" => quote! { result as u8 },
                        "usize" => quote! { result as usize },
                        _ => quote! { result as u64 },
                    }
                } else {
                    quote! { result as u64 }
                }
            } else {
                quote! { result as u64 }
            }
        } else {
            // Signed integer types
            if let Type::Path(type_path) = field_type {
                if let Some(segment) = type_path.path.segments.last() {
                    let type_name = segment.ident.to_string();
                    match type_name.as_str() {
                        "i64" => quote! { result as i64 },
                        "i32" => quote! { result as i32 },
                        "i16" => quote! { result as i16 },
                        "i8" => quote! { result as i8 },
                        "isize" => quote! { result as isize },
                        _ => quote! { result as i64 },
                    }
                } else {
                    quote! { result as i64 }
                }
            } else {
                quote! { result as i64 }
            }
        }
    };

    // Handle the integer case specially
    let int_conversion = if is_float {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "f32" {
                    quote! { *i as f32 }
                } else {
                    quote! { *i as f64 }
                }
            } else {
                quote! { *i as f64 }
            }
        } else {
            quote! { *i as f64 }
        }
    } else if is_unsigned_int_type(field_type) {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "u64" => quote! { *i as u64 },
                    "u32" => quote! { *i as u32 },
                    "u16" => quote! { *i as u16 },
                    "u8" => quote! { *i as u8 },
                    "usize" => quote! { *i as usize },
                    _ => quote! { *i as u64 },
                }
            } else {
                quote! { *i as u64 }
            }
        } else {
            quote! { *i as u64 }
        }
    } else {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "i64" => quote! { *i },
                    "i32" => quote! { *i as i32 },
                    "i16" => quote! { *i as i16 },
                    "i8" => quote! { *i as i8 },
                    "isize" => quote! { *i as isize },
                    _ => quote! { *i },
                }
            } else {
                quote! { *i }
            }
        } else {
            quote! { *i }
        }
    };

    let float_conversion = if is_float {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "f32" {
                    quote! { *f as f32 }
                } else {
                    quote! { *f }
                }
            } else {
                quote! { *f }
            }
        } else {
            quote! { *f }
        }
    } else if is_unsigned_int_type(field_type) {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "u64" => quote! { *f as u64 },
                    "u32" => quote! { *f as u32 },
                    "u16" => quote! { *f as u16 },
                    "u8" => quote! { *f as u8 },
                    "usize" => quote! { *f as usize },
                    _ => quote! { *f as u64 },
                }
            } else {
                quote! { *f as u64 }
            }
        } else {
            quote! { *f as u64 }
        }
    } else {
        if let Type::Path(type_path) = field_type {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    "i64" => quote! { *f as i64 },
                    "i32" => quote! { *f as i32 },
                    "i16" => quote! { *f as i16 },
                    "i8" => quote! { *f as i8 },
                    "isize" => quote! { *f as isize },
                    _ => quote! { *f as i64 },
                }
            } else {
                quote! { *f as i64 }
            }
        } else {
            quote! { *f as i64 }
        }
    };

    // Generate complete code with proper error handling
    if let Some(default_expr) = default_tokens {
        // With default - use default on parse error
        quote! {
            {
                let mut __parse_error: Option<feuilletage::Error> = None;
                let mut __temp_value: #field_type = match field_value {
                    feuilletage::ContextValue::String(s, _) => {
                        match feuilletage::transform::parse_duration_to_nanos(s) {
                            Ok(nanos) => {
                                let result = nanos / #ns_divisor_lit;
                                #final_conversion
                            }
                            Err(e) => {
                                let error = feuilletage::Error::InvalidValue {
                                    path: tracker.current_path(),
                                    message: format!("Failed to parse duration for '{}': {}", #field_name_str, e),
                                };
                                tracker.record(error);
                                #default_expr
                            }
                        }
                    }
                    feuilletage::ContextValue::Int(i, _) => {
                        // Integer value - assume it's already in the target unit
                        #int_conversion
                    }
                    feuilletage::ContextValue::Float(f, _) => {
                        // Float value - assume it's already in the target unit
                        #float_conversion
                    }
                    _ => {
                        let error = feuilletage::Error::TypeMismatch {
                            path: tracker.current_path(),
                            expected: format!("string, integer, or float for duration field '{}'", #field_name_str),
                            actual: field_value.type_name().to_string(),
                        };
                        tracker.record(error);
                        #default_expr
                    }
                };
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
    } else {
        // No default - fail on parse error
        quote! {
            {
                let mut __temp_value: #field_type = match field_value {
                    feuilletage::ContextValue::String(s, _) => {
                        match feuilletage::transform::parse_duration_to_nanos(s) {
                            Ok(nanos) => {
                                let result = nanos / #ns_divisor_lit;
                                #final_conversion
                            }
                            Err(e) => {
                                let error = feuilletage::Error::InvalidValue {
                                    path: tracker.current_path(),
                                    message: format!("Failed to parse duration for '{}': {}", #field_name_str, e),
                                };
                                tracker.record(error.clone());
                                tracker.pop();
                                return Err(error);
                            }
                        }
                    }
                    feuilletage::ContextValue::Int(i, _) => {
                        // Integer value - assume it's already in the target unit
                        #int_conversion
                    }
                    feuilletage::ContextValue::Float(f, _) => {
                        // Float value - assume it's already in the target unit
                        #float_conversion
                    }
                    _ => {
                        let error = feuilletage::Error::TypeMismatch {
                            path: tracker.current_path(),
                            expected: format!("string, integer, or float for duration field '{}'", #field_name_str),
                            actual: field_value.type_name().to_string(),
                        };
                        tracker.record(error.clone());
                        tracker.pop();
                        return Err(error);
                    }
                };
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
    }
}

/// Generate field lookup code that checks primary name first, then aliases in order
fn generate_field_lookup(field_name_str: &str, aliases: &[String]) -> proc_macro2::TokenStream {
    if aliases.is_empty() {
        quote! { obj.get(#field_name_str) }
    } else {
        let alias_lookups: Vec<proc_macro2::TokenStream> = aliases
            .iter()
            .map(|alias| {
                quote! { .or_else(|| obj.get(#alias)) }
            })
            .collect();
        quote! { obj.get(#field_name_str)#(#alias_lookups)* }
    }
}

fn generate_field_deserialization(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Determine the lookup key: use rename if specified, otherwise use field name
    let lookup_key = attrs.rename.as_deref().unwrap_or(field_name_str);

    // Handle transform shortcuts:
    // - relative_path flag -> transform = "relative_path"
    // - normalize_path flag -> transform = "normalize_path"
    // Note: duration is now handled separately with inline code generation
    let transform = if attrs.relative_path && attrs.transform.is_none() {
        Some("relative_path".to_string())
    } else if attrs.normalize_path && attrs.transform.is_none() {
        Some("normalize_path".to_string())
    } else {
        attrs.transform.clone()
    };

    // Determine default value:
    // 1. Use explicit default if provided (with auto-conversion for String types)
    // 2. Use default_fn if provided (call function to get default)
    // 3. For Option<T> types, automatically default to None
    // 4. Otherwise, field is required (no default)
    let effective_default = if let Some(ref dv) = attrs.default_value {
        match dv {
            DefaultValue::Explicit(value) => {
                // Auto-convert raw strings for String fields
                Some(DefaultValue::Explicit(convert_string_default(
                    value, field_type,
                )))
            }
            DefaultValue::UseDefault => Some(DefaultValue::UseDefault),
            DefaultValue::Function(fn_name) => Some(DefaultValue::Function(fn_name.clone())),
        }
    } else if is_option_type(field_type) {
        Some(DefaultValue::Explicit("None".to_string()))
    } else {
        None
    };

    // Create effective attrs with updated transform and default
    let effective_attrs = FieldConfigAttributes {
        transform,
        default_value: effective_default,
        ..attrs.clone()
    };

    // Handle Vec types specially (allow_single in flag form or allow_map)
    // For Vec fields: allow_single without a field name (empty string) wraps scalar in vec
    // For Vec fields: allow_single with field name used with allow_map wraps scalar in object
    // For Vec fields: allow_map flag uses inner type's AllowMapKeys trait
    let is_vec_allow_single = attrs.allow_single.as_ref().is_some_and(|s| s.is_empty());
    if is_vec_allow_single || attrs.allow_map.is_some() || attrs.allow_map_flag {
        generate_vec_deserialization(field_name, lookup_key, field_type, &effective_attrs)
    } else {
        // Unified generation that handles all combinations
        // This includes struct fields with allow_single = "field"
        generate_unified_deserialization(field_name, lookup_key, field_type, &effective_attrs)
    }
}

/// Unified deserialization generation that handles all attribute combinations
fn generate_unified_deserialization(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Generate default value tokens if present
    // For DefaultValue::UseDefault on non-primitive types, we need to create a synthetic
    // empty object and deserialize it through FromContextValue, so that field-level defaults
    // in nested structs are properly applied.
    let default_tokens = attrs.default_value.as_ref().map(|dv| match dv {
        DefaultValue::Explicit(expr) => expr.parse::<proc_macro2::TokenStream>().unwrap(),
        DefaultValue::UseDefault => {
            // For struct types that derive Config, we need to deserialize an empty object
            // to trigger their field-level defaults. For primitives, Default::default() is fine.
            // We try deserialization first; if it fails, fall back to Default::default().
            quote! {
                {
                    // Create a synthetic empty object to trigger field-level defaults in nested structs
                    let __empty_obj: feuilletage::ContextValue<__FeuilletageS, __FeuilletageL> = feuilletage::ContextValue::object(
                        __FeuilletageIndexMap::default(),
                        feuilletage::Context::new(
                            __FeuilletageS::programmatic(),
                            __FeuilletageL::default(),
                        )
                    );
                    // This is only a probe before the Rust Default fallback, so its
                    // diagnostics must not affect the actual configuration result.
                    let mut __probe_tracker = tracker.child();
                    match feuilletage::FromContextValue::from_context_value(&__empty_obj, &mut __probe_tracker) {
                        Ok(v) => v,
                        Err(_) => Default::default(),
                    }
                }
            }
        },
        DefaultValue::Function(fn_name) => {
            let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
            quote! { #fn_ident() }
        }
    });

    // Get on_error mode (default to Default which uses field default on error)
    let on_error = attrs.on_error.unwrap_or_default();

    // Generate mutable_by check if present
    let mutable_by_check = if let Some(ref levels) = attrs.mutable_by {
        if !levels.is_empty() {
            Some(quote! {
                // Check mutable_by constraint using level names
                let allowed_level_names: &[&str] = &[#(#levels),*];
                let current_level = &field_value.context().level;
                let current_level_name = current_level.name();

                let is_allowed = allowed_level_names.iter().any(|&allowed| {
                    current_level_name == allowed
                });

                if !is_allowed {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!(
                            "Field '{}' can only be set by levels: {:?}, but was set by: {}",
                            #field_name_str,
                            allowed_level_names,
                            current_level_name
                        ),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                }
            })
        } else {
            None
        }
    } else {
        None
    };

    // Generate deprecation warning if present
    let deprecation_warning = attrs.deprecated.as_ref().map(|msg| {
        quote! {
            eprintln!("Warning: Field '{}' is deprecated: {}", #field_name_str, #msg);
        }
    });

    // Generate allow_single wrapping code for struct fields
    // When allow_single = "field_name" is set, wrap scalar input as {field_name: value}
    let allow_single_wrapping = if let Some(ref scalar_field) = attrs.allow_single {
        // Only generate wrapping for non-empty field names (struct field case)
        // Empty string means Vec wrapping which is handled in generate_vec_deserialization
        if !scalar_field.is_empty() {
            Some(quote! {
                // allow_single wrapping: if input is scalar, wrap as object
                let field_value: __FeuilletageCow<'_, feuilletage::ContextValue<__FeuilletageS, __FeuilletageL>> = match field_value {
                    feuilletage::ContextValue::String(_, _) | feuilletage::ContextValue::Int(_, _) | feuilletage::ContextValue::Float(_, _) | feuilletage::ContextValue::Bool(_, _) => {
                        // Scalar input - wrap as {scalar_field: value}
                        let mut wrapped_obj = __FeuilletageIndexMap::default();
                        wrapped_obj.insert(#scalar_field.to_string(), field_value.clone());
                        __FeuilletageCow::Owned(feuilletage::ContextValue::object(wrapped_obj, field_value.context().clone()))
                    }
                    _ => __FeuilletageCow::Borrowed(field_value),
                };
                let field_value = field_value.as_ref();
            })
        } else {
            None
        }
    } else {
        None
    };

    // Generate validation code
    // Pass the default expression so validation failures can use the default instead of failing
    let validation_code =
        generate_validation_code(field_name, field_name_str, attrs, default_tokens.as_ref());

    // Generate transform_after code (post-deserialization transform on the Rust value)
    let transform_after_code = if let Some(ref transform_fn) = attrs.transform_after {
        let transform_fn_ident: proc_macro2::TokenStream = transform_fn.parse().unwrap();
        let error_handling = match on_error {
            OnErrorMode::Fail => quote! {
                tracker.record(e.clone());
                tracker.pop();
                return Err(e);
            },
            OnErrorMode::Default | OnErrorMode::Skip => {
                if let Some(ref default_expr) = default_tokens {
                    quote! {
                        tracker.record(e);
                        __temp_value = #default_expr;
                    }
                } else {
                    quote! {
                        tracker.record(e.clone());
                        tracker.pop();
                        return Err(e);
                    }
                }
            }
        };
        quote! {
            if let Err(e) = #transform_fn_ident(&mut __temp_value) {
                #error_handling
            }
        }
    } else {
        quote! {}
    };

    // Check if coercion is enabled and supported for this type
    let use_coercion = attrs.coerce && get_coercion_info(field_type).is_some();
    let coercion_code = if use_coercion {
        generate_coercion_code(field_type, field_name_str)
    } else {
        None
    };

    // Handle duration attribute specially with inline code generation
    if let Some(ref duration_config) = attrs.duration {
        let duration_code = generate_duration_deserialization(
            field_type,
            field_name_str,
            duration_config,
            &default_tokens,
            &validation_code,
        );

        // Generate missing field handling
        let missing_field_handling = if let Some(ref default_expr) = &default_tokens {
            quote! { #default_expr }
        } else {
            quote! {
                let error = feuilletage::Error::InvalidValue {
                    path: format!("{}.{}", tracker.current_path(), #field_name_str),
                    message: format!("required field '{}' was not provided", #field_name_str),
                };
                tracker.record(error.clone());
                return Err(error);
            }
        };

        // Generate field lookup with aliases support
        let field_lookup = generate_field_lookup(field_name_str, &attrs.aliases);

        // Generate env variable check if specified
        if let Some(ref var_name) = attrs.env {
            return quote! {
                let #field_name = if let Some(field_value) = #field_lookup {
                    if field_value.is_null() {
                        if let Ok(env_value) = std::env::var(#var_name) {
                            let env_config_value: feuilletage::ContextValue<__FeuilletageS, __FeuilletageL> = feuilletage::ContextValue::string(
                                env_value,
                                feuilletage::Context::new(
                                    __FeuilletageS::environment(),
                                    __FeuilletageL::default(),
                                )
                            );
                            let field_value = &env_config_value;
                            tracker.push_field(#field_name_str);
                            #duration_code
                        } else {
                            #missing_field_handling
                        }
                    } else {
                        tracker.push_field(#field_name_str);
                        #mutable_by_check
                        #deprecation_warning
                        #allow_single_wrapping
                        #duration_code
                    }
                } else if let Ok(env_value) = std::env::var(#var_name) {
                    let env_config_value: feuilletage::ContextValue<__FeuilletageS, __FeuilletageL> = feuilletage::ContextValue::string(
                        env_value,
                        feuilletage::Context::new(
                            __FeuilletageS::environment(),
                            __FeuilletageL::default(),
                        )
                    );
                    let field_value = &env_config_value;
                    tracker.push_field(#field_name_str);
                    #duration_code
                } else {
                    #missing_field_handling
                };
            };
        }

        return quote! {
            let #field_name = if let Some(field_value) = #field_lookup {
                tracker.push_field(#field_name_str);
                #mutable_by_check
                #deprecation_warning
                #allow_single_wrapping
                #duration_code
            } else {
                #missing_field_handling
            };
        };
    }

    // Generate deserialization logic combining transform, coercion, and default
    let deserialization = match (&attrs.transform, &default_tokens, &coercion_code) {
        // No transform, no default, with coercion - use coercion function
        (None, None, Some(coerce_expr)) => quote! {
            let mut __temp_value: #field_type = #coerce_expr;
            #transform_after_code
            #validation_code
            tracker.pop();
            __temp_value
        },
        // No transform, no default, no coercion - standard deserialization
        (None, None, None) => quote! {
            let mut __temp_value: #field_type = {
                let result = feuilletage::FromContextValue::from_context_value(field_value, tracker);
                result?
            };
            #transform_after_code
            #validation_code
            tracker.pop();
            __temp_value
        },
        // No transform, with default, with coercion - try coercion with default fallback
        (None, Some(default_expr), Some(_coerce_expr)) => {
            let (coerce_fn, cast) = get_coercion_info(field_type).unwrap();
            let coerce_fn_ident: proc_macro2::TokenStream =
                format!("feuilletage::coerce::{}", coerce_fn)
                    .parse()
                    .unwrap();
            let error_handling = match on_error {
                OnErrorMode::Fail => quote! {
                    let error = feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: field_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                },
                OnErrorMode::Default | OnErrorMode::Skip => quote! {
                    tracker.record(feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: field_value.type_name().to_string(),
                    });
                    #default_expr
                },
            };
            let coerce_with_default = if let Some(cast_type) = cast {
                let cast_ident: proc_macro2::TokenStream = cast_type.parse().unwrap();
                quote! {
                    match #coerce_fn_ident(field_value) {
                        Some(v) => v as #cast_ident,
                        None => {
                            #error_handling
                        }
                    }
                }
            } else {
                quote! {
                    match #coerce_fn_ident(field_value) {
                        Some(v) => v,
                        None => {
                            #error_handling
                        }
                    }
                }
            };
            quote! {
                let mut __temp_value: #field_type = #coerce_with_default;
                #transform_after_code
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
        // No transform, with default, no coercion - deserialize with fallback
        (None, Some(default_expr), None) => {
            let error_handling = match on_error {
                OnErrorMode::Fail => quote! {
                    tracker.record(e.clone());
                    tracker.pop();
                    return Err(e);
                },
                OnErrorMode::Default | OnErrorMode::Skip => quote! {
                    tracker.record(e);
                    #default_expr
                },
            };
            quote! {
                let mut __temp_value: #field_type = match feuilletage::FromContextValue::from_context_value(field_value, tracker) {
                    Ok(value) => value,
                    Err(e) => {
                        #error_handling
                    }
                };
                #transform_after_code
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
        // With transform, no default - transform then deserialize/coerce, fail on any error
        (Some(t), None, coerce_opt) => {
            let transform_ident: proc_macro2::TokenStream = parse_transform_path(t);
            let deser_code = if coerce_opt.is_some() {
                // With coercion after transform
                let (coerce_fn, cast) = get_coercion_info(field_type).unwrap();
                let coerce_fn_ident: proc_macro2::TokenStream =
                    format!("feuilletage::coerce::{}", coerce_fn)
                        .parse()
                        .unwrap();
                if let Some(cast_type) = cast {
                    let cast_ident: proc_macro2::TokenStream = cast_type.parse().unwrap();
                    quote! {
                        match #coerce_fn_ident(transformed_value) {
                            Some(v) => v as #cast_ident,
                            None => {
                                let error = feuilletage::Error::TypeMismatch {
                                    path: tracker.current_path(),
                                    expected: #field_name_str.to_string(),
                                    actual: transformed_value.type_name().to_string(),
                                };
                                tracker.record(error.clone());
                                tracker.pop();
                                return Err(error);
                            }
                        }
                    }
                } else {
                    quote! {
                        match #coerce_fn_ident(transformed_value) {
                            Some(v) => v,
                            None => {
                                let error = feuilletage::Error::TypeMismatch {
                                    path: tracker.current_path(),
                                    expected: #field_name_str.to_string(),
                                    actual: transformed_value.type_name().to_string(),
                                };
                                tracker.record(error.clone());
                                tracker.pop();
                                return Err(error);
                            }
                        }
                    }
                }
            } else {
                // Standard deserialization after transform
                quote! {
                    {
                        let result = feuilletage::FromContextValue::from_context_value(&transformed_value, tracker);
                        result?
                    }
                }
            };
            quote! {
                let mut transformed_value = field_value.clone();
                let transform_context = transformed_value.context().clone();
                if let Err(e) = #transform_ident(&mut transformed_value, &transform_context) {
                    tracker.record(e);
                    tracker.pop();
                    return Err(feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Transform failed for field '{}'", #field_name_str),
                    });
                }
                let mut __temp_value: #field_type = #deser_code;
                #transform_after_code
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
        // With transform and default - try both, fallback to default on any error
        (Some(t), Some(default_expr), coerce_opt) => {
            let transform_ident: proc_macro2::TokenStream = parse_transform_path(t);

            // Generate error handling based on on_error mode for coercion failure
            let coerce_error_handling = match on_error {
                OnErrorMode::Fail => quote! {
                    let error = feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: transformed_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                },
                OnErrorMode::Default | OnErrorMode::Skip => quote! {
                    tracker.record(feuilletage::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: #field_name_str.to_string(),
                        actual: transformed_value.type_name().to_string(),
                    });
                    #default_expr
                },
            };

            // Generate error handling for deserialization failure
            let deser_error_handling = match on_error {
                OnErrorMode::Fail => quote! {
                    tracker.record(e.clone());
                    tracker.pop();
                    return Err(e);
                },
                OnErrorMode::Default | OnErrorMode::Skip => quote! {
                    tracker.record(e);
                    #default_expr
                },
            };

            // Generate error handling for transform failure
            let transform_error_handling = match on_error {
                OnErrorMode::Fail => quote! {
                    tracker.record(e.clone());
                    tracker.pop();
                    return Err(feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Transform failed for field '{}'", #field_name_str),
                    });
                },
                OnErrorMode::Default | OnErrorMode::Skip => quote! {
                    tracker.record(e);
                    #default_expr
                },
            };

            let deser_code = if coerce_opt.is_some() {
                // With coercion after transform
                let (coerce_fn, cast) = get_coercion_info(field_type).unwrap();
                let coerce_fn_ident: proc_macro2::TokenStream =
                    format!("feuilletage::coerce::{}", coerce_fn)
                        .parse()
                        .unwrap();
                if let Some(cast_type) = cast {
                    let cast_ident: proc_macro2::TokenStream = cast_type.parse().unwrap();
                    quote! {
                        match #coerce_fn_ident(transformed_value) {
                            Some(v) => v as #cast_ident,
                            None => {
                                #coerce_error_handling
                            }
                        }
                    }
                } else {
                    quote! {
                        match #coerce_fn_ident(transformed_value) {
                            Some(v) => v,
                            None => {
                                #coerce_error_handling
                            }
                        }
                    }
                }
            } else {
                // Standard deserialization after transform
                quote! {
                    match feuilletage::FromContextValue::from_context_value(&transformed_value, tracker) {
                        Ok(value) => value,
                        Err(e) => {
                            #deser_error_handling
                        }
                    }
                }
            };
            quote! {
                let mut transformed_value = field_value.clone();
                let transform_context = transformed_value.context().clone();
                let mut __temp_value: #field_type = match #transform_ident(&mut transformed_value, &transform_context) {
                    Ok(()) => {
                        // Transform succeeded, try deserialization/coercion
                        #deser_code
                    }
                    Err(e) => {
                        // Transform failed
                        #transform_error_handling
                    }
                };
                #transform_after_code
                #validation_code
                tracker.pop();
                __temp_value
            }
        }
    };

    // Wrap deserialization with allow_list handling for HashMap/BTreeMap fields.
    // When allow_list is set, check if input is an Array first and process it specially,
    // otherwise fall through to normal map deserialization.
    let deserialization = if attrs.allow_list {
        if let Some(value_type) = extract_map_value_type(field_type) {
            let map_kind = get_map_kind(field_type).unwrap_or("HashMap");
            let map_constructor: proc_macro2::TokenStream = if map_kind == "BTreeMap" {
                quote! { std::collections::BTreeMap::new() }
            } else {
                quote! { std::collections::HashMap::new() }
            };
            let original_deser = deserialization;
            quote! {
                // allow_list: check if input is an array
                match field_value {
                    feuilletage::ContextValue::Array(arr, _) => {
                        let mut __allow_list_map = #map_constructor;
                        for (__idx, __item) in arr.iter().enumerate() {
                            tracker.push_index(__idx);
                            match __item {
                                feuilletage::ContextValue::String(s, _) => {
                                    __allow_list_map.insert(s.clone(), Default::default());
                                }
                                feuilletage::ContextValue::Object(obj_map, _) => {
                                    for (__key, __val) in obj_map {
                                        tracker.push_field(__key);
                                        match <#value_type as feuilletage::FromContextValue<__FeuilletageS, __FeuilletageL>>::from_context_value(__val, tracker) {
                                            Ok(parsed) => { __allow_list_map.insert(__key.clone(), parsed); }
                                            Err(e) => tracker.record(e),
                                        }
                                        tracker.pop();
                                    }
                                }
                                _ => {
                                    let __err = feuilletage::Error::TypeMismatch {
                                        path: tracker.current_path(),
                                        expected: "string or object".to_string(),
                                        actual: __item.type_name().to_string(),
                                    };
                                    tracker.record(__err);
                                }
                            }
                            tracker.pop();
                        }
                        let mut __temp_value: #field_type = __allow_list_map;
                        #transform_after_code
                        #validation_code
                        tracker.pop();
                        __temp_value
                    }
                    _ => {
                        // Not an array - fall through to normal map deserialization
                        #original_deser
                    }
                }
            }
        } else {
            // Not a HashMap/BTreeMap type - ignore allow_list
            deserialization
        }
    } else {
        deserialization
    };

    // Generate missing field handling
    // Fields are required by default unless they have a default value or are Option<T>
    let missing_field_handling = if let Some(ref default_expr) = &default_tokens {
        quote! { #default_expr }
    } else {
        // No default - field is required
        quote! {
            let error = feuilletage::Error::InvalidValue {
                path: format!("{}.{}", tracker.current_path(), #field_name_str),
                message: format!("required field '{}' was not provided", #field_name_str),
            };
            tracker.record(error.clone());
            return Err(error);
        }
    };

    // Generate field lookup with aliases support
    let field_lookup = generate_field_lookup(field_name_str, &attrs.aliases);

    // Generate env variable check if specified
    // Config values take precedence over environment variables.
    // Order: config field -> env var -> default
    if let Some(ref var_name) = attrs.env {
        quote! {
            let #field_name = if let Some(field_value) = #field_lookup {
                // Config field exists - check if it's null
                if field_value.is_null() {
                    // Field is null - try env, then default
                    if let Ok(env_value) = std::env::var(#var_name) {
                        let env_config_value: feuilletage::ContextValue<__FeuilletageS, __FeuilletageL> = feuilletage::ContextValue::string(
                            env_value,
                            feuilletage::Context::new(
                                __FeuilletageS::environment(),
                                __FeuilletageL::default(),
                            )
                        );
                        tracker.push_field(#field_name_str);
                        #deprecation_warning
                        let mut __temp_value: #field_type = {
                            let result = feuilletage::FromContextValue::from_context_value(&env_config_value, tracker);
                            result?
                        };
                        #transform_after_code
                        #validation_code
                        tracker.pop();
                        __temp_value
                    } else {
                        #missing_field_handling
                    }
                } else {
                    // Config field has a value - use it
                    tracker.push_field(#field_name_str);
                    #mutable_by_check
                    #deprecation_warning
                    #allow_single_wrapping
                    #deserialization
                }
            } else if let Ok(env_value) = std::env::var(#var_name) {
                // No config field - try environment variable
                let env_config_value: feuilletage::ContextValue<__FeuilletageS, __FeuilletageL> = feuilletage::ContextValue::string(
                    env_value,
                    feuilletage::Context::new(
                        __FeuilletageS::environment(),
                        __FeuilletageL::default(),
                    )
                );
                tracker.push_field(#field_name_str);
                #deprecation_warning
                let mut __temp_value: #field_type = {
                    let result = feuilletage::FromContextValue::from_context_value(&env_config_value, tracker);
                    result?
                };
                #transform_after_code
                #validation_code
                tracker.pop();
                __temp_value
            } else {
                // No config field and no env var - use default or fail
                #missing_field_handling
            };
        }
    } else {
        // No env loading - standard flow with alias support
        quote! {
            let #field_name = if let Some(field_value) = #field_lookup {
                // Check if it's explicitly null
                if field_value.is_null() {
                    #missing_field_handling
                } else {
                    tracker.push_field(#field_name_str);
                    #mutable_by_check
                    #deprecation_warning
                    #allow_single_wrapping
                    #deserialization
                }
            } else {
                #missing_field_handling
            };
        }
    }
}

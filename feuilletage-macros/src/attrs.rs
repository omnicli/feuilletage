//! Attribute type definitions and parsing for the `#[feuilletage(...)]` namespace.
//!
//! This module is the *parsing layer* only — it consumes `syn::Attribute`s
//! and produces typed `FieldConfigAttributes` / `ContainerAttributes` /
//! `VariantAttributes` records that the codegen functions in `lib.rs`
//! then consume.
//!
//! No codegen lives here; if you find yourself reaching for `quote!`,
//! the function probably belongs in `lib.rs`.

use quote::ToTokens;
use syn::spanned::Spanned;

use crate::helpers::{to_camel_case, to_kebab_case, to_screaming_snake_case, to_snake_case};

/// Parse a default_fn value like "fn_name" or "fn_name(field1, field2)"
/// Returns (function_name, field_params)
pub(crate) fn parse_default_fn_value(s: &str) -> (String, Vec<String>) {
    if let Some(paren_start) = s.find('(') {
        if let Some(paren_end) = s.rfind(')') {
            let fn_name = s[..paren_start].trim().to_string();
            let params_str = &s[paren_start + 1..paren_end];
            let params: Vec<String> = params_str
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            return (fn_name, params);
        }
    }
    (s.to_string(), vec![])
}

/// Configuration for struct-level allow_map attribute.
/// When a struct has this, it can accept single-key maps where the key becomes a field value.
#[derive(Clone, Default)]
pub(crate) struct StructAllowMapConfig {
    /// The field name where the map key should be injected
    pub(crate) key_field: Option<String>,
    /// The field name where scalar values should be placed (when map value is a scalar)
    pub(crate) scalar_as_field: Option<String>,
}

/// Container-level attributes for enums and structs
#[derive(Default)]
pub(crate) struct ContainerAttributes {
    /// Parse this type through an intermediate wire type, then project it with
    /// `feuilletage::FromParsed`.
    pub(crate) parse_as: Option<syn::Type>,
    /// Default mutable_by constraint for fields without a field-level override
    pub(crate) mutable_by: Option<Vec<String>>,
    /// Tag field name for internally tagged enums
    pub(crate) tag: Option<String>,
    /// Whether this is an untagged enum
    pub(crate) untagged: bool,
    /// Whether this enum uses external tagging (map key determines variant)
    pub(crate) external_tag: bool,
    /// Whether this enum uses value matching (variant determined by literal value)
    /// When true, variants use #[feuilletage(variant = ...)] to specify match conditions
    pub(crate) value_matched: bool,
    /// For structs: if input is a scalar, wrap it as {scalar_as_field: value}
    pub(crate) scalar_as: Option<String>,
    /// For structs: if input is an array, wrap it as {array_as_field: value}
    pub(crate) array_as: Option<String>,
    /// For enums: rename all variants using the specified case convention
    pub(crate) rename_all: Option<RenameAllCase>,
    /// For structs: allow_map configuration for accepting single-key maps
    /// where the key becomes a field value (external tag pattern for structs)
    pub(crate) struct_allow_map: Option<StructAllowMapConfig>,
    /// Skip generating Serialize impl (only generate FromContextValue)
    pub(crate) skip_serialize: bool,
    /// Transparent wrapper - serialize/deserialize as the single inner field
    pub(crate) transparent: bool,
    /// Post-process function to call after deserialization
    /// Signature: fn<S, L>(&mut T, &ContextValue<S, L>, &mut ErrorTracker) -> Result<(), Error>
    pub(crate) post_process: Option<String>,
    /// Skip generating serde::Deserialize impl (only generate FromContextValue and Serialize)
    /// By default, feuilletage generates Deserialize for supported types (external_tag enums).
    pub(crate) skip_deserialize: bool,
    /// Container-level transform applied to the raw input ContextValue before
    /// any field deserialization or scalar_as/array_as wrapping. Lets a struct
    /// normalize its input shape declaratively.
    /// Signature: fn<S, L>(&mut ContextValue<S, L>, &Context<S, L>) -> Result<(), Error>
    /// Bare names resolve to `feuilletage::transform::*`; qualified paths are used verbatim.
    pub(crate) transform: Option<String>,
}

/// Supported case conventions for rename_all
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RenameAllCase {
    /// lowercase
    Lowercase,
    /// UPPERCASE
    Uppercase,
    /// snake_case
    SnakeCase,
    /// camelCase
    CamelCase,
    /// PascalCase
    PascalCase,
    /// kebab-case
    KebabCase,
    /// SCREAMING_SNAKE_CASE
    ScreamingSnakeCase,
}

impl RenameAllCase {
    pub(crate) fn from_str(s: &str) -> Option<Self> {
        match s {
            "lowercase" => Some(RenameAllCase::Lowercase),
            "UPPERCASE" => Some(RenameAllCase::Uppercase),
            "snake_case" => Some(RenameAllCase::SnakeCase),
            "camelCase" => Some(RenameAllCase::CamelCase),
            "PascalCase" => Some(RenameAllCase::PascalCase),
            "kebab-case" => Some(RenameAllCase::KebabCase),
            "SCREAMING_SNAKE_CASE" => Some(RenameAllCase::ScreamingSnakeCase),
            _ => None,
        }
    }

    /// Convert a PascalCase identifier to the target case
    pub(crate) fn convert(&self, s: &str) -> String {
        match self {
            RenameAllCase::Lowercase => s.to_lowercase(),
            RenameAllCase::Uppercase => s.to_uppercase(),
            RenameAllCase::SnakeCase => to_snake_case(s),
            RenameAllCase::CamelCase => to_camel_case(s),
            RenameAllCase::PascalCase => s.to_string(), // Already PascalCase
            RenameAllCase::KebabCase => to_kebab_case(s),
            RenameAllCase::ScreamingSnakeCase => to_screaming_snake_case(s),
        }
    }
}

/// Parse container-level attributes like #[feuilletage(tag = "type")] or #[feuilletage(untagged)]
/// Also parses struct-level attributes: scalar_as, array_as
fn try_parse_container_attributes(attrs: &[syn::Attribute]) -> syn::Result<ContainerAttributes> {
    let mut container_attrs = ContainerAttributes::default();

    for attr in attrs {
        if !attr.path().is_ident("feuilletage") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("parse_as") {
                let value = meta.value()?;
                let value: syn::LitStr = value.parse()?;
                container_attrs.parse_as = Some(value.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("mutable_by") {
                let value = meta.value()?;
                let content;
                syn::bracketed!(content in value);
                let levels: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> =
                    content.parse_terminated(|input| input.parse::<syn::LitStr>(), syn::Token![,])?;
                container_attrs.mutable_by = Some(
                    levels
                        .into_iter()
                        .map(|lit| lit.value())
                        .collect(),
                );
                return Ok(());
            }
            if meta.path.is_ident("tag") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                container_attrs.tag = Some(s.value());
                return Ok(());
            }
            if meta.path.is_ident("untagged") {
                container_attrs.untagged = true;
                return Ok(());
            }
            if meta.path.is_ident("external_tag") {
                container_attrs.external_tag = true;
                return Ok(());
            }
            if meta.path.is_ident("value_matched") {
                container_attrs.value_matched = true;
                return Ok(());
            }
            if meta.path.is_ident("scalar_as") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                container_attrs.scalar_as = Some(s.value());
                return Ok(());
            }
            if meta.path.is_ident("array_as") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                container_attrs.array_as = Some(s.value());
                return Ok(());
            }
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                let case_str = s.value();
                container_attrs.rename_all = Some(
                    RenameAllCase::from_str(&case_str)
                        .unwrap_or_else(|| panic!(
                            "Invalid rename_all value '{}'. Valid values are: lowercase, UPPERCASE, snake_case, camelCase, PascalCase, kebab-case, SCREAMING_SNAKE_CASE",
                            case_str
                        ))
                );
                return Ok(());
            }
            // Parse struct-level allow_map(key = field, scalar_as = field)
            if meta.path.is_ident("allow_map") {
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);

                    let mut config = StructAllowMapConfig::default();

                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        let _: syn::Token![=] = content.parse()?;

                        if ident == "key" {
                            // Accept either string literal or ident
                            if content.peek(syn::LitStr) {
                                let lit: syn::LitStr = content.parse()?;
                                config.key_field = Some(lit.value());
                            } else {
                                let field_ident: syn::Ident = content.parse()?;
                                config.key_field = Some(field_ident.to_string());
                            }
                        } else if ident == "scalar_as" {
                            // Accept either string literal or ident
                            if content.peek(syn::LitStr) {
                                let lit: syn::LitStr = content.parse()?;
                                config.scalar_as_field = Some(lit.value());
                            } else {
                                let field_ident: syn::Ident = content.parse()?;
                                config.scalar_as_field = Some(field_ident.to_string());
                            }
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                "unknown feuilletage allow_map option",
                            ));
                        }

                        // Skip comma if present
                        if content.peek(syn::Token![,]) {
                            let _: syn::Token![,] = content.parse()?;
                        }
                    }

                    container_attrs.struct_allow_map = Some(config);
                }
                return Ok(());
            }
            // Parse skip_serialize - only generate FromContextValue, not Serialize
            if meta.path.is_ident("skip_serialize") {
                container_attrs.skip_serialize = true;
                return Ok(());
            }
            // Parse transparent - serialize/deserialize as the single inner field
            if meta.path.is_ident("transparent") {
                container_attrs.transparent = true;
                return Ok(());
            }
            // Parse skip_deserialize - skip generating serde::Deserialize impl
            if meta.path.is_ident("skip_deserialize") {
                container_attrs.skip_deserialize = true;
                return Ok(());
            }
            // Parse post_process = "function_name" - call after deserialization
            if meta.path.is_ident("post_process") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                container_attrs.post_process = Some(s.value());
                return Ok(());
            }
            // Parse transform = "function_name" - container-level shape normalizer
            // applied to the raw input ContextValue before field deserialization.
            if meta.path.is_ident("transform") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                container_attrs.transform = Some(s.value());
                return Ok(());
            }
            Err(meta.error("unknown feuilletage container attribute"))
        })?;
    }

    Ok(container_attrs)
}

pub(crate) fn parse_container_attributes(attrs: &[syn::Attribute]) -> ContainerAttributes {
    try_parse_container_attributes(attrs)
        .expect("feuilletage attributes were validated before code generation")
}

/// Represents a literal value or predicate that a variant can match against.
/// Used with `#[feuilletage(variant = ...)]` for value-matched enums.
#[derive(Clone, Debug)]
pub(crate) enum VariantMatch {
    /// Match a boolean literal (true or false)
    Bool(bool),
    /// Match a string literal
    String(String),
    /// Match an integer literal (can be positive or negative)
    Int(i64),
    /// Match a float literal (can be positive or negative)
    Float(f64),
    /// Match truthy values: true, "true", "yes", "y", 1, non-zero integers
    Truthy,
    /// Match falsy values: false, "false", "no", "n", 0
    Falsy,
    /// Match null input (for external_tag enums, replaces null_variant)
    Null,
    /// Match any string value (for external_tag scalar handling)
    AnyString,
    /// Match any integer value (for external_tag scalar handling)
    AnyInt,
    /// Match any float value (for external_tag scalar handling)
    AnyFloat,
    /// Match any boolean value (for external_tag scalar handling)
    AnyBool,
    /// Match any scalar (string, int, float, bool) - replaces scalar_variant
    AnyScalar,
    /// Match strings starting with a prefix: starts_with("prefix")
    StartsWith(String),
    /// Match strings ending with a suffix: ends_with("suffix")
    EndsWith(String),
    /// Match strings containing a substring: contains("substring")
    Contains(String),
    /// Match numbers in a range (inclusive): range(min, max)
    /// Stores min and max as strings to support both int and float
    Range(String, String),
    /// Match strings against a regex pattern: regex(r"pattern")
    #[cfg(feature = "regex")]
    Regex(String),
    /// Custom predicate function: predicate("fn_name")
    /// Function signature: fn(&ContextValue) -> bool
    Predicate(String),
    /// Custom extractor function: parse("fn_name")
    /// Function signature: fn(&ContextValue) -> Option<T>
    /// Returns None = no match, Some(val) = match with that value
    Parse(String),
}

impl VariantMatch {
    /// Check if this is a built-in parameterized predicate
    pub(crate) fn is_builtin_predicate(&self) -> bool {
        matches!(
            self,
            VariantMatch::StartsWith(_)
                | VariantMatch::EndsWith(_)
                | VariantMatch::Contains(_)
                | VariantMatch::Range(_, _)
        ) || {
            #[cfg(feature = "regex")]
            {
                matches!(self, VariantMatch::Regex(_))
            }
            #[cfg(not(feature = "regex"))]
            {
                false
            }
        }
    }
    /// Check if this is a custom predicate
    pub(crate) fn is_custom_predicate(&self) -> bool {
        matches!(self, VariantMatch::Predicate(_))
    }
    /// Check if this is a custom extractor (parse)
    pub(crate) fn is_parse(&self) -> bool {
        matches!(self, VariantMatch::Parse(_))
    }
}

/// Variant-level attributes
#[derive(Default, Clone)]
pub(crate) struct VariantAttributes {
    /// Renamed tag value for this variant
    pub(crate) rename: Option<String>,
    /// Alternative names for this variant
    pub(crate) aliases: Vec<String>,
    /// Whether this variant is the fallback for unknown keys (external_tag only)
    pub(crate) fallback: bool,
    /// Whether this variant is selected when input is null (external_tag only)
    pub(crate) null_variant: bool,
    /// Whether this variant is selected when input is a scalar (external_tag only)
    pub(crate) scalar_variant: bool,
    /// Literal values and predicates this variant matches (for value-matched enums)
    /// Populated by `#[feuilletage(variant = true | "string" | 1)]` syntax
    pub(crate) variant_matches: Vec<VariantMatch>,
    /// Custom matching function name (for value-matched enums)
    /// Populated by `#[feuilletage(variant_fn = "predicate")]` syntax
    pub(crate) variant_fn: Option<String>,
    /// Value to use when scalar-matched, instead of deserializing
    /// Populated by `#[feuilletage(variant_value = <literal>)]` syntax
    pub(crate) variant_value: Option<syn::Lit>,
    /// Use Default::default() when scalar-matched, instead of deserializing
    /// Populated by `#[feuilletage(variant_default)]` flag
    pub(crate) variant_default: bool,
    /// Field name on the inner type to inject the external tag into (fallback variants only)
    /// Populated by `#[feuilletage(from_tag = "field_name")]` syntax
    pub(crate) from_tag: Option<String>,
    /// Whether this variant supports allow_single (wraps scalar into single-element array)
    /// Only valid for tuple variants with Vec<T> inner type in untagged enums
    pub(crate) allow_single: bool,
}

/// Parse a single variant match value from tokens
/// Supports:
/// - Literals: true, false, "string", 123, -456, 1.5, -3.14
/// - Simple predicates: truthy, falsy, null, any_string, any_int, any_float, any_bool, any_scalar
/// - Parameterized predicates: starts_with("prefix"), ends_with("suffix"), contains("substring"),
///   range(min, max), regex(r"pattern"), predicate("fn_name"), parse("fn_name")
pub(crate) fn parse_variant_match_value(
    input: syn::parse::ParseStream,
) -> syn::Result<VariantMatch> {
    // Check for negative number first
    if input.peek(syn::Token![-]) {
        input.parse::<syn::Token![-]>()?;
        // Could be negative int or negative float
        if input.peek(syn::LitFloat) {
            let lit: syn::LitFloat = input.parse()?;
            let val: f64 = lit.base10_parse()?;
            return Ok(VariantMatch::Float(-val));
        }
        let lit: syn::LitInt = input.parse()?;
        let val: i64 = lit.base10_parse()?;
        return Ok(VariantMatch::Int(-val));
    }

    // Check for boolean literal (true/false are keywords, not identifiers)
    if input.peek(syn::LitBool) {
        let lit: syn::LitBool = input.parse()?;
        return Ok(VariantMatch::Bool(lit.value()));
    }

    // Check for identifier (could be simple predicate or function call)
    if input.peek(syn::Ident) {
        let ident: syn::Ident = input.parse()?;
        let name = ident.to_string();

        // Check if this is a function call (identifier followed by parentheses)
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);

            return match name.as_str() {
                "starts_with" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::StartsWith(arg.value()))
                }
                "ends_with" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::EndsWith(arg.value()))
                }
                "contains" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::Contains(arg.value()))
                }
                "range" => {
                    // Parse min, max - can be int or float
                    let min = parse_range_bound(&content)?;
                    content.parse::<syn::Token![,]>()?;
                    let max = parse_range_bound(&content)?;
                    Ok(VariantMatch::Range(min, max))
                }
                #[cfg(feature = "regex")]
                "regex" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::Regex(arg.value()))
                }
                #[cfg(not(feature = "regex"))]
                "regex" => {
                    Err(syn::Error::new(
                        ident.span(),
                        "regex predicate requires the 'regex' feature to be enabled",
                    ))
                }
                "predicate" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::Predicate(arg.value()))
                }
                "parse" => {
                    let arg: syn::LitStr = content.parse()?;
                    Ok(VariantMatch::Parse(arg.value()))
                }
                _ => Err(syn::Error::new(
                    ident.span(),
                    format!(
                        "unknown parameterized predicate '{}'. Use: starts_with, ends_with, contains, range, regex, predicate, or parse",
                        name
                    ),
                )),
            };
        }

        // Simple predicate (no parentheses)
        return match name.as_str() {
            "truthy" => Ok(VariantMatch::Truthy),
            "falsy" => Ok(VariantMatch::Falsy),
            "null" => Ok(VariantMatch::Null),
            "any_string" => Ok(VariantMatch::AnyString),
            "any_int" => Ok(VariantMatch::AnyInt),
            "any_float" => Ok(VariantMatch::AnyFloat),
            "any_bool" => Ok(VariantMatch::AnyBool),
            "any_scalar" => Ok(VariantMatch::AnyScalar),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown variant match predicate '{}'. Use: true, false, truthy, falsy, null, any_string, any_int, any_float, any_bool, any_scalar, starts_with(), ends_with(), contains(), range(), regex(), predicate(), parse(), or a literal",
                    name
                ),
            )),
        };
    }

    // Check for string literal
    if input.peek(syn::LitStr) {
        let lit: syn::LitStr = input.parse()?;
        return Ok(VariantMatch::String(lit.value()));
    }

    // Check for float literal (must check before int since 1.0 could be parsed as int otherwise)
    if input.peek(syn::LitFloat) {
        let lit: syn::LitFloat = input.parse()?;
        let val: f64 = lit.base10_parse()?;
        return Ok(VariantMatch::Float(val));
    }

    // Check for integer literal
    if input.peek(syn::LitInt) {
        let lit: syn::LitInt = input.parse()?;
        let val: i64 = lit.base10_parse()?;
        return Ok(VariantMatch::Int(val));
    }

    Err(input.error("expected variant match value: true, false, truthy, falsy, null, any_string, any_int, any_float, any_bool, any_scalar, starts_with(), ends_with(), contains(), range(), regex(), predicate(), parse(), \"string\", integer, or float"))
}

/// Helper to parse a range bound (can be int or float, positive or negative)
pub(crate) fn parse_range_bound(input: syn::parse::ParseStream) -> syn::Result<String> {
    let negative = if input.peek(syn::Token![-]) {
        input.parse::<syn::Token![-]>()?;
        true
    } else {
        false
    };

    if input.peek(syn::LitFloat) {
        let lit: syn::LitFloat = input.parse()?;
        let val: f64 = lit.base10_parse()?;
        let val = if negative { -val } else { val };
        return Ok(val.to_string());
    }

    if input.peek(syn::LitInt) {
        let lit: syn::LitInt = input.parse()?;
        let val: i64 = lit.base10_parse()?;
        let val = if negative { -val } else { val };
        return Ok(val.to_string());
    }

    Err(input.error("expected integer or float for range bound"))
}

/// Parse variant-level attributes like #[feuilletage(rename = "text")] or #[feuilletage(alias = "alt")]
fn try_parse_variant_attributes(attrs: &[syn::Attribute]) -> syn::Result<VariantAttributes> {
    let mut variant_attrs = VariantAttributes::default();

    for attr in attrs {
        if !attr.path().is_ident("feuilletage") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                variant_attrs.rename = Some(s.value());
                return Ok(());
            }
            if meta.path.is_ident("alias") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                variant_attrs.aliases.push(s.value());
                return Ok(());
            }
            // Handle aliases = ["alias1", "alias2"] (array form)
            if meta.path.is_ident("aliases") {
                let value = meta.value()?;
                let content;
                syn::bracketed!(content in value);
                let aliases: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> = content
                    .parse_terminated(|input| input.parse::<syn::LitStr>(), syn::Token![,])?;
                variant_attrs
                    .aliases
                    .extend(aliases.into_iter().map(|lit| lit.value()));
                return Ok(());
            }
            if meta.path.is_ident("fallback") {
                variant_attrs.fallback = true;
                return Ok(());
            }
            if meta.path.is_ident("null_variant") {
                variant_attrs.null_variant = true;
                return Ok(());
            }
            if meta.path.is_ident("scalar_variant") || meta.path.is_ident("scalar") {
                variant_attrs.scalar_variant = true;
                return Ok(());
            }
            // Handle variant = VALUE | VALUE | ... (pipe-separated match values)
            if meta.path.is_ident("variant") {
                let value = meta.value()?;
                // Parse pipe-separated values: true | "string" | 1
                loop {
                    let match_val = parse_variant_match_value(value)?;
                    variant_attrs.variant_matches.push(match_val);

                    // Check for pipe separator
                    if value.peek(syn::Token![|]) {
                        value.parse::<syn::Token![|]>()?;
                    } else {
                        break;
                    }
                }
                return Ok(());
            }
            // Handle variant_fn = "predicate_function"
            if meta.path.is_ident("variant_fn") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                variant_attrs.variant_fn = Some(s.value());
                return Ok(());
            }
            // Handle variant_value = <literal>
            if meta.path.is_ident("variant_value") {
                meta.input.parse::<syn::Token![=]>()?;
                let lit: syn::Lit = meta.input.parse()?;
                variant_attrs.variant_value = Some(lit);
                return Ok(());
            }
            // Handle variant_default flag
            if meta.path.is_ident("variant_default") {
                variant_attrs.variant_default = true;
                return Ok(());
            }
            // Handle from_tag = "field_name" for fallback variants
            if meta.path.is_ident("from_tag") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                variant_attrs.from_tag = Some(s.value());
                return Ok(());
            }
            // Handle allow_single flag for wrapping scalar into single-element array
            if meta.path.is_ident("allow_single") {
                variant_attrs.allow_single = true;
                return Ok(());
            }
            Err(meta.error("unknown feuilletage variant attribute"))
        })?;
    }

    // Validate: variant_value and variant_default are mutually exclusive
    if variant_attrs.variant_value.is_some() && variant_attrs.variant_default {
        return Err(syn::Error::new(
            attrs
                .first()
                .map_or_else(proc_macro2::Span::call_site, Spanned::span),
            "variant_value and variant_default are mutually exclusive",
        ));
    }

    Ok(variant_attrs)
}

pub(crate) fn parse_variant_attributes(attrs: &[syn::Attribute]) -> VariantAttributes {
    try_parse_variant_attributes(attrs)
        .expect("feuilletage attributes were validated before code generation")
}

#[derive(Clone)]
pub(crate) enum DefaultValue {
    Explicit(String),
    UseDefault,
    Function(String), // Call a function to get the default value
}

/// Configurable error handling mode for field deserialization.
///
/// This enum determines how errors are handled during field deserialization:
/// - `Skip`: Graceful mode - skip invalid items in collections, convert Option to None
/// - `Default`: Use field's default value on any error
/// - `Fail`: Hard stop - fail entire parsing immediately on first error
#[derive(Debug, Clone, Copy, PartialEq, Eq, std::default::Default)]
pub(crate) enum OnErrorMode {
    #[default]
    Skip, // Graceful: skip invalid items, convert to None
    Default, // Use field default on any error
    Fail,    // Hard stop: fail entire parsing
}

/// Represents a numeric range with optional min and max bounds
#[derive(Clone)]
pub(crate) struct NumericRange {
    pub(crate) min: Option<String>,
    pub(crate) max: Option<String>,
}

/// Represents a length constraint with optional min and max bounds
#[derive(Clone)]
pub(crate) struct LengthRange {
    pub(crate) min: Option<String>,
    pub(crate) max: Option<String>,
}

/// Configuration for allow_map attribute
/// Supports two syntaxes:
/// - `allow_map = "key_field"` - for object values, injects map key into specified field
/// - `allow_map(key = "key_field", scalar_as = "value_field")` - for scalar values, creates object with key and value fields
///
/// Additional options:
/// - `order_by = "field"` - sort by field name (ascending) after map-to-vec conversion
/// - `order_by_fn = "function"` - sort using custom comparison function after map-to-vec conversion
#[derive(Clone)]
pub(crate) struct AllowMapConfig {
    /// The field name where the map key should be injected
    pub(crate) key_field: String,
    /// Optional field name for the scalar value (used when map values are scalars)
    pub(crate) scalar_as_field: Option<String>,
    /// Optional field name to sort by (ascending order) after map-to-vec conversion
    pub(crate) order_by: Option<String>,
    /// Optional custom comparison function for sorting after map-to-vec conversion
    pub(crate) order_by_fn: Option<String>,
}

/// Configuration for duration attribute
/// Supports three syntaxes:
/// - `#[feuilletage(duration)]` - default unit is seconds ("s")
/// - `#[feuilletage(duration(ms))]` - shorthand: specify target unit (just the identifier)
/// - `#[feuilletage(duration(unit = ms))]` - explicit: specify target unit with named parameter
///
/// Supported units: ns, us, ms, s, m, h, d, w
#[derive(Clone)]
pub(crate) struct DurationConfig {
    /// The target unit for duration parsing (default: "s" for seconds)
    pub(crate) unit: String,
}

impl Default for DurationConfig {
    fn default() -> Self {
        Self {
            unit: "s".to_string(),
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct FieldConfigAttributes {
    /// allow_single attribute for flexible input handling:
    /// - For Vec fields: None means disabled, Some("") means wrap scalar in vec
    /// - For struct fields: Some("field") means wrap scalar as {field: scalar_value}
    ///
    /// The empty string "" is used to indicate flag-style usage (no field specified)
    pub(crate) allow_single: Option<String>,
    /// allow_map with explicit configuration (key field, optional scalar_as, order_by)
    pub(crate) allow_map: Option<AllowMapConfig>,
    /// allow_map as flag (no parameters) - uses inner type's AllowMapKeys trait for detection
    pub(crate) allow_map_flag: bool,
    /// allow_list flag for HashMap/BTreeMap fields - allows array input in addition to object input.
    /// Array items: strings become keys with default values, objects have their key-value pairs inserted.
    pub(crate) allow_list: bool,
    /// Sort by field name after map-to-vec conversion (works with allow_map flag form)
    pub(crate) order_by: Option<String>,
    /// Sort using custom function after map-to-vec conversion (works with allow_map flag form)
    pub(crate) order_by_fn: Option<String>,
    pub(crate) default_value: Option<DefaultValue>,
    pub(crate) mutable_by: Option<Vec<String>>,
    pub(crate) transform: Option<String>,
    pub(crate) transform_each: Option<String>,
    // Post-deserialization transforms (applied after FromContextValue)
    pub(crate) transform_after: Option<String>, // Transform the deserialized value
    pub(crate) transform_each_after: Option<String>, // Transform each element in collections after deserialization
    pub(crate) relative_path: bool,
    pub(crate) normalize_path: bool,
    pub(crate) coerce: bool, // Enable liberal type coercion
    // Error handling mode
    pub(crate) on_error: Option<OnErrorMode>, // How to handle errors: skip (graceful), default, fail
    // Validation attributes
    pub(crate) range: Option<NumericRange>,
    pub(crate) regex: Option<String>,
    pub(crate) length: Option<LengthRange>,
    pub(crate) validate: Option<String>,
    // Loading attributes
    pub(crate) env: Option<String>,
    // Metadata attributes
    pub(crate) deprecated: Option<String>,
    pub(crate) secret: bool,
    // Type-specific attributes
    pub(crate) absolute_path: bool,
    pub(crate) duration: Option<DurationConfig>, // Duration parsing with configurable output unit
    pub(crate) datetime: Option<String>,         // Date/time format string for chrono parsing
    // Flatten attribute - embeds nested struct fields at the same level
    pub(crate) flatten: bool,
    // Include this field's MutabilityInfo under its serialized path
    pub(crate) nested: bool,
    // Serialization attributes
    pub(crate) skip: bool,          // Always skip serializing this field
    pub(crate) skip_if_empty: bool, // Skip serializing if field is empty (Vec, Option, String, HashMap, etc.)
    pub(crate) skip_if_empty_recursive: bool, // Skip serializing if field is empty recursively (checks inner values)
    pub(crate) skip_if_default: bool,         // Skip serializing if field equals its default value
    pub(crate) skip_if: Option<String>,       // Custom skip function for serde
    // Field name mapping
    pub(crate) rename: Option<String>, // Rename field for serialization/deserialization
    pub(crate) aliases: Vec<String>, // Alternative key names for the same field (deserialization only)
    // Vec serialization control
    pub(crate) serialize_single_as_value: bool, // Serialize Vec with single item as value, empty as null
    pub(crate) serialize_single_as_value_explicit: Option<bool>, // Explicit value if user set serialize_single_as_value = false
    // Template support - allows %{field} interpolation
    pub(crate) template: bool, // Enable template interpolation for this field
    pub(crate) template_refs: Option<Vec<String>>, // Explicit field dependencies for compile-time validation
    pub(crate) template_vec_delimiter: Option<String>, // Custom delimiter for Vec fields (default ",")
    // Context injection
    pub(crate) from_context: Option<String>, // Populate field from context metadata (e.g., "source.file_path", "level.name")
    pub(crate) from_context_fn: Option<String>, // Call a user function with the context to compute the field value
    // Fallback field - use another field's value if this field is missing
    pub(crate) fallback: Option<String>, // Field name to use as fallback source
}

fn try_parse_field_config_attributes(
    attrs: &[syn::Attribute],
) -> syn::Result<FieldConfigAttributes> {
    let mut config_attrs = FieldConfigAttributes::default();

    for attr in attrs {
        if !attr.path().is_ident("feuilletage") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            // Handle allow_single with optional value:
            // - #[feuilletage(allow_single)] - flag form for Vec (empty string)
            // - #[feuilletage(allow_single = "field")] - field name for struct fields
            if meta.path.is_ident("allow_single") {
                if meta.input.peek(syn::Token![=]) {
                    // allow_single = "field" - for struct fields
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    config_attrs.allow_single = Some(s.value());
                } else {
                    // allow_single (flag form) - for Vec fields
                    config_attrs.allow_single = Some(String::new());
                }
                return Ok(());
            }
            if meta.path.is_ident("relative_path") {
                config_attrs.relative_path = true;
                return Ok(());
            }
            if meta.path.is_ident("normalize_path") {
                config_attrs.normalize_path = true;
                return Ok(());
            }
            if meta.path.is_ident("flatten") {
                config_attrs.flatten = true;
                return Ok(());
            }
            if meta.path.is_ident("nested") {
                config_attrs.nested = true;
                return Ok(());
            }
            if meta.path.is_ident("default") {
                // Check if there's a value
                if meta.input.peek(syn::Token![=]) {
                    // #[feuilletage(default = "value")]
                    let _eq: syn::Token![=] = meta.input.parse()?;
                    let value: syn::Expr = meta.input.parse()?;
                    let value = match value {
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(value),
                            ..
                        }) => value.value(),
                        value => value.into_token_stream().to_string(),
                    };
                    config_attrs.default_value = Some(DefaultValue::Explicit(value));
                } else {
                    // #[feuilletage(default)]
                    config_attrs.default_value = Some(DefaultValue::UseDefault);
                }
                return Ok(());
            }

            // Handle default_fn = "function_name"
            if meta.path.is_ident("default_fn") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.default_value = Some(DefaultValue::Function(s.value()));
                return Ok(());
            }

            // Handle allow_map with three possible syntaxes:
            // 1. allow_map (flag form) - uses inner type's AllowMapKeys trait
            // 2. allow_map = "key_field"
            // 3. allow_map(key = "key_field", scalar_as = "value_field")
            if meta.path.is_ident("allow_map") {
                if meta.input.peek(syn::Token![=]) {
                    // Syntax: allow_map = "key_field"
                    let value = meta.value()?;
                    let s: syn::LitStr = value.parse()?;
                    config_attrs.allow_map = Some(AllowMapConfig {
                        key_field: s.value(),
                        scalar_as_field: None,
                        order_by: None,
                        order_by_fn: None,
                    });
                } else if meta.input.peek(syn::token::Paren) {
                    // Syntax: allow_map(key = "key_field", scalar_as = "value_field", order_by = "field", order_by_fn = "fn")
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let mut key_field: Option<String> = None;
                    let mut value_field: Option<String> = None;
                    let mut order_by_field: Option<String> = None;
                    let mut order_by_fn_field: Option<String> = None;

                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        let _: syn::Token![=] = content.parse()?;

                        // Accept either string literal or identifier
                        let value = if content.peek(syn::LitStr) {
                            let lit: syn::LitStr = content.parse()?;
                            lit.value()
                        } else {
                            let field_ident: syn::Ident = content.parse()?;
                            field_ident.to_string()
                        };

                        if ident == "key" {
                            key_field = Some(value);
                        } else if ident == "scalar_as" {
                            value_field = Some(value);
                        } else if ident == "order_by" {
                            order_by_field = Some(value);
                        } else if ident == "order_by_fn" {
                            order_by_fn_field = Some(value);
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                "unknown feuilletage allow_map option",
                            ));
                        }

                        // Skip comma if present
                        let _ = content.parse::<syn::Token![,]>();
                    }

                    // Validate: order_by and order_by_fn are mutually exclusive
                    if order_by_field.is_some() && order_by_fn_field.is_some() {
                        return Err(syn::Error::new(
                            meta.path.span(),
                            "allow_map: order_by and order_by_fn are mutually exclusive"
                        ));
                    }

                    if let Some(key) = key_field {
                        config_attrs.allow_map = Some(AllowMapConfig {
                            key_field: key,
                            scalar_as_field: value_field,
                            order_by: order_by_field,
                            order_by_fn: order_by_fn_field,
                        });
                    } else {
                        // Flag form with optional order_by/order_by_fn: allow_map(order_by = "field")
                        config_attrs.allow_map_flag = true;
                        config_attrs.order_by = order_by_field;
                        config_attrs.order_by_fn = order_by_fn_field;
                    }
                } else {
                    // Flag form: allow_map (no parameters)
                    // Uses inner type's AllowMapKeys trait for detection
                    config_attrs.allow_map_flag = true;
                }
                return Ok(());
            }

            // Handle allow_list flag for HashMap/BTreeMap fields
            // Syntax: #[feuilletage(allow_list)]
            if meta.path.is_ident("allow_list") {
                config_attrs.allow_list = true;
                return Ok(());
            }

            if meta.path.is_ident("transform") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.transform = Some(s.value());
                return Ok(());
            }

            // Handle transform_each = "fn"
            if meta.path.is_ident("transform_each") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.transform_each = Some(s.value());
                return Ok(());
            }

            // Handle transform_after = "fn" (post-deserialization transform)
            if meta.path.is_ident("transform_after") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.transform_after = Some(s.value());
                return Ok(());
            }

            // Handle transform_each_after = "fn" (post-deserialization per-element transform)
            if meta.path.is_ident("transform_each_after") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.transform_each_after = Some(s.value());
                return Ok(());
            }

            // Handle mutable_by = ["level1", "level2"]
            if meta.path.is_ident("mutable_by") {
                let value = meta.value()?;
                let content;
                syn::bracketed!(content in value);
                let levels: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> =
                    content.parse_terminated(|input| input.parse::<syn::LitStr>(), syn::Token![,])?;
                config_attrs.mutable_by = Some(
                    levels
                        .into_iter()
                        .map(|lit| lit.value())
                        .collect()
                );
                return Ok(());
            }

            // Handle range(min, max) or range(min =, max =)
            if meta.path.is_ident("range") {
                let content;
                syn::parenthesized!(content in meta.input);
                let mut min: Option<String> = None;
                let mut max: Option<String> = None;

                // Parse comma-separated values or named arguments
                while !content.is_empty() {
                    if content.peek(syn::Ident) {
                        let ident: syn::Ident = content.parse()?;
                        let _: syn::Token![=] = content.parse()?;
                        let lit: syn::Lit = content.parse()?;
                        let value = match lit {
                            syn::Lit::Int(i) => i.base10_digits().to_string(),
                            syn::Lit::Float(f) => f.base10_digits().to_string(),
                            _ => continue,
                        };
                        if ident == "min" {
                            min = Some(value);
                        } else if ident == "max" {
                            max = Some(value);
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                "unknown feuilletage range option",
                            ));
                        }
                    } else {
                        // Positional: range(min, max)
                        let lit: syn::Lit = content.parse()?;
                        let value = match lit {
                            syn::Lit::Int(i) => i.base10_digits().to_string(),
                            syn::Lit::Float(f) => f.base10_digits().to_string(),
                            _ => continue,
                        };
                        if min.is_none() {
                            min = Some(value);
                        } else {
                            max = Some(value);
                        }
                    }
                    // Skip comma if present
                    let _ = content.parse::<syn::Token![,]>();
                }
                config_attrs.range = Some(NumericRange { min, max });
                return Ok(());
            }

            // Handle regex = "pattern"
            if meta.path.is_ident("regex") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.regex = Some(s.value());
                return Ok(());
            }

            // Handle length(min, max) or length(min =, max =)
            if meta.path.is_ident("length") {
                let content;
                syn::parenthesized!(content in meta.input);
                let mut min: Option<String> = None;
                let mut max: Option<String> = None;

                while !content.is_empty() {
                    if content.peek(syn::Ident) {
                        let ident: syn::Ident = content.parse()?;
                        let _: syn::Token![=] = content.parse()?;
                        let lit: syn::LitInt = content.parse()?;
                        let value = lit.base10_digits().to_string();
                        if ident == "min" {
                            min = Some(value);
                        } else if ident == "max" {
                            max = Some(value);
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                "unknown feuilletage length option",
                            ));
                        }
                    } else {
                        // Positional: length(min, max)
                        let lit: syn::LitInt = content.parse()?;
                        let value = lit.base10_digits().to_string();
                        if min.is_none() {
                            min = Some(value);
                        } else {
                            max = Some(value);
                        }
                    }
                    let _ = content.parse::<syn::Token![,]>();
                }
                config_attrs.length = Some(LengthRange { min, max });
                return Ok(());
            }

            // Handle validate = "function_name"
            if meta.path.is_ident("validate") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.validate = Some(s.value());
                return Ok(());
            }

            // Handle env = "VAR_NAME"
            if meta.path.is_ident("env") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.env = Some(s.value());
                return Ok(());
            }

            // Handle deprecated = "message" or deprecated (flag)
            if meta.path.is_ident("deprecated") {
                if meta.input.peek(syn::Token![=]) {
                    let _eq: syn::Token![=] = meta.input.parse()?;
                    let s: syn::LitStr = meta.input.parse()?;
                    config_attrs.deprecated = Some(s.value());
                } else {
                    config_attrs.deprecated = Some("This field is deprecated".to_string());
                }
                return Ok(());
            }

            // Handle secret flag
            if meta.path.is_ident("secret") {
                config_attrs.secret = true;
                return Ok(());
            }

            // Handle absolute_path flag
            if meta.path.is_ident("absolute_path") {
                config_attrs.absolute_path = true;
                return Ok(());
            }

            // Handle duration attribute with optional unit parameter
            // Syntax: #[feuilletage(duration)] or #[feuilletage(duration(ms))] or #[feuilletage(duration(unit = ms))]
            if meta.path.is_ident("duration") {
                if meta.input.peek(syn::token::Paren) {
                    // Syntax: duration(ms) or duration(unit = ms)
                    let content;
                    syn::parenthesized!(content in meta.input);

                    if content.is_empty() {
                        // Empty parens: duration() - defaults to seconds
                        config_attrs.duration = Some(DurationConfig { unit: "s".to_string() });
                    } else {
                        // Parse first identifier
                        let first_ident: syn::Ident = content.parse()?;

                        // Check if this is explicit syntax: duration(unit = ms)
                        if first_ident == "unit" && content.peek(syn::Token![=]) {
                            // Explicit syntax: duration(unit = ms)
                            let _: syn::Token![=] = content.parse()?;
                            let unit_ident: syn::Ident = content.parse()?;
                            let unit_value = unit_ident.to_string();
                            // Validate unit
                            match unit_value.as_str() {
                                "ns" | "us" | "ms" | "s" | "m" | "h" | "d" | "w" => {
                                    config_attrs.duration = Some(DurationConfig { unit: unit_value });
                                }
                                _ => {
                                    return Err(syn::Error::new(
                                        unit_ident.span(),
                                        format!("Invalid duration unit '{}'. Valid units: ns, us, ms, s, m, h, d, w", unit_value)
                                    ));
                                }
                            }
                        } else {
                            // Shorthand syntax: duration(ms)
                            let unit_value = first_ident.to_string();
                            // Validate unit
                            match unit_value.as_str() {
                                "ns" | "us" | "ms" | "s" | "m" | "h" | "d" | "w" => {
                                    config_attrs.duration = Some(DurationConfig { unit: unit_value });
                                }
                                _ => {
                                    return Err(syn::Error::new(
                                        first_ident.span(),
                                        format!("Invalid duration unit '{}'. Valid units: ns, us, ms, s, m, h, d, w", unit_value)
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    // Flag form: duration (defaults to seconds)
                    config_attrs.duration = Some(DurationConfig { unit: "s".to_string() });
                }
                return Ok(());
            }

            // Handle coerce flag - enable liberal type coercion
            if meta.path.is_ident("coerce") {
                config_attrs.coerce = true;
                return Ok(());
            }

            // Handle datetime = "fmt" for date/time parsing
            if meta.path.is_ident("datetime") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.datetime = Some(s.value());
                return Ok(());
            }

            // Handle skip flag - always skip serializing this field
            if meta.path.is_ident("skip") {
                config_attrs.skip = true;
                return Ok(());
            }

            // Handle skip_if_empty flag for serde serialization
            if meta.path.is_ident("skip_if_empty") {
                config_attrs.skip_if_empty = true;
                return Ok(());
            }

            // Handle skip_if_empty_recursive flag for serde serialization
            // Checks inner values (e.g., Option<String> skips for None AND Some(""))
            if meta.path.is_ident("skip_if_empty_recursive") {
                config_attrs.skip_if_empty_recursive = true;
                return Ok(());
            }

            // Handle skip_if_default flag for serde serialization
            // Skips when field equals its default value (from default attr or Default trait)
            if meta.path.is_ident("skip_if_default") {
                config_attrs.skip_if_default = true;
                return Ok(());
            }

            // Handle skip_if = "function_name" for custom skip function
            if meta.path.is_ident("skip_if") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.skip_if = Some(s.value());
                return Ok(());
            }

            // Handle rename = "new_name"
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.rename = Some(s.value());
                return Ok(());
            }

            // Handle alias = "value" (singular form)
            if meta.path.is_ident("alias") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.aliases.push(s.value());
                return Ok(());
            }

            // Handle aliases = ["alias1", "alias2"]
            if meta.path.is_ident("aliases") {
                let value = meta.value()?;
                let content;
                syn::bracketed!(content in value);
                let aliases: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> =
                    content.parse_terminated(|input| input.parse::<syn::LitStr>(), syn::Token![,])?;
                config_attrs.aliases.extend(aliases.into_iter().map(|lit| lit.value()));
                return Ok(());
            }

            // Handle serialize_single_as_value for Vec serialization
            // Empty Vec -> null/skip, single item -> unwrapped value, multiple items -> array
            // Supports: serialize_single_as_value (flag) or serialize_single_as_value = true/false
            if meta.path.is_ident("serialize_single_as_value") {
                if meta.input.peek(syn::Token![=]) {
                    // serialize_single_as_value = true/false
                    let _eq: syn::Token![=] = meta.input.parse()?;
                    let lit: syn::LitBool = meta.input.parse()?;
                    config_attrs.serialize_single_as_value = lit.value();
                    config_attrs.serialize_single_as_value_explicit = Some(lit.value());
                } else {
                    // serialize_single_as_value (flag, means true)
                    config_attrs.serialize_single_as_value = true;
                    config_attrs.serialize_single_as_value_explicit = Some(true);
                }
                return Ok(());
            }

            // Handle template attribute for field reference interpolation
            // Supports:
            // - template (flag form)
            // - template(refs = ["field1", "field2"])
            // - template(vec_delimiter = ",")
            // - template(refs = ["field1"], vec_delimiter = ",")
            if meta.path.is_ident("template") {
                config_attrs.template = true;

                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);

                    while !content.is_empty() {
                        let ident: syn::Ident = content.parse()?;
                        let _: syn::Token![=] = content.parse()?;

                        if ident == "refs" {
                            let refs_content;
                            syn::bracketed!(refs_content in content);
                            let refs: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> =
                                refs_content.parse_terminated(|input| input.parse::<syn::LitStr>(), syn::Token![,])?;
                            config_attrs.template_refs = Some(
                                refs.into_iter()
                                    .map(|lit| lit.value())
                                    .collect()
                            );
                        } else if ident == "vec_delimiter" {
                            let lit: syn::LitStr = content.parse()?;
                            config_attrs.template_vec_delimiter = Some(lit.value());
                        } else {
                            return Err(syn::Error::new(
                                ident.span(),
                                "unknown feuilletage template option",
                            ));
                        }

                        // Skip comma if present
                        let _ = content.parse::<syn::Token![,]>();
                    }
                }
                return Ok(());
            }

            // Handle on_error = skip | default | fail (accepts both string literal and identifier)
            if meta.path.is_ident("on_error") {
                let value = meta.value()?;
                // Try parsing as string literal first, then as identifier
                let (mode_str, span) = if value.peek(syn::LitStr) {
                    let lit: syn::LitStr = value.parse()?;
                    (lit.value(), lit.span())
                } else {
                    let ident: syn::Ident = value.parse()?;
                    (ident.to_string(), ident.span())
                };
                config_attrs.on_error = Some(match mode_str.as_str() {
                    "skip" => OnErrorMode::Skip,
                    "default" => OnErrorMode::Default,
                    "fail" => OnErrorMode::Fail,
                    _ => return Err(syn::Error::new(
                        span,
                        "on_error must be one of: skip, default, fail"
                    )),
                });
                return Ok(());
            }

            // from_context = "path" - populate field from context metadata
            if meta.path.is_ident("from_context") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                let path = s.value();
                // Validate supported paths
                let valid_paths = ["source.file_path", "source.display_name", "level.name"];
                if !valid_paths.contains(&path.as_str()) {
                    return Err(syn::Error::new(
                        s.span(),
                        format!(
                            "Invalid from_context path: '{}'. Supported paths: {}",
                            path,
                            valid_paths.join(", ")
                        )
                    ));
                }
                if config_attrs.from_context_fn.is_some() {
                    return Err(syn::Error::new(
                        s.span(),
                        "from_context and from_context_fn are mutually exclusive",
                    ));
                }
                config_attrs.from_context = Some(path);
                return Ok(());
            }

            // from_context_fn = "function_name" - call a function with context to compute value
            if meta.path.is_ident("from_context_fn") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                if config_attrs.from_context.is_some() {
                    return Err(syn::Error::new(
                        s.span(),
                        "from_context and from_context_fn are mutually exclusive",
                    ));
                }
                config_attrs.from_context_fn = Some(s.value());
                return Ok(());
            }

            // fallback = "field_name" - use another field's value if this field is missing
            if meta.path.is_ident("fallback") {
                let value = meta.value()?;
                let s: syn::LitStr = value.parse()?;
                config_attrs.fallback = Some(s.value());
                return Ok(());
            }

            Err(meta.error("unknown feuilletage field attribute"))
        })?;
    }

    Ok(config_attrs)
}

pub(crate) fn parse_field_config_attributes(attrs: &[syn::Attribute]) -> FieldConfigAttributes {
    try_parse_field_config_attributes(attrs)
        .expect("feuilletage attributes were validated before code generation")
}

pub(crate) fn validate_feuilletage_attributes(input: &syn::DeriveInput) -> syn::Result<()> {
    let container_attrs = try_parse_container_attributes(&input.attrs)?;
    let is_projection = container_attrs.parse_as.is_some();

    if is_projection && container_attrs.mutable_by.is_some() {
        return Err(syn::Error::new(
            container_attrs.parse_as.as_ref().unwrap().span(),
            "`mutable_by` must be defined on the `parse_as` wire type",
        ));
    }

    match &input.data {
        syn::Data::Struct(data) => {
            for field in &data.fields {
                let field_attrs = try_parse_field_config_attributes(&field.attrs)?;
                if is_projection && field_attrs.mutable_by.is_some() {
                    return Err(syn::Error::new(
                        field.span(),
                        "`mutable_by` must be defined on the `parse_as` wire type",
                    ));
                }
            }
        }
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                try_parse_variant_attributes(&variant.attrs)?;
                for field in &variant.fields {
                    let field_attrs = try_parse_field_config_attributes(&field.attrs)?;
                    if is_projection && field_attrs.mutable_by.is_some() {
                        return Err(syn::Error::new(
                            field.span(),
                            "`mutable_by` must be defined on the `parse_as` wire type",
                        ));
                    }
                }
            }
        }
        syn::Data::Union(data) => {
            for field in &data.fields.named {
                try_parse_field_config_attributes(&field.attrs)?;
            }
        }
    }

    Ok(())
}

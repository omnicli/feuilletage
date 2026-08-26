//! Small utility helpers shared between `attrs.rs` and the codegen
//! functions in `lib.rs`. No state, no cross-module dependencies —
//! pure functions over `syn::Type` / `&str`.

use syn::Type;

/// Convert PascalCase to snake_case
pub(crate) fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert PascalCase to camelCase
pub(crate) fn to_camel_case(s: &str) -> String {
    let snake = to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert PascalCase to kebab-case
pub(crate) fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert PascalCase to SCREAMING_SNAKE_CASE
pub(crate) fn to_screaming_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

/// Check if a type is Option<T>
pub(crate) fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Check if a type is String
pub(crate) fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

/// Check if a type is a HashMap or BTreeMap. Returns the map kind ("HashMap" or "BTreeMap") if it is.
pub(crate) fn get_map_kind(ty: &Type) -> Option<&'static str> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            match name.as_str() {
                "HashMap" => return Some("HashMap"),
                "BTreeMap" => return Some("BTreeMap"),
                _ => {}
            }
        }
    }
    None
}

/// Extract the value type V from HashMap<K, V> or BTreeMap<K, V>.
/// Returns Some(V) if the type is a map with two generic arguments.
pub(crate) fn extract_map_value_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            if name == "HashMap" || name == "BTreeMap" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Get the second generic argument (the value type V)
                    let mut iter = args.args.iter();
                    iter.next(); // Skip K
                    if let Some(syn::GenericArgument::Type(value_ty)) = iter.next() {
                        return Some(value_ty);
                    }
                }
            }
        }
    }
    None
}

/// Check if a type is bool
pub(crate) fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}

/// Check if a type is a signed integer (i8, i16, i32, i64, isize)
pub(crate) fn is_signed_int_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            return matches!(name.as_str(), "i8" | "i16" | "i32" | "i64" | "isize");
        }
    }
    false
}

/// Check if a type is an unsigned integer (u8, u16, u32, u64, usize)
pub(crate) fn is_unsigned_int_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            return matches!(name.as_str(), "u8" | "u16" | "u32" | "u64" | "usize");
        }
    }
    false
}

/// Check if a type is a float (f32, f64)
pub(crate) fn is_float_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let name = segment.ident.to_string();
            return matches!(name.as_str(), "f32" | "f64");
        }
    }
    false
}

/// Get the simple type name from a path-typed `Type`, e.g. `Option` from `Option<T>`.
pub(crate) fn get_type_name(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.to_string();
        }
    }
    String::new()
}

/// Parse a transform function path into a TokenStream.
/// Bare names (no `::`) are prefixed with `compote::transform::` for built-in transforms.
/// Paths containing `::` are used as-is, allowing custom transform functions.
pub(crate) fn parse_transform_path(path: &str) -> proc_macro2::TokenStream {
    if path.contains("::") {
        path.parse().unwrap()
    } else {
        format!("compote::transform::{}", path).parse().unwrap()
    }
}

/// Extract the inner type from a generic type like Option<T> or Vec<T>
pub(crate) fn get_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                    return Some(inner_ty);
                }
            }
        }
    }
    None
}

/// Check if a default value looks like a raw string that needs conversion
/// Returns true if the value doesn't contain Rust expression syntax
pub(crate) fn is_raw_string_default(value: &str) -> bool {
    // If it looks like a Rust expression, it's not a raw string.
    // A raw string is a simple literal value like "hello" or "https://example.com".
    //
    // Things that indicate it's NOT a raw string (i.e., it's Rust code):
    // - Contains `::` (path separator, e.g., "String::from")
    // - Contains `(` (function/method call, e.g., "foo()" or ".to_string()")
    // - Contains `!` (macro invocation, e.g., "format!")
    // - Starts with `"` (already a string literal, e.g., "\"hello\"")
    // - Starts with `'` (char literal)
    //
    // Note: We do NOT check for `.` because dots appear in:
    // - URLs: "https://example.com"
    // - File paths: "config.yaml"
    // - Decimal numbers: "3.14"
    // - Domain-style identifiers: "my.config.value()"
    //
    // If someone wants a method call like `foo.bar()`, they must include the `()`.
    !value.contains("::")
        && !value.contains('(')
        && !value.contains('!')
        && !value.starts_with('"')
        && !value.starts_with('\'')
}

/// Convert a default value for String fields
/// If the value is a raw string like "hello", convert to "\"hello\".to_string()"
pub(crate) fn convert_string_default(value: &str, field_type: &Type) -> String {
    if is_string_type(field_type) && is_raw_string_default(value) {
        // Escape any quotes in the value and wrap it
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\".to_string()", escaped)
    } else if get_type_name(field_type) == "Vec" && value.starts_with('[') && value.ends_with(']') {
        format!("({}).into()", value)
    } else {
        value.to_string()
    }
}

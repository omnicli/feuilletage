//! Codegen for `serde::Serialize` impls and all related skip-condition
//! logic. Consumed by struct codegen and (indirectly) by enum codegen
//! that emits per-variant Serialize bodies.

use quote::quote;
use syn::Type;

use crate::attrs::{parse_field_config_attributes, DefaultValue, FieldConfigAttributes};
use crate::helpers::{
    get_inner_type, get_type_name, is_pathbuf_type, is_raw_string_default, is_string_type,
    option_inner_type,
};

/// Information about a field for serialization
pub(crate) struct SerializeFieldInfo {
    field_name: syn::Ident,
    field_name_str: String,
    field_type: Type,
    attrs: FieldConfigAttributes,
}

/// Determine if a field should use serialize_single_as_value behavior
/// Returns true if:
/// - explicitly set serialize_single_as_value = true, OR
/// - has allow_single (flag form) AND did not explicitly set serialize_single_as_value = false
pub(crate) fn should_serialize_single_as_value(attrs: &FieldConfigAttributes) -> bool {
    match attrs.serialize_single_as_value_explicit {
        Some(explicit) => explicit, // User explicitly set it
        // Auto-enable for allow_single fields in flag form (empty string = Vec wrapping)
        None => attrs.allow_single.as_ref().is_some_and(|s| s.is_empty()),
    }
}

fn expand_home_serialized_value(info: &SerializeFieldInfo) -> Option<proc_macro2::TokenStream> {
    let enabled = info.attrs.expand_home || info.attrs.transform.as_deref() == Some("expand_home");
    if !enabled {
        return None;
    }

    let field_name = &info.field_name;
    if is_string_type(&info.field_type) {
        return Some(quote! {
            feuilletage::transform::contract_home_str(&self.#field_name)
        });
    }
    if is_pathbuf_type(&info.field_type) {
        return Some(quote! {
            feuilletage::transform::contract_home_path(&self.#field_name)
        });
    }
    if let Some(inner) = option_inner_type(&info.field_type) {
        if is_string_type(inner) {
            return Some(quote! {
                self.#field_name
                    .as_ref()
                    .map(|value| feuilletage::transform::contract_home_str(value))
            });
        }
        if is_pathbuf_type(inner) {
            return Some(quote! {
                self.#field_name
                    .as_ref()
                    .map(|value| feuilletage::transform::contract_home_path(value))
            });
        }
    }

    None
}

/// Generate Serialize implementation for a struct
pub(crate) fn generate_serialize_impl(
    name: &syn::Ident,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::Token![,]>,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    // Collect field info
    let field_infos: Vec<SerializeFieldInfo> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().clone();
            let field_name_str = field_name.to_string();
            let field_type = field.ty.clone();
            let attrs = parse_field_config_attributes(&field.attrs);
            SerializeFieldInfo {
                field_name,
                field_name_str,
                field_type,
                attrs,
            }
        })
        .collect();

    // Check if any field has flatten attribute - requires special handling
    let has_flatten = field_infos.iter().any(|f| f.attrs.flatten);

    // If there are flattened fields, use a different serialization strategy
    if has_flatten {
        return generate_serialize_impl_with_flatten(
            name,
            &field_infos,
            impl_generics,
            ty_generics,
            where_clause,
        );
    }

    // Check if any field has skip conditions (other than unconditional skip)
    // Also include serialize_single_as_value or allow_map since they may skip (empty Vec)
    let has_conditional_skips = field_infos.iter().any(|f| {
        !f.attrs.skip
            && (f.attrs.skip_if_empty
                || f.attrs.skip_if_empty_recursive
                || f.attrs.skip_if_default
                || f.attrs.skip_if.is_some()
                || should_serialize_single_as_value(&f.attrs)
                || f.attrs.allow_map.is_some())
    });

    // Generate skip condition checks for each field
    let field_skip_conditions: Vec<proc_macro2::TokenStream> = field_infos
        .iter()
        .map(|info| generate_skip_condition(&info.field_name, &info.field_type, &info.attrs))
        .collect();

    // Generate field count calculation
    let field_count_increments: Vec<proc_macro2::TokenStream> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            let condition = &field_skip_conditions[idx];
            if info.attrs.skip
                || info.attrs.from_context.is_some()
                || info.attrs.from_context_fn.is_some()
            {
                // Unconditional skip (or from_context which is not serialized) - never count
                quote! {}
            } else if condition.is_empty() {
                // No skip condition - always count
                quote! { __field_count += 1; }
            } else {
                // Conditional skip - count if not skipped
                let skip_var = syn::Ident::new(
                    &format!("__skip_{}", info.field_name),
                    info.field_name.span(),
                );
                quote! {
                    if !#skip_var {
                        __field_count += 1;
                    }
                }
            }
        })
        .collect();

    // Generate skip variable declarations
    let skip_var_declarations: Vec<proc_macro2::TokenStream> = field_infos
        .iter()
        .enumerate()
        .map(|(idx, info)| {
            let condition = &field_skip_conditions[idx];
            if info.attrs.skip
                || info.attrs.from_context.is_some()
                || info.attrs.from_context_fn.is_some()
                || condition.is_empty()
            {
                quote! {}
            } else {
                let skip_var = syn::Ident::new(
                    &format!("__skip_{}", info.field_name),
                    info.field_name.span(),
                );
                let field_name = &info.field_name;
                quote! {
                    let #skip_var = {
                        let value = &self.#field_name;
                        #condition
                    };
                }
            }
        })
        .collect();

    // Generate serialize_field calls
    let serialize_fields: Vec<proc_macro2::TokenStream> = field_infos.iter().enumerate().map(|(idx, info)| {
        let field_name = &info.field_name;
        // Use rename if specified, otherwise use the field name
        let serialized_key = info.attrs.rename.as_ref().unwrap_or(&info.field_name_str);
        let condition = &field_skip_conditions[idx];

        if info.attrs.skip || info.attrs.from_context.is_some() || info.attrs.from_context_fn.is_some() {
            // Unconditional skip (or from_context which is not serialized) - never serialize
            quote! {}
        } else if let Some(ref allow_map_config) = info.attrs.allow_map {
            // allow_map smart serialization: serialize as map with key -> value/object
            let skip_var = syn::Ident::new(&format!("__skip_{}", field_name), field_name.span());
            let key_field = &allow_map_config.key_field;

            if let Some(ref scalar_as_field) = allow_map_config.scalar_as_field {
                // Smart serialization: if item only has key and scalar_as populated (rest default),
                // serialize as {key: scalar_as_value}; otherwise serialize as {key: full_object}
                quote! {
                    if !#skip_var {
                        // Build a map from Vec items
                        let mut __field_map = serde_json::Map::new();
                        for item in &self.#field_name {
                            // Get the key value using serde_json for reflection
                            let json_value = serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                            if let serde_json::Value::Object(obj) = json_value {
                                if let Some(serde_json::Value::String(key_val)) = obj.get(#key_field).cloned() {
                                    // Check if this item can be serialized in compact form
                                    // (only key and scalar_as fields are non-default)
                                    let is_compact = {
                                        let field_count = obj.len();
                                        let has_key = obj.contains_key(#key_field);
                                        let has_scalar = obj.contains_key(#scalar_as_field);
                                        // Item is compact if it has exactly 2 fields (key and scalar_as)
                                        // or if all other fields are at their default/empty values
                                        if field_count == 2 && has_key && has_scalar {
                                            true
                                        } else {
                                            // Check if non-key/scalar fields are default
                                            obj.iter().all(|(k, v)| {
                                                k == #key_field || k == #scalar_as_field || {
                                                    // Consider empty arrays, empty objects, null as default
                                                    match v {
                                                        serde_json::Value::Null => true,
                                                        serde_json::Value::Array(a) => a.is_empty(),
                                                        serde_json::Value::Object(o) => o.is_empty(),
                                                        serde_json::Value::String(s) => s.is_empty(),
                                                        serde_json::Value::Bool(false) => true,
                                                        serde_json::Value::Number(n) => n.as_i64() == Some(0) || n.as_f64() == Some(0.0),
                                                        _ => false,
                                                    }
                                                }
                                            })
                                        }
                                    };

                                    if is_compact {
                                        // Serialize as compact: {key: scalar_as_value}
                                        if let Some(scalar_val) = obj.get(#scalar_as_field).cloned() {
                                            __field_map.insert(key_val, scalar_val);
                                        } else {
                                            __field_map.insert(key_val, serde_json::Value::Null);
                                        }
                                    } else {
                                        // Serialize as full object (without the key field in the value)
                                        let mut obj_without_key = obj.clone();
                                        obj_without_key.remove(#key_field);
                                        __field_map.insert(key_val, serde_json::Value::Object(obj_without_key));
                                    }
                                }
                            }
                        }
                        __state.serialize_field(#serialized_key, &__field_map)?;
                    }
                }
            } else {
                // Simple allow_map: serialize as {key: full_object}
                quote! {
                    if !#skip_var {
                        let mut __field_map = serde_json::Map::new();
                        for item in &self.#field_name {
                            let json_value = serde_json::to_value(item).unwrap_or(serde_json::Value::Null);
                            if let serde_json::Value::Object(obj) = json_value {
                                if let Some(serde_json::Value::String(key_val)) = obj.get(#key_field).cloned() {
                                    let mut obj_without_key = obj.clone();
                                    obj_without_key.remove(#key_field);
                                    __field_map.insert(key_val, serde_json::Value::Object(obj_without_key));
                                }
                            }
                        }
                        __state.serialize_field(#serialized_key, &__field_map)?;
                    }
                }
            }
        } else if should_serialize_single_as_value(&info.attrs) {
            // Special serialization for collection types (Vec, BTreeSet, HashSet, etc.):
            // empty -> skip, single -> value, multiple -> array
            let skip_var = syn::Ident::new(&format!("__skip_{}", field_name), field_name.span());
            quote! {
                if !#skip_var {
                    // Collection has at least one element (skip condition handles empty)
                    if self.#field_name.len() == 1 {
                        // Serialize single element as unwrapped value
                        // Use iter().next() instead of [0] to work with all collection types
                        __state.serialize_field(#serialized_key, self.#field_name.iter().next().unwrap())?;
                    } else {
                        // Serialize as array for 2+ elements
                        __state.serialize_field(#serialized_key, &self.#field_name)?;
                    }
                }
            }
        } else if let Some(serialized_value) = expand_home_serialized_value(info) {
            if condition.is_empty() {
                quote! {
                    __state.serialize_field(#serialized_key, &#serialized_value)?;
                }
            } else {
                let skip_var = syn::Ident::new(&format!("__skip_{}", field_name), field_name.span());
                quote! {
                    if !#skip_var {
                        __state.serialize_field(#serialized_key, &#serialized_value)?;
                    }
                }
            }
        } else if condition.is_empty() {
            // No skip condition - always serialize
            quote! {
                __state.serialize_field(#serialized_key, &self.#field_name)?;
            }
        } else {
            // Conditional skip
            let skip_var = syn::Ident::new(&format!("__skip_{}", field_name), field_name.span());
            quote! {
                if !#skip_var {
                    __state.serialize_field(#serialized_key, &self.#field_name)?;
                }
            }
        }
    }).collect();

    let struct_name_str = name.to_string();

    // Count non-skip fields for the simple case
    let static_field_count = field_infos.iter().filter(|f| !f.attrs.skip).count();

    if has_conditional_skips {
        // Dynamic field count due to conditional skips
        quote! {
            impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
                fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
                where
                    __S: serde::Serializer,
                {
                    use serde::ser::SerializeStruct;

                    // Pre-compute skip conditions
                    #(#skip_var_declarations)*

                    // Count fields that will be serialized
                    let mut __field_count: usize = 0;
                    #(#field_count_increments)*

                    let mut __state = serializer.serialize_struct(#struct_name_str, __field_count)?;

                    #(#serialize_fields)*

                    __state.end()
                }
            }
        }
    } else {
        // Static field count (no conditional skips, only unconditional skips)
        quote! {
            impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
                fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
                where
                    __S: serde::Serializer,
                {
                    use serde::ser::SerializeStruct;

                    let mut __state = serializer.serialize_struct(#struct_name_str, #static_field_count)?;

                    #(#serialize_fields)*

                    __state.end()
                }
            }
        }
    }
}

/// Generate Serialize impl for structs with flatten attribute
/// Uses SerializeMap to allow merging fields from flattened structs into the parent
pub(crate) fn generate_serialize_impl_with_flatten(
    name: &syn::Ident,
    field_infos: &[SerializeFieldInfo],
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    // Generate serialization logic for each field
    let serialize_entries: Vec<proc_macro2::TokenStream> = field_infos
        .iter()
        .map(|info| {
            let field_name = &info.field_name;
            let serialized_key = info.attrs.rename.as_ref().unwrap_or(&info.field_name_str);

            // Skip fields with skip attribute or from_context
            if info.attrs.skip
                || info.attrs.from_context.is_some()
                || info.attrs.from_context_fn.is_some()
            {
                return quote! {};
            }

            if info.attrs.flatten {
                // Flattened field: serialize to JSON, then merge its fields into the parent map
                quote! {
                    // Serialize flattened field and merge its entries
                    let __flattened_value = serde_json::to_value(&self.#field_name)
                        .map_err(serde::ser::Error::custom)?;
                    if let serde_json::Value::Object(__obj) = __flattened_value {
                        for (__key, __val) in __obj {
                            __map.serialize_entry(&__key, &__val)?;
                        }
                    }
                }
            } else {
                // Regular field: serialize as entry
                quote! {
                    __map.serialize_entry(#serialized_key, &self.#field_name)?;
                }
            }
        })
        .collect();

    quote! {
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
            where
                __S: serde::Serializer,
            {
                use serde::ser::SerializeMap;

                // Use serialize_map with None length since we don't know total count with flattened fields
                let mut __map = serializer.serialize_map(None)?;

                #(#serialize_entries)*

                __map.end()
            }
        }
    }
}

/// Generate the skip condition expression for a field
pub(crate) fn generate_skip_condition(
    _field_name: &syn::Ident,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // If unconditional skip, return empty (handled separately)
    if attrs.skip {
        return quote! {};
    }

    let mut conditions = Vec::new();

    // skip_if_empty
    if attrs.skip_if_empty {
        let check = get_skip_check_for_type(field_type);
        conditions.push(check);
    }

    // skip_if_empty_recursive
    if attrs.skip_if_empty_recursive {
        let check = get_skip_check_recursive_for_type(field_type);
        conditions.push(check);
    }

    // skip_if_default
    if attrs.skip_if_default {
        let check = get_skip_check_for_default(field_type, attrs);
        conditions.push(check);
    }

    // skip_if custom function
    if let Some(ref custom_fn) = attrs.skip_if {
        let custom_fn_ident: proc_macro2::TokenStream = custom_fn.parse().unwrap();
        conditions.push(quote! { #custom_fn_ident(value) });
    }

    // serialize_single_as_value (or allow_single with auto-inferred serialization) - skip when Vec is empty
    if should_serialize_single_as_value(attrs) {
        conditions.push(quote! { value.is_empty() });
    }

    // allow_map - skip when Vec is empty (serialized as map)
    if attrs.allow_map.is_some() && !should_serialize_single_as_value(attrs) {
        // Only add is_empty check if not already added by serialize_single_as_value
        conditions.push(quote! { value.is_empty() });
    }

    // Combine conditions with OR (skip if any condition is true)
    if conditions.is_empty() {
        quote! {}
    } else if conditions.len() == 1 {
        conditions.into_iter().next().unwrap()
    } else {
        // Multiple conditions - combine with ||
        let first = conditions.remove(0);
        conditions.into_iter().fold(first, |acc, cond| {
            quote! { (#acc) || (#cond) }
        })
    }
}

/// Get the appropriate skip check expression for a given type.
/// Uses the feuilletage::IsEmpty trait, allowing custom types to implement
/// the trait and work with skip_if_empty.
pub(crate) fn get_skip_check_for_type(_ty: &Type) -> proc_macro2::TokenStream {
    quote! { ::feuilletage::IsEmpty::is_empty(value) }
}

/// Get the appropriate recursive skip check expression for a given type.
/// This checks inner values for emptiness (e.g., Option<String> skips for None AND Some("")).
/// Uses the feuilletage::IsEmpty trait for emptiness checks.
pub(crate) fn get_skip_check_recursive_for_type(ty: &Type) -> proc_macro2::TokenStream {
    let type_str = get_type_name(ty);

    match type_str.as_str() {
        "Option" => {
            // For Option, check None OR inner is empty
            if let Some(inner_ty) = get_inner_type(ty) {
                let inner_type_str = get_type_name(inner_ty);
                if inner_type_str == "Option" {
                    // Nested Option - recursively check
                    let inner_check = get_skip_check_recursive_for_type(inner_ty);
                    quote! {
                        match value {
                            None => true,
                            Some(inner) => {
                                let value = inner;
                                #inner_check
                            },
                        }
                    }
                } else {
                    // Use IsEmpty trait for inner value
                    quote! {
                        match value {
                            None => true,
                            Some(inner) => ::feuilletage::IsEmpty::is_empty(inner),
                        }
                    }
                }
            } else {
                quote! { value.is_none() }
            }
        }
        // For non-Option types, just use IsEmpty
        _ => quote! { ::feuilletage::IsEmpty::is_empty(value) },
    }
}

/// Get the appropriate skip check for default value comparison
pub(crate) fn get_skip_check_for_default(
    ty: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // If there's an explicit default value, compare against it
    if let Some(ref default_val) = attrs.default_value {
        match default_val {
            DefaultValue::Explicit(expr) => {
                // Convert raw string defaults for String type
                if is_string_type(ty) && is_raw_string_default(expr) {
                    let default = syn::LitStr::new(expr, proc_macro2::Span::call_site());
                    quote! { value.as_str() == #default }
                } else {
                    let default_tokens: proc_macro2::TokenStream = expr.parse().unwrap();
                    quote! { *value == #default_tokens }
                }
            }
            DefaultValue::UseDefault => {
                // Use Default::default() for comparison with explicit type
                quote! { *value == <#ty as Default>::default() }
            }
            DefaultValue::Function(fn_name) => {
                // Call the default function and compare
                let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
                quote! { *value == #fn_ident() }
            }
        }
    } else {
        // No explicit default - use Default trait with explicit type
        quote! { *value == <#ty as Default>::default() }
    }
}

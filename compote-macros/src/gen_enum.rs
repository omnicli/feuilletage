//! Codegen for all enum dispatch shapes: `tag = ".."` (internally tagged),
//! `untagged`, `external_tag` (map-key dispatch), and `value_matched`
//! (scalar value matches a variant). Includes the variant-predicate
//! helpers (`has_null_match`, `has_any_scalar_matching`, etc.) and the
//! shared `generate_deserialize_via_from_context_value` helper that all
//! generated enum impls use to bridge to `serde::Deserialize`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Fields, Variant};

use crate::attrs::{
    parse_field_config_attributes, parse_variant_attributes, ContainerAttributes, RenameAllCase,
    VariantAttributes, VariantMatch,
};
use crate::helpers::to_snake_case;

/// Generate FromContextValue implementation for enums
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_enum_impl(
    name: &syn::Ident,
    _generics: &syn::Generics,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    container_attrs: &ContainerAttributes,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    let skip_serialize = container_attrs.skip_serialize;
    let base = if container_attrs.untagged {
        generate_untagged_enum_impl(
            name,
            variants,
            skip_serialize,
            impl_generics,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else if container_attrs.value_matched {
        generate_value_matched_enum_impl(
            name,
            variants,
            skip_serialize,
            impl_generics,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else if container_attrs.external_tag {
        generate_external_tag_enum_impl(
            name,
            variants,
            container_attrs.rename_all,
            skip_serialize,
            impl_generics,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else if let Some(ref tag) = container_attrs.tag {
        generate_tagged_enum_impl(
            name,
            variants,
            tag,
            container_attrs.rename_all,
            skip_serialize,
            impl_generics,
            extended_impl_generics,
            ty_generics.clone(),
            where_clause,
        )
    } else {
        panic!("Enums must have either #[compote(tag = \"...\")], #[compote(untagged)], #[compote(external_tag)], or #[compote(value_matched)] attribute")
    };

    if !container_attrs.skip_deserialize {
        let deserialize_impl =
            generate_deserialize_via_from_context_value(name, ty_generics, where_clause);
        let base: proc_macro2::TokenStream = base.into();
        TokenStream::from(quote! {
            #base
            #deserialize_impl
        })
    } else {
        base
    }
}

/// Generate FromContextValue implementation for internally tagged enums
#[allow(clippy::too_many_arguments)]
fn generate_tagged_enum_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    tag_field: &str,
    rename_all: Option<RenameAllCase>,
    skip_serialize: bool,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    // Find the fallback variant (if any)
    let fallback_variant: Option<&Variant> = variants
        .iter()
        .find(|v| parse_variant_attributes(&v.attrs).fallback);

    // Validate: only one fallback variant allowed
    let fallback_count = variants
        .iter()
        .filter(|v| parse_variant_attributes(&v.attrs).fallback)
        .count();
    if fallback_count > 1 {
        panic!("Only one variant can have #[compote(fallback)] attribute");
    }

    // Generate match arms for all variants (including fallback)
    // The fallback attribute only affects behavior when the tag field is MISSING,
    // but when the tag IS present, we still need to match it normally.
    let variant_arms: Vec<proc_macro2::TokenStream> = variants.iter()
        .map(|variant| {
        let variant_attrs = parse_variant_attributes(&variant.attrs);
        let variant_name = &variant.ident;

        // Determine the tag value(s) this variant matches
        // Priority: explicit rename > rename_all > default snake_case
        let primary_tag = variant_attrs.rename
            .clone()
            .unwrap_or_else(|| {
                let name_str = variant_name.to_string();
                match rename_all {
                    Some(case) => case.convert(&name_str),
                    None => to_snake_case(&name_str),
                }
            });

        // Build list of all tag values (primary + aliases)
        let mut all_tags = vec![primary_tag.clone()];
        all_tags.extend(variant_attrs.aliases.clone());

        // Generate the match pattern for all tags
        let tag_patterns: Vec<proc_macro2::TokenStream> = all_tags.iter().map(|t| {
            quote! { #t }
        }).collect();

        // Generate the variant construction based on fields
        let variant_construction = match &variant.fields {
            Fields::Unit => {
                quote! { Ok(#name::#variant_name) }
            }
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_deserializations = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let field_type = &field.ty;
                    let attrs = parse_field_config_attributes(&field.attrs);
                    crate::generate_field_deserialization(field_name, &field_name_str, field_type, &attrs)
                });

                quote! {
                    {
                        #(#field_deserializations)*

                        // Note: Errors are recorded but deserialization continues with defaults
                        Ok(#name::#variant_name {
                            #(#field_names: #field_names),*
                        })
                    }
                }
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Tagged enums only support tuple variants with exactly one field");
                }
                let field_type = &fields.unnamed.first().unwrap().ty;
                quote! {
                    {
                        // For tuple variants, deserialize from the entire object
                        let inner: #field_type = compote::FromContextValue::from_context_value(value, tracker)?;
                        Ok(#name::#variant_name(inner))
                    }
                }
            }
        };

        quote! {
            #(#tag_patterns)|* => #variant_construction,
        }
    }).collect();

    // Collect all valid tag values for error message
    let all_valid_tags: Vec<String> = variants
        .iter()
        .flat_map(|variant| {
            let variant_attrs = parse_variant_attributes(&variant.attrs);
            let primary = variant_attrs.rename.clone().unwrap_or_else(|| {
                let name_str = variant.ident.to_string();
                match rename_all {
                    Some(case) => case.convert(&name_str),
                    None => to_snake_case(&name_str),
                }
            });
            let mut tags = vec![primary];
            tags.extend(variant_attrs.aliases.clone());
            tags
        })
        .collect();
    let valid_tags_str = all_valid_tags.join(", ");

    // Generate Serialize impl for tagged enum (unless skip_serialize is set)
    let serialize_impl = if skip_serialize {
        quote! {}
    } else {
        generate_tagged_enum_serialize_impl(
            name,
            variants,
            tag_field,
            rename_all,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    // Generate fallback handling for when tag field is missing
    // This code is placed inside a let assignment, so it needs to use `return` to exit early
    let fallback_handling = if let Some(fallback) = fallback_variant {
        let fallback_name = &fallback.ident;
        let fallback_construction = match &fallback.fields {
            Fields::Unit => {
                quote! { return Ok(#name::#fallback_name) }
            }
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_deserializations = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let field_type = &field.ty;
                    let attrs = parse_field_config_attributes(&field.attrs);
                    crate::generate_field_deserialization(
                        field_name,
                        &field_name_str,
                        field_type,
                        &attrs,
                    )
                });

                quote! {
                    {
                        #(#field_deserializations)*
                        return Ok(#name::#fallback_name {
                            #(#field_names: #field_names),*
                        })
                    }
                }
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Tagged enums only support tuple variants with exactly one field");
                }
                let field_type = &fields.unnamed.first().unwrap().ty;
                quote! {
                    {
                        let inner: #field_type = compote::FromContextValue::from_context_value(value, tracker)?;
                        return Ok(#name::#fallback_name(inner))
                    }
                }
            }
        };
        fallback_construction
    } else {
        quote! {
            return Err(compote::Error::MissingField {
                path: format!("{}.{}", tracker.current_path(), #tag_field),
            })
        }
    };

    let expanded = quote! {
        impl #extended_impl_generics compote::FromContextValue<__CompoteS, __CompoteL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &compote::ContextValue<__CompoteS, __CompoteL>,
                tracker: &mut compote::ErrorTracker,
            ) -> Result<Self, compote::Error> {
                // Expect an object at the root
                let obj = match value {
                    compote::ContextValue::Object(obj, _) => obj,
                    _ => {
                        tracker.record_type_mismatch("object", value.type_name());
                        return Err(compote::Error::TypeMismatch {
                            path: tracker.current_path(),
                            expected: "object".to_string(),
                            actual: value.type_name().to_string(),
                        });
                    }
                };

                // Read the tag field
                let tag_value = match obj.get(#tag_field) {
                    Some(tv) => match tv {
                        compote::ContextValue::String(s, _) => s.clone(),
                        _ => {
                            tracker.record_type_mismatch("string", tv.type_name());
                            return Err(compote::Error::TypeMismatch {
                                path: format!("{}.{}", tracker.current_path(), #tag_field),
                                expected: "string".to_string(),
                                actual: tv.type_name().to_string(),
                            });
                        }
                    },
                    None => {
                        // Use fallback variant if available, otherwise error
                        #fallback_handling
                    }
                };

                // Match on the tag value
                match tag_value.as_str() {
                    #(#variant_arms)*
                    other => {
                        Err(compote::Error::InvalidValue {
                            path: format!("{}.{}", tracker.current_path(), #tag_field),
                            message: format!("Unknown tag value '{}'. Valid values are: {}", other, #valid_tags_str),
                        })
                    }
                }
            }
        }

        #serialize_impl
    };

    TokenStream::from(expanded)
}

/// Generate Serialize implementation for internally tagged enums
fn generate_tagged_enum_serialize_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    tag_field: &str,
    rename_all: Option<RenameAllCase>,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let variant_serialize_arms: Vec<proc_macro2::TokenStream> = variants.iter().map(|variant| {
        let variant_attrs = parse_variant_attributes(&variant.attrs);
        let variant_name = &variant.ident;

        // Determine the tag value (use rename if specified, otherwise rename_all, otherwise snake_case)
        let tag_value = variant_attrs.rename
            .clone()
            .unwrap_or_else(|| {
                let name_str = variant_name.to_string();
                match rename_all {
                    Some(case) => case.convert(&name_str),
                    None => to_snake_case(&name_str),
                }
            });

        match &variant.fields {
            Fields::Unit => {
                // Unit variant: serialize as { "tag": "value" }
                quote! {
                    #name::#variant_name => {
                        let mut __state = serializer.serialize_struct(stringify!(#name), 1)?;
                        __state.serialize_field(#tag_field, #tag_value)?;
                        __state.end()
                    }
                }
            }
            Fields::Named(fields) => {
                let field_count = fields.named.len() + 1; // +1 for tag field
                let field_names: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_name_strs: Vec<_> = field_names.iter()
                    .map(|f| f.to_string())
                    .collect();

                quote! {
                    #name::#variant_name { #(ref #field_names),* } => {
                        let mut __state = serializer.serialize_struct(stringify!(#name), #field_count)?;
                        __state.serialize_field(#tag_field, #tag_value)?;
                        #(__state.serialize_field(#field_name_strs, #field_names)?;)*
                        __state.end()
                    }
                }
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Tagged enums only support tuple variants with exactly one field");
                }
                // Tuple variant with one field - serialize the inner value with the tag
                quote! {
                    #name::#variant_name(ref __inner) => {
                        // For tuple variants, we need to serialize the inner struct with the tag added
                        // This is complex - for now, serialize as a map
                        use serde::ser::SerializeMap;
                        let mut __map = serializer.serialize_map(None)?;
                        __map.serialize_entry(#tag_field, #tag_value)?;
                        // Note: This doesn't serialize the inner fields. Consider using serde_json::to_value
                        // For now, just serialize the tag
                        __map.end()
                    }
                }
            }
        }
    }).collect();

    quote! {
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
            where
                __S: serde::Serializer,
            {
                use serde::ser::SerializeStruct;

                match self {
                    #(#variant_serialize_arms)*
                }
            }
        }
    }
}

/// Generate FromContextValue implementation for untagged enums
fn generate_untagged_enum_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    skip_serialize: bool,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    // Find the fallback variant (if any)
    let fallback_variant: Option<&Variant> = variants
        .iter()
        .find(|v| parse_variant_attributes(&v.attrs).fallback);

    // Validate: only one fallback variant allowed
    let fallback_count = variants
        .iter()
        .filter(|v| parse_variant_attributes(&v.attrs).fallback)
        .count();
    if fallback_count > 1 {
        panic!("Only one variant can have #[compote(fallback)] attribute");
    }

    // Helper to check if a variant has any predicate matching (variant = ...)
    let has_variant_predicates = |v: &Variant| -> bool {
        let attrs = parse_variant_attributes(&v.attrs);
        !attrs.variant_matches.is_empty() || attrs.variant_fn.is_some() || attrs.scalar_variant
    };

    // Separate variants: those with predicates go first, then type-based matching
    // Predicates use the same priority order as external_tag:
    // null > exact values > truthy/falsy > built-in predicates > custom predicates > parse extractors > wildcards

    // Phase 1: null handling (variants with variant = null)
    let null_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && is_null_variant(&attrs)
        })
        .map(|variant| {
            generate_untagged_variant_construction(name, variant, quote! { value.is_null() })
        })
        .collect();

    // Phase 2: exact value matching (Bool, String, Int, Float literals)
    let exact_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && has_exact_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let exact_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| {
                    matches!(
                        m,
                        VariantMatch::Bool(_)
                            | VariantMatch::String(_)
                            | VariantMatch::Int(_)
                            | VariantMatch::Float(_)
                    )
                })
                .map(|m| m.to_match_condition())
                .collect();
            let condition = quote! { #(#exact_conditions)||* };
            generate_untagged_variant_construction(name, variant, condition)
        })
        .collect();

    // Phase 3: predicate matching (truthy/falsy)
    let predicate_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && has_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let pred_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| matches!(m, VariantMatch::Truthy | VariantMatch::Falsy))
                .map(|m| m.to_match_condition())
                .collect();
            let condition = quote! { #(#pred_conditions)||* };
            generate_untagged_variant_construction(name, variant, condition)
        })
        .collect();

    // Phase 4: built-in predicates (starts_with, ends_with, contains, range, regex)
    let builtin_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && has_builtin_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let builtin_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| m.is_builtin_predicate())
                .map(|m| m.to_match_condition())
                .collect();
            let condition = quote! { #(#builtin_conditions)||* };
            generate_untagged_variant_construction(name, variant, condition)
        })
        .collect();

    // Phase 5: custom predicates (predicate("fn_name"))
    let custom_predicate_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && has_custom_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let custom_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| m.is_custom_predicate())
                .map(|m| m.to_match_condition())
                .collect();
            let condition = quote! { #(#custom_conditions)||* };
            generate_untagged_variant_construction(name, variant, condition)
        })
        .collect();

    // Phase 6: parse extractors (parse("fn_name"))
    let parse_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && has_parse_extractor(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;
            let parse_match = attrs
                .variant_matches
                .iter()
                .find(|m| m.is_parse())
                .expect("Variant with parse extractor must have Parse match");
            let fn_name = match parse_match {
                VariantMatch::Parse(name) => name,
                _ => unreachable!(),
            };
            let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());

            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                    quote! {
                        if let Some(__parsed_value) = #fn_ident(value) {
                            return Ok(#name::#variant_name(__parsed_value));
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    let field_count = fields.unnamed.len();
                    let field_names: Vec<syn::Ident> = (0..field_count)
                        .map(|i| {
                            syn::Ident::new(&format!("__p{}", i), proc_macro2::Span::call_site())
                        })
                        .collect();
                    let destructure_pattern = quote! { (#(#field_names),*) };
                    quote! {
                        if let Some(#destructure_pattern) = #fn_ident(value) {
                            return Ok(#name::#variant_name(#(#field_names),*));
                        }
                    }
                }
                _ => panic!(
                    "parse() extractor variant '{}' must have at least one field",
                    variant.ident
                ),
            }
        })
        .collect();

    // Phase 7: type wildcards (any_scalar, any_string, any_int, etc.) and legacy scalar_variant
    let wildcard_handlers: Vec<proc_macro2::TokenStream> = variants.iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && (has_scalar_match(&attrs) || attrs.scalar_variant)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            let type_condition = if attrs.scalar_variant {
                quote! { matches!(value, compote::ContextValue::String(_, _) | compote::ContextValue::Int(_, _) | compote::ContextValue::Float(_, _) | compote::ContextValue::Bool(_, _)) }
            } else {
                let wildcard_conditions: Vec<proc_macro2::TokenStream> = attrs.variant_matches.iter()
                    .filter(|m| matches!(m, VariantMatch::AnyScalar | VariantMatch::AnyString | VariantMatch::AnyInt | VariantMatch::AnyFloat | VariantMatch::AnyBool))
                    .map(|m| m.to_match_condition())
                    .collect();
                quote! { #(#wildcard_conditions)||* }
            };

            generate_untagged_variant_construction(name, variant, type_condition)
        })
        .collect();

    // Phase 8: Type-based matching for variants WITHOUT predicates (existing behavior)
    let type_based_handlers: Vec<proc_macro2::TokenStream> = variants.iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && !has_variant_predicates(v)
        })
        .map(|variant| {
            let variant_name = &variant.ident;

            match &variant.fields {
                Fields::Unit => {
                    // Unit variants without predicates match null
                    quote! {
                        if value.is_null() {
                            return Ok(#name::#variant_name);
                        }
                    }
                }
                Fields::Named(fields) => {
                    let field_names: Vec<_> = fields.named.iter()
                        .map(|f| f.ident.as_ref().unwrap())
                        .collect();

                    let field_deserializations = fields.named.iter().map(|field| {
                        let field_name = field.ident.as_ref().unwrap();
                        let field_name_str = field_name.to_string();
                        let field_type = &field.ty;
                        let attrs = parse_field_config_attributes(&field.attrs);
                        crate::generate_field_deserialization(field_name, &field_name_str, field_type, &attrs)
                    });

                    quote! {
                        {
                            let mut try_tracker = tracker.child();
                            if let compote::ContextValue::Object(obj, _) = value {
                                let result = {
                                    let tracker = &mut try_tracker;
                                    (|| -> Result<#name, compote::Error> {
                                        #(#field_deserializations)*

                                        Ok(#name::#variant_name {
                                            #(#field_names: #field_names),*
                                        })
                                    })()
                                };

                                if let Ok(v) = result {
                                    if !try_tracker.has_errors() {
                                        tracker.commit_child(try_tracker);
                                        return Ok(v);
                                    }
                                }
                            }
                        }
                    }
                }
                Fields::Unnamed(fields) => {
                    if fields.unnamed.len() != 1 {
                        panic!("Untagged enums only support tuple variants with exactly one field");
                    }
                    let field_type = &fields.unnamed.first().unwrap().ty;
                    let type_str = quote!(#field_type).to_string().replace(' ', "");
                    let variant_attrs = parse_variant_attributes(&variant.attrs);

                    // For allow_single variants, try wrapping scalar input in a single-element array
                    let allow_single_arm = if variant_attrs.allow_single {
                        quote! {
                            // allow_single: wrap scalar input into single-element array
                            if !value.is_array() && !value.is_null() {
                                let wrapped = compote::ContextValue::Array(
                                    vec![value.clone()],
                                    value.context().clone(),
                                );
                                let mut try_tracker = tracker.child();
                                if let Ok(inner) = <#field_type as compote::FromContextValue<__CompoteS, __CompoteL>>::from_context_value(&wrapped, &mut try_tracker) {
                                    if !try_tracker.has_errors() {
                                        tracker.commit_child(try_tracker);
                                        return Ok(#name::#variant_name(inner));
                                    }
                                }
                            }
                        }
                    } else {
                        quote! {}
                    };

                    let type_guard = if type_str == "String" {
                        quote! { matches!(value, compote::ContextValue::String(_, _)) }
                    } else if type_str == "bool" {
                        quote! { matches!(value, compote::ContextValue::Bool(_, _)) }
                    } else if type_str == "i64" || type_str == "i32" || type_str == "i16" || type_str == "i8"
                        || type_str == "u64" || type_str == "u32" || type_str == "u16" || type_str == "u8"
                        || type_str == "isize" || type_str == "usize"
                    {
                        quote! { matches!(value, compote::ContextValue::Int(_, _)) }
                    } else if type_str == "f64" || type_str == "f32" {
                        quote! { matches!(value, compote::ContextValue::Float(_, _) | compote::ContextValue::Int(_, _)) }
                    } else {
                        quote! { true }
                    };

                    quote! {
                        {
                            #allow_single_arm
                            if #type_guard {
                                let mut try_tracker = tracker.child();
                                if let Ok(inner) = <#field_type as compote::FromContextValue<__CompoteS, __CompoteL>>::from_context_value(value, &mut try_tracker) {
                                    if !try_tracker.has_errors() {
                                        tracker.commit_child(try_tracker);
                                        return Ok(#name::#variant_name(inner));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }).collect();

    let variant_names: Vec<String> = variants
        .iter()
        .filter(|v| !parse_variant_attributes(&v.attrs).fallback)
        .map(|v| v.ident.to_string())
        .collect();
    let variants_str = variant_names.join(", ");

    // Generate Serialize impl for untagged enum (unless skip_serialize is set)
    let serialize_impl = if skip_serialize {
        quote! {}
    } else {
        generate_untagged_enum_serialize_impl(
            name,
            variants,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    // Generate fallback handling for when no variant matches
    let fallback_handling = if let Some(fallback) = fallback_variant {
        let fallback_name = &fallback.ident;
        match &fallback.fields {
            Fields::Unit => {
                quote! { Ok(#name::#fallback_name) }
            }
            Fields::Named(fields) => {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_deserializations = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let field_type = &field.ty;
                    let attrs = parse_field_config_attributes(&field.attrs);
                    crate::generate_field_deserialization(
                        field_name,
                        &field_name_str,
                        field_type,
                        &attrs,
                    )
                });

                quote! {
                    {
                        // Fallback to struct variant - requires object input
                        if let compote::ContextValue::Object(obj, _) = value {
                            #(#field_deserializations)*
                            Ok(#name::#fallback_name {
                                #(#field_names: #field_names),*
                            })
                        } else {
                            Err(compote::Error::InvalidValue {
                                path: tracker.current_path(),
                                message: format!("Value does not match any variant of {}. Tried: {}", stringify!(#name), #variants_str),
                            })
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    panic!("Untagged enums only support tuple variants with exactly one field");
                }
                let field_type = &fields.unnamed.first().unwrap().ty;
                quote! {
                    {
                        // Fallback to tuple variant - deserialize inner type
                        let inner: #field_type = compote::FromContextValue::from_context_value(value, tracker)?;
                        Ok(#name::#fallback_name(inner))
                    }
                }
            }
        }
    } else {
        quote! {
            Err(compote::Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("Value does not match any variant of {}. Tried: {}", stringify!(#name), #variants_str),
            })
        }
    };

    let expanded = quote! {
        impl #extended_impl_generics compote::FromContextValue<__CompoteS, __CompoteL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &compote::ContextValue<__CompoteS, __CompoteL>,
                tracker: &mut compote::ErrorTracker,
            ) -> Result<Self, compote::Error> {
                // Phase 1: null handling (variant = null)
                #(#null_handlers)*

                // Phase 2: exact value matching
                #(#exact_handlers)*

                // Phase 3: predicate matching (truthy/falsy)
                #(#predicate_handlers)*

                // Phase 4: built-in predicates
                #(#builtin_handlers)*

                // Phase 5: custom predicates
                #(#custom_predicate_handlers)*

                // Phase 6: parse extractors
                #(#parse_handlers)*

                // Phase 7: type wildcards
                #(#wildcard_handlers)*

                // Phase 8: type-based matching (variants without predicates)
                #(#type_based_handlers)*

                // No variant matched - use fallback or error
                #fallback_handling
            }
        }

        #serialize_impl
    };

    TokenStream::from(expanded)
}

/// Helper function to generate variant construction code for untagged enums
fn generate_untagged_variant_construction(
    enum_name: &syn::Ident,
    variant: &Variant,
    condition: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let attrs = parse_variant_attributes(&variant.attrs);

    match &variant.fields {
        Fields::Unit => {
            if attrs.variant_value.is_some() {
                panic!("variant_value is not supported on unit variant '{}' (unit variants don't need a value)", variant_name);
            }
            if attrs.variant_default {
                panic!("variant_default is not supported on unit variant '{}' (unit variants don't need a default)", variant_name);
            }
            quote! {
                if #condition {
                    return Ok(#enum_name::#variant_name);
                }
            }
        }
        Fields::Named(fields) => {
            if attrs.variant_value.is_some() {
                panic!(
                    "variant_value not supported for named-field variant '{}', use variant_default",
                    variant_name
                );
            }
            if attrs.variant_default {
                quote! {
                    if #condition {
                        return Ok(#enum_name::#variant_name { ..Default::default() });
                    }
                }
            } else {
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                let field_deserializations = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = field_name.to_string();
                    let field_type = &field.ty;
                    let attrs = parse_field_config_attributes(&field.attrs);
                    crate::generate_field_deserialization(
                        field_name,
                        &field_name_str,
                        field_type,
                        &attrs,
                    )
                });

                quote! {
                    if #condition {
                        if let compote::ContextValue::Object(obj, _) = value {
                            let mut try_tracker = tracker.child();
                            let result = {
                                let tracker = &mut try_tracker;
                                (|| -> Result<#enum_name, compote::Error> {
                                    #(#field_deserializations)*
                                    Ok(#enum_name::#variant_name {
                                        #(#field_names: #field_names),*
                                    })
                                })()
                            };
                            if let Ok(variant) = result {
                                if !try_tracker.has_errors() {
                                    tracker.commit_child(try_tracker);
                                    return Ok(variant);
                                }
                            }
                        }
                    }
                }
            }
        }
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                panic!("Untagged enum tuple variants with predicates must have exactly one field");
            }
            let field_type = &fields.unnamed.first().unwrap().ty;
            if let Some(ref lit) = attrs.variant_value {
                quote! {
                    if #condition {
                        return Ok(#enum_name::#variant_name(#lit));
                    }
                }
            } else if attrs.variant_default {
                quote! {
                    if #condition {
                        return Ok(#enum_name::#variant_name(<#field_type as Default>::default()));
                    }
                }
            } else {
                quote! {
                    if #condition {
                        let mut try_tracker = tracker.child();
                        if let Ok(inner) = compote::FromContextValue::from_context_value(value, &mut try_tracker) {
                            if !try_tracker.has_errors() {
                                tracker.commit_child(try_tracker);
                                return Ok(#enum_name::#variant_name(inner));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Generate Serialize implementation for untagged enums
fn generate_untagged_enum_serialize_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let variant_serialize_arms: Vec<proc_macro2::TokenStream> = variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        match &variant.fields {
            Fields::Unit => {
                // Unit variant: serialize as null
                quote! {
                    #name::#variant_name => {
                        serializer.serialize_unit()
                    }
                }
            }
            Fields::Named(fields) => {
                let field_count = fields.named.len();
                let field_names: Vec<_> = fields.named.iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_name_strs: Vec<_> = field_names.iter()
                    .map(|f| f.to_string())
                    .collect();

                quote! {
                    #name::#variant_name { #(ref #field_names),* } => {
                        use serde::ser::SerializeStruct;
                        let mut __state = serializer.serialize_struct(stringify!(#name), #field_count)?;
                        #(__state.serialize_field(#field_name_strs, #field_names)?;)*
                        __state.end()
                    }
                }
            }
            Fields::Unnamed(_fields) => {
                // Tuple variant: serialize the inner value directly
                quote! {
                    #name::#variant_name(ref __inner) => {
                        serde::Serialize::serialize(__inner, serializer)
                    }
                }
            }
        }
    }).collect();

    quote! {
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
            where
                __S: serde::Serializer,
            {
                match self {
                    #(#variant_serialize_arms)*
                }
            }
        }
    }
}

/// Helper: check if variant matches null via variant_matches
fn has_null_match(attrs: &VariantAttributes) -> bool {
    attrs
        .variant_matches
        .iter()
        .any(|m| matches!(m, VariantMatch::Null))
}

/// Helper: check if variant has exact value matches (Bool, String, Int, Float literals)
fn has_exact_matches(attrs: &VariantAttributes) -> bool {
    attrs.variant_matches.iter().any(|m| {
        matches!(
            m,
            VariantMatch::Bool(_)
                | VariantMatch::String(_)
                | VariantMatch::Int(_)
                | VariantMatch::Float(_)
        )
    })
}

/// Helper: check if variant has predicate matches (Truthy, Falsy)
fn has_predicate_matches(attrs: &VariantAttributes) -> bool {
    attrs
        .variant_matches
        .iter()
        .any(|m| matches!(m, VariantMatch::Truthy | VariantMatch::Falsy))
}

/// Helper: check if variant matches scalars via variant_matches (type wildcards)
fn has_scalar_match(attrs: &VariantAttributes) -> bool {
    attrs.variant_matches.iter().any(|m| {
        matches!(
            m,
            VariantMatch::AnyScalar
                | VariantMatch::AnyString
                | VariantMatch::AnyInt
                | VariantMatch::AnyFloat
                | VariantMatch::AnyBool
        )
    })
}

/// Helper: check if this is a null variant (either via null_variant flag or variant = null)
fn is_null_variant(attrs: &VariantAttributes) -> bool {
    attrs.null_variant || has_null_match(attrs)
}

/// Helper: check if this is a scalar variant (either via scalar_variant flag or variant = any_scalar/any_string/etc.)
fn is_scalar_variant(attrs: &VariantAttributes) -> bool {
    attrs.scalar_variant || has_scalar_match(attrs)
}

/// Helper: check if this variant has any scalar-related matching (exact values, predicates, or type wildcards)
fn has_any_scalar_matching(attrs: &VariantAttributes) -> bool {
    has_exact_matches(attrs)
        || has_predicate_matches(attrs)
        || has_scalar_match(attrs)
        || attrs.scalar_variant
        || has_builtin_predicate_matches(attrs)
        || has_custom_predicate_matches(attrs)
        || has_parse_extractor(attrs)
}

// Helper: check if variant has built-in parameterized predicates (starts_with, ends_with, contains, range, regex)
fn has_builtin_predicate_matches(attrs: &VariantAttributes) -> bool {
    attrs
        .variant_matches
        .iter()
        .any(|m| m.is_builtin_predicate())
}

// Helper: check if variant has custom predicates (predicate("fn_name"))
fn has_custom_predicate_matches(attrs: &VariantAttributes) -> bool {
    attrs
        .variant_matches
        .iter()
        .any(|m| m.is_custom_predicate())
}

// Helper: check if variant has parse extractors (parse("fn_name"))
fn has_parse_extractor(attrs: &VariantAttributes) -> bool {
    attrs.variant_matches.iter().any(|m| m.is_parse())
}

/// Generate the construction code for a variant matched in a scalar phase (phases 2-7).
/// If variant_value is set, uses the literal directly.
/// If variant_default is set, uses Default::default().
/// Otherwise, deserializes from the value (existing behavior).
fn generate_scalar_match_construction(
    enum_name: &syn::Ident,
    variant: &Variant,
) -> proc_macro2::TokenStream {
    let attrs = parse_variant_attributes(&variant.attrs);
    let variant_name = &variant.ident;

    match &variant.fields {
        Fields::Unit => {
            if attrs.variant_value.is_some() {
                panic!("variant_value is not supported on unit variant '{}' (unit variants don't need a value)", variant_name);
            }
            if attrs.variant_default {
                panic!("variant_default is not supported on unit variant '{}' (unit variants don't need a default)", variant_name);
            }
            quote! { return Ok(#enum_name::#variant_name); }
        }
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            let field_type = &fields.unnamed.first().unwrap().ty;
            if let Some(ref lit) = attrs.variant_value {
                quote! {
                    return Ok(#enum_name::#variant_name(#lit));
                }
            } else if attrs.variant_default {
                quote! {
                    return Ok(#enum_name::#variant_name(<#field_type as Default>::default()));
                }
            } else {
                quote! {
                    let inner: #field_type = compote::FromContextValue::from_context_value(value, tracker)?;
                    return Ok(#enum_name::#variant_name(inner));
                }
            }
        }
        Fields::Unnamed(_) => {
            // Multi-field -- only parse() extractor works, no variant_value/variant_default
            if attrs.variant_value.is_some() || attrs.variant_default {
                panic!(
                    "variant_value/variant_default not supported for multi-field variant '{}'",
                    variant_name
                );
            }
            // This shouldn't normally be reached for scalar match phases
            panic!(
                "Multi-field variant '{}' cannot be constructed from scalar match",
                variant_name
            );
        }
        Fields::Named(_) => {
            if attrs.variant_default {
                quote! {
                    return Ok(#enum_name::#variant_name { ..Default::default() });
                }
            } else if attrs.variant_value.is_some() {
                panic!(
                    "variant_value not supported for named-field variant '{}', use variant_default",
                    variant_name
                );
            } else {
                // Named field variants can't normally be deserialized from scalars
                quote! {
                    return Err(compote::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: "object".to_string(),
                        actual: value.type_name().to_string(),
                    });
                }
            }
        }
    }
}

/// Generate FromContextValue implementation for externally tagged enums
/// Input format: single-key map {key: value} where key determines variant
/// Also supports null_variant (for null input) and scalar_variant (for scalar input)
/// These can be specified via either the legacy attributes or the unified variant = ... syntax
///
/// Matching order (most specific to least specific):
/// 1. null - match null input first
/// 2. exact values - variant = "help", variant = true, variant = 42, variant = 3.14
/// 3. predicates - truthy, falsy
/// 4. built-in parameterized predicates - starts_with, ends_with, contains, range, regex
/// 5. custom predicates - predicate("fn_name")
/// 6. custom extractors - parse("fn_name")
/// 7. type wildcards - any_string, any_int, any_float, any_bool, any_scalar
/// 8. map-based - standard external_tag {key: value} handling (includes variants with scalar matching)
#[allow(clippy::too_many_arguments)]
fn generate_external_tag_enum_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    rename_all: Option<RenameAllCase>,
    skip_serialize: bool,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    // Find special variants (checking both legacy flags and new unified syntax)
    let fallback_variant: Option<&Variant> = variants
        .iter()
        .find(|v| parse_variant_attributes(&v.attrs).fallback);
    let null_variant: Option<&Variant> = variants.iter().find(|v| {
        let attrs = parse_variant_attributes(&v.attrs);
        is_null_variant(&attrs)
    });

    // Validate: only one of each special variant allowed
    let fallback_count = variants
        .iter()
        .filter(|v| parse_variant_attributes(&v.attrs).fallback)
        .count();
    if fallback_count > 1 {
        panic!("Only one variant can have #[compote(fallback)] attribute");
    }
    let null_count = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            is_null_variant(&attrs)
        })
        .count();
    if null_count > 1 {
        panic!("Only one variant can have #[compote(null_variant)] or #[compote(variant = null)] attribute");
    }

    // Validate variant forms
    for variant in variants.iter() {
        let attrs = parse_variant_attributes(&variant.attrs);

        // null_variant can be unit variant or newtype
        if is_null_variant(&attrs) {
            match &variant.fields {
                Fields::Unit => {}                                         // OK for null_variant
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {} // OK for null_variant
                _ => panic!(
                    "null variant '{}' must be unit variant or newtype (single-field tuple)",
                    variant.ident
                ),
            }
        }
        // parse() extractor variants can have any number of fields (1 or more)
        // The parse function returns Option<T> for single field or Option<(T1, T2, ...)> for multiple
        // Check this BEFORE has_any_scalar_matching since parse is included in that check
        else if has_parse_extractor(&attrs) {
            match &variant.fields {
                Fields::Unnamed(fields) if !fields.unnamed.is_empty() => {} // OK - at least one field
                _ => panic!(
                    "parse() extractor variant '{}' must have at least one field",
                    variant.ident
                ),
            }
        }
        // Variants with any scalar matching (exact, predicate, or wildcard) must be unit or newtype
        else if has_any_scalar_matching(&attrs) {
            match &variant.fields {
                Fields::Unit => {} // OK for exact matches like variant = "help"
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
                _ => panic!(
                    "variant '{}' with scalar matching must be unit variant or newtype (single-field tuple)",
                    variant.ident
                ),
            }
        }
        // Regular map-based variants must be newtype
        else if !attrs.fallback {
            match &variant.fields {
                Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
                _ => panic!(
                    "external_tag enum variants must be newtype (single-field tuple), but '{}' is not",
                    variant.ident
                ),
            }
        }
    }

    // Phase 1: Generate null handling code
    let null_handling = if let Some(null_var) = null_variant {
        let null_name = &null_var.ident;
        match &null_var.fields {
            Fields::Unit => {
                quote! {
                    if value.is_null() {
                        return Ok(#name::#null_name);
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let field_type = &fields.unnamed.first().unwrap().ty;
                quote! {
                    if value.is_null() {
                        let inner: #field_type = Default::default();
                        return Ok(#name::#null_name(inner));
                    }
                }
            }
            _ => unreachable!(), // Already validated above
        }
    } else {
        quote! {}
    };

    // Phase 2: Generate exact value matching code
    // Collect variants that have exact matches (Bool, String, Int, Float literals)
    let exact_match_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_exact_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            // Get the exact match conditions
            let exact_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| {
                    matches!(
                        m,
                        VariantMatch::Bool(_)
                            | VariantMatch::String(_)
                            | VariantMatch::Int(_)
                            | VariantMatch::Float(_)
                    )
                })
                .map(|m| m.to_match_condition())
                .collect();

            let condition = quote! { #(#exact_conditions)||* };
            let construction = generate_scalar_match_construction(name, variant);

            quote! {
                if #condition {
                    #construction
                }
            }
        })
        .collect();

    let exact_handling = if exact_match_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 2: Exact value matching
            #(#exact_match_handlers)*
        }
    };

    // Phase 3: Generate predicate matching code (truthy/falsy)
    let predicate_match_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            // Get the predicate match conditions
            let predicate_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| matches!(m, VariantMatch::Truthy | VariantMatch::Falsy))
                .map(|m| m.to_match_condition())
                .collect();

            let condition = quote! { #(#predicate_conditions)||* };
            let construction = generate_scalar_match_construction(name, variant);

            quote! {
                if #condition {
                    #construction
                }
            }
        })
        .collect();

    let predicate_handling = if predicate_match_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 3: Predicate matching (truthy/falsy)
            #(#predicate_match_handlers)*
        }
    };

    // Phase 4: Generate built-in parameterized predicate matching code
    // (starts_with, ends_with, contains, range, regex)
    let builtin_predicate_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_builtin_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            // Get the builtin predicate match conditions
            let builtin_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| m.is_builtin_predicate())
                .map(|m| m.to_match_condition())
                .collect();

            let condition = quote! { #(#builtin_conditions)||* };
            let construction = generate_scalar_match_construction(name, variant);

            quote! {
                if #condition {
                    #construction
                }
            }
        })
        .collect();

    let builtin_predicate_handling = if builtin_predicate_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 4: Built-in parameterized predicates (starts_with, ends_with, contains, range, regex)
            #(#builtin_predicate_handlers)*
        }
    };

    // Phase 5: Generate custom predicate matching code (predicate("fn_name"))
    let custom_predicate_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_custom_predicate_matches(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            // Get the custom predicate match conditions
            let custom_conditions: Vec<proc_macro2::TokenStream> = attrs
                .variant_matches
                .iter()
                .filter(|m| m.is_custom_predicate())
                .map(|m| m.to_match_condition())
                .collect();

            let condition = quote! { #(#custom_conditions)||* };
            let construction = generate_scalar_match_construction(name, variant);

            quote! {
                if #condition {
                    #construction
                }
            }
        })
        .collect();

    let custom_predicate_handling = if custom_predicate_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 5: Custom predicates (predicate("fn_name"))
            #(#custom_predicate_handlers)*
        }
    };

    // Phase 6: Generate custom extractor matching code (parse("fn_name"))
    // This is different from predicates - the function returns Option<T> directly
    let parse_extractor_handlers: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_parse_extractor(&attrs)
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;

            // Get the parse extractor (should only be one per variant)
            let parse_match = attrs
                .variant_matches
                .iter()
                .find(|m| m.is_parse())
                .expect("Variant with parse extractor must have Parse match");

            let fn_name = match parse_match {
                VariantMatch::Parse(name) => name,
                _ => unreachable!(),
            };

            let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());

            // parse() variants must have at least one field
            match &variant.fields {
                Fields::Unnamed(fields) => {
                    let field_count = fields.unnamed.len();
                    if field_count == 1 {
                        // Single field (newtype) - pass value directly
                        quote! {
                            if let Some(__parsed_value) = #fn_ident(value) {
                                return Ok(#name::#variant_name(__parsed_value));
                            }
                        }
                    } else {
                        // Multiple fields - function returns Option<(T1, T2, ...)>
                        // Destructure the tuple and pass each element
                        let field_names: Vec<syn::Ident> = (0..field_count)
                            .map(|i| {
                                syn::Ident::new(
                                    &format!("__p{}", i),
                                    proc_macro2::Span::call_site(),
                                )
                            })
                            .collect();
                        let destructure_pattern = quote! { (#(#field_names),*) };
                        quote! {
                            if let Some(#destructure_pattern) = #fn_ident(value) {
                                return Ok(#name::#variant_name(#(#field_names),*));
                            }
                        }
                    }
                }
                Fields::Unit => {
                    panic!(
                        "parse() extractor variant '{}' must have at least one field",
                        variant.ident
                    );
                }
                _ => unreachable!(),
            }
        })
        .collect();

    let parse_extractor_handling = if parse_extractor_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 6: Custom extractors (parse("fn_name"))
            #(#parse_extractor_handlers)*
        }
    };

    // Phase 7: Generate type wildcard matching code (any_string, any_int, any_float, any_bool, any_scalar)
    // This also handles legacy scalar_variant flag
    let wildcard_match_handlers: Vec<proc_macro2::TokenStream> = variants.iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            has_scalar_match(&attrs) || attrs.scalar_variant
        })
        .map(|variant| {
            let attrs = parse_variant_attributes(&variant.attrs);

            // Determine which types to match based on the variant matches or legacy scalar_variant
            let type_condition = if attrs.scalar_variant {
                // Legacy scalar_variant matches all scalars
                quote! {
                    matches!(value, compote::ContextValue::String(_, _) | compote::ContextValue::Int(_, _) | compote::ContextValue::Float(_, _) | compote::ContextValue::Bool(_, _))
                }
            } else {
                // Use the specific wildcard matches
                let wildcard_conditions: Vec<proc_macro2::TokenStream> = attrs.variant_matches.iter()
                    .filter(|m| matches!(m, VariantMatch::AnyScalar | VariantMatch::AnyString | VariantMatch::AnyInt | VariantMatch::AnyFloat | VariantMatch::AnyBool))
                    .map(|m| m.to_match_condition())
                    .collect();
                quote! { #(#wildcard_conditions)||* }
            };

            let construction = generate_scalar_match_construction(name, variant);

            quote! {
                if #type_condition {
                    #construction
                }
            }
        })
        .collect();

    let wildcard_handling = if wildcard_match_handlers.is_empty() {
        quote! {}
    } else {
        quote! {
            // Phase 4: Type wildcard matching (any_string, any_int, etc.)
            #(#wildcard_match_handlers)*
        }
    };

    // Determine if we have special variants that handle non-map input
    let has_null_variant = null_variant.is_some();
    let has_any_scalar_handling = variants.iter().any(|v| {
        let attrs = parse_variant_attributes(&v.attrs);
        has_any_scalar_matching(&attrs)
    });

    // Generate match arms for known map-based variants.
    // Include all newtype variants in map dispatch (even those with scalar matching).
    // Scalar phases (1-7) handle scalars; map phase (8) handles objects - no ambiguity.
    // Exclude: fallback (handled separately), null_variant (handled separately),
    // unit variants (can't deserialize from object value),
    // parse() extractor variants (inner type may not implement FromContextValue).
    let variant_arms: Vec<proc_macro2::TokenStream> = variants.iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback && !is_null_variant(&attrs) && !has_parse_extractor(&attrs)
                && matches!(&v.fields, Fields::Unnamed(f) if f.unnamed.len() == 1)
        })
        .map(|variant| {
            let variant_attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;

            // Determine the tag value(s) this variant matches
            let primary_tag = variant_attrs.rename
                .clone()
                .unwrap_or_else(|| {
                    let name_str = variant_name.to_string();
                    match rename_all {
                        Some(case) => case.convert(&name_str),
                        None => to_snake_case(&name_str),
                    }
                });

            // Build list of all tag values (primary + aliases)
            let mut all_tags = vec![primary_tag];
            all_tags.extend(variant_attrs.aliases);

            // Generate the match pattern for all tags
            let tag_patterns: Vec<proc_macro2::TokenStream> = all_tags.iter().map(|t| {
                quote! { #t }
            }).collect();

            // Get the inner type
            let field_type = match &variant.fields {
                Fields::Unnamed(fields) => &fields.unnamed.first().unwrap().ty,
                _ => unreachable!(), // Already validated above
            };

            quote! {
                #(#tag_patterns)|* => {
                    let inner: #field_type = compote::FromContextValue::from_context_value(inner_value, tracker)?;
                    Ok(#name::#variant_name(inner))
                }
            }
        }).collect();

    // Generate fallback handling
    let fallback_handling = if let Some(fallback) = fallback_variant {
        let fallback_name = &fallback.ident;
        let fallback_type = match &fallback.fields {
            Fields::Unnamed(fields) => &fields.unnamed.first().unwrap().ty,
            _ => unreachable!(),
        };
        let fallback_attrs = parse_variant_attributes(&fallback.attrs);

        // If from_tag is set, inject the tag into the specified field after deserialization
        let from_tag_injection = if let Some(ref field_name) = fallback_attrs.from_tag {
            let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
            quote! {
                inner.#field_ident = __other_key.to_string();
            }
        } else {
            quote! {}
        };

        quote! {
            __other_key => {
                let mut inner: #fallback_type = compote::FromContextValue::from_context_value(
                    inner_value,
                    tracker,
                )?;
                #from_tag_injection
                Ok(#name::#fallback_name(inner))
            }
        }
    } else {
        // Collect valid tags for error message (all newtype variants participating in map dispatch)
        let all_valid_tags: Vec<String> = variants
            .iter()
            .filter(|v| {
                let attrs = parse_variant_attributes(&v.attrs);
                !attrs.fallback
                    && !is_null_variant(&attrs)
                    && !has_parse_extractor(&attrs)
                    && matches!(&v.fields, Fields::Unnamed(f) if f.unnamed.len() == 1)
            })
            .flat_map(|variant| {
                let variant_attrs = parse_variant_attributes(&variant.attrs);
                let primary = variant_attrs.rename.clone().unwrap_or_else(|| {
                    let name_str = variant.ident.to_string();
                    match rename_all {
                        Some(case) => case.convert(&name_str),
                        None => to_snake_case(&name_str),
                    }
                });
                let mut tags = vec![primary];
                tags.extend(variant_attrs.aliases);
                tags
            })
            .collect();
        let valid_tags_str = all_valid_tags.join(", ");
        let enum_name_str = name.to_string();

        quote! {
            __other_key => {
                Err(compote::Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("unknown variant '{}' for enum {}. Valid variants are: {}", __other_key, #enum_name_str, #valid_tags_str),
                })
            }
        }
    };

    // Build the error message for non-map types based on what's supported
    let type_mismatch_error = if has_null_variant && has_any_scalar_handling {
        quote! {
            return Err(compote::Error::TypeMismatch {
                path: tracker.current_path(),
                expected: "object, scalar, or null".to_string(),
                actual: value.type_name().to_string(),
            });
        }
    } else if has_null_variant {
        quote! {
            return Err(compote::Error::TypeMismatch {
                path: tracker.current_path(),
                expected: "object or null".to_string(),
                actual: value.type_name().to_string(),
            });
        }
    } else if has_any_scalar_handling {
        quote! {
            return Err(compote::Error::TypeMismatch {
                path: tracker.current_path(),
                expected: "object or scalar".to_string(),
                actual: value.type_name().to_string(),
            });
        }
    } else {
        quote! {
            tracker.record_type_mismatch("object", value.type_name());
            return Err(compote::Error::TypeMismatch {
                path: tracker.current_path(),
                expected: "object".to_string(),
                actual: value.type_name().to_string(),
            });
        }
    };

    // Generate Serialize impl for external_tag enum (unless skip_serialize is set)
    let serialize_impl = if skip_serialize {
        quote! {}
    } else {
        generate_external_tag_enum_serialize_impl(
            name,
            variants,
            rename_all,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    let expanded = quote! {
        impl #extended_impl_generics compote::FromContextValue<__CompoteS, __CompoteL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &compote::ContextValue<__CompoteS, __CompoteL>,
                tracker: &mut compote::ErrorTracker,
            ) -> Result<Self, compote::Error> {
                // Phase 1: Handle null_variant first (if defined)
                #null_handling

                // Phase 2: Handle exact value matching (variant = "help", variant = true, etc.)
                #exact_handling

                // Phase 3: Handle predicate matching (truthy/falsy)
                #predicate_handling

                // Phase 4: Handle built-in parameterized predicates (starts_with, ends_with, etc.)
                #builtin_predicate_handling

                // Phase 5: Handle custom predicates (predicate("fn_name"))
                #custom_predicate_handling

                // Phase 6: Handle custom extractors (parse("fn_name"))
                #parse_extractor_handling

                // Phase 7: Handle type wildcard matching (any_string, any_int, etc.)
                #wildcard_handling

                // Phase 8: Expect a single-key map for standard external_tag handling
                let obj = match value {
                    compote::ContextValue::Object(obj, _) => obj,
                    _ => {
                        #type_mismatch_error
                    }
                };

                if obj.len() != 1 {
                    return Err(compote::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("value should be a table with a single key-value pair but found {} keys", obj.len()),
                    });
                }

                let (key, inner_value) = obj.iter().next().unwrap();

                match key.as_str() {
                    #(#variant_arms)*
                    #fallback_handling
                }
            }
        }

        #serialize_impl
    };

    TokenStream::from(expanded)
}

/// Generate Serialize implementation for externally tagged enums
/// Handles null_variant (serialize as null), scalar_variant (serialize inner value directly),
/// exact value matches (serialize the first match value), predicates (serialize as true/false),
/// and standard variants (serialize as {key: value} map)
/// These can be specified via either the legacy attributes or the unified variant = ... syntax
fn generate_external_tag_enum_serialize_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    rename_all: Option<RenameAllCase>,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let variant_serialize_arms: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|variant| {
            let variant_attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;

            // Determine the tag value
            let tag_value = variant_attrs.rename.clone().unwrap_or_else(|| {
                let name_str = variant_name.to_string();
                match rename_all {
                    Some(case) => case.convert(&name_str),
                    None => to_snake_case(&name_str),
                }
            });

            // Handle null_variant - serialize as null (via legacy flag or variant = null)
            if is_null_variant(&variant_attrs) {
                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                serializer.serialize_none()
                            }
                        }
                    }
                    Fields::Unnamed(_) => {
                        quote! {
                            #name::#variant_name(ref __inner) => {
                                // null_variant with inner value still serializes as null
                                let _ = __inner; // Silence unused warning
                                serializer.serialize_none()
                            }
                        }
                    }
                    _ => unreachable!(), // Already validated
                }
            }
            // Handle exact value matches - serialize the first match value
            else if has_exact_matches(&variant_attrs) {
                // Get the first exact match value for serialization
                let first_exact = variant_attrs
                    .variant_matches
                    .iter()
                    .find(|m| {
                        matches!(
                            m,
                            VariantMatch::Bool(_)
                                | VariantMatch::String(_)
                                | VariantMatch::Int(_)
                                | VariantMatch::Float(_)
                        )
                    })
                    .map(|m| m.to_serialize_value())
                    .unwrap();

                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                serde::Serialize::serialize(&#first_exact, serializer)
                            }
                        }
                    }
                    Fields::Unnamed(_) => {
                        quote! {
                            #name::#variant_name(ref __inner) => {
                                // Ignore inner value, serialize the matched value
                                let _ = __inner;
                                serde::Serialize::serialize(&#first_exact, serializer)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // Handle predicate matches (truthy/falsy) - serialize as true/false
            else if has_predicate_matches(&variant_attrs) {
                // Check which predicate(s) this variant has
                let has_truthy = variant_attrs
                    .variant_matches
                    .iter()
                    .any(|m| matches!(m, VariantMatch::Truthy));
                let serialize_value = if has_truthy {
                    quote! { true }
                } else {
                    quote! { false }
                };

                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                serde::Serialize::serialize(&#serialize_value, serializer)
                            }
                        }
                    }
                    Fields::Unnamed(_) => {
                        quote! {
                            #name::#variant_name(ref __inner) => {
                                // Ignore inner value, serialize as bool
                                let _ = __inner;
                                serde::Serialize::serialize(&#serialize_value, serializer)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // Handle scalar_variant / type wildcards - serialize inner value directly
            // (via legacy flag or variant = any_scalar/any_string/etc.)
            else if is_scalar_variant(&variant_attrs) {
                quote! {
                    #name::#variant_name(ref __inner) => {
                        serde::Serialize::serialize(__inner, serializer)
                    }
                }
            }
            // Handle built-in parameterized predicates (starts_with, ends_with, contains, range, regex)
            // Serialize inner value directly (if newtype) or use map serialization (if unit)
            else if has_builtin_predicate_matches(&variant_attrs) {
                match &variant.fields {
                    Fields::Unit => {
                        // Unit variant with builtin predicate - serialize as map with variant tag
                        quote! {
                            #name::#variant_name => {
                                use serde::ser::SerializeMap;
                                let mut __map = serializer.serialize_map(Some(1))?;
                                __map.serialize_entry(#tag_value, &())?;
                                __map.end()
                            }
                        }
                    }
                    Fields::Unnamed(_) => {
                        // Newtype variant - serialize inner value directly
                        quote! {
                            #name::#variant_name(ref __inner) => {
                                serde::Serialize::serialize(__inner, serializer)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // Handle custom predicates (predicate("fn_name")) - same as builtin predicates
            else if has_custom_predicate_matches(&variant_attrs) {
                match &variant.fields {
                    Fields::Unit => {
                        quote! {
                            #name::#variant_name => {
                                use serde::ser::SerializeMap;
                                let mut __map = serializer.serialize_map(Some(1))?;
                                __map.serialize_entry(#tag_value, &())?;
                                __map.end()
                            }
                        }
                    }
                    Fields::Unnamed(_) => {
                        quote! {
                            #name::#variant_name(ref __inner) => {
                                serde::Serialize::serialize(__inner, serializer)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // Handle parse extractors (parse("fn_name")) - serialize inner value(s) directly
            else if has_parse_extractor(&variant_attrs) {
                // parse() variants can have one or more fields
                match &variant.fields {
                    Fields::Unnamed(fields) => {
                        let field_count = fields.unnamed.len();
                        if field_count == 1 {
                            // Single field - serialize directly
                            quote! {
                                #name::#variant_name(ref __inner) => {
                                    serde::Serialize::serialize(__inner, serializer)
                                }
                            }
                        } else {
                            // Multiple fields - serialize as tuple
                            let field_names: Vec<syn::Ident> = (0..field_count)
                                .map(|i| {
                                    syn::Ident::new(
                                        &format!("__f{}", i),
                                        proc_macro2::Span::call_site(),
                                    )
                                })
                                .collect();
                            let pattern = quote! { (#(ref #field_names),*) };
                            let tuple_ser = quote! { (#(#field_names),*) };
                            quote! {
                                #name::#variant_name #pattern => {
                                    serde::Serialize::serialize(&#tuple_ser, serializer)
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            // Handle fallback variants - use from_tag field as key if specified
            else if variant_attrs.fallback {
                let tag_expr = if let Some(ref field_name) = variant_attrs.from_tag {
                    let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
                    quote! { __inner.#field_ident.clone() }
                } else {
                    quote! { #tag_value.to_string() }
                };
                quote! {
                    #name::#variant_name(ref __inner) => {
                        use serde::ser::SerializeMap;
                        let __tag: String = #tag_expr;
                        let mut __map = serializer.serialize_map(Some(1))?;
                        __map.serialize_entry(&__tag, __inner)?;
                        __map.end()
                    }
                }
            }
            // Standard variants - serialize as {key: value} map
            else {
                quote! {
                    #name::#variant_name(ref __inner) => {
                        use serde::ser::SerializeMap;
                        let mut __map = serializer.serialize_map(Some(1))?;
                        __map.serialize_entry(#tag_value, __inner)?;
                        __map.end()
                    }
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
                match self {
                    #(#variant_serialize_arms)*
                }
            }
        }
    }
}

/// Generate serde::Deserialize implementation by delegating to FromContextValue.
///
/// This approach deserializes the input into `compote::Value` first, then converts
/// to `ContextValue` with a default context, and finally delegates to `FromContextValue`.
/// This ensures all compote features (defaults, transforms, flatten, scalar_as, etc.)
/// work identically in both config loading and serde deserialization paths.
pub(crate) fn generate_deserialize_via_from_context_value(
    name: &syn::Ident,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    quote! {
        impl<'__compote_de> serde::Deserialize<'__compote_de> for #name #ty_generics #where_clause {
            fn deserialize<__D>(__deserializer: __D) -> Result<Self, __D::Error>
            where
                __D: serde::Deserializer<'__compote_de>,
            {
                let __value = <compote::Value as serde::Deserialize>::deserialize(__deserializer)?;
                let __ctx_value = compote::ContextValue::<compote::Source, compote::Level>::from(__value);
                let mut __tracker = compote::ErrorTracker::new();
                <Self as compote::FromContextValue<compote::Source, compote::Level>>::from_context_value(
                    &__ctx_value,
                    &mut __tracker,
                ).map_err(serde::de::Error::custom)
            }
        }
    }
}

/// Generate FromContextValue implementation for value-matched enums.
/// These enums are parsed from raw scalar values (bool, string, int) rather than maps.
/// Each variant specifies which values it matches via `#[compote(variant = ...)]`.
///
/// Example:
/// ```text
/// #[derive(compote::Config)]
/// #[compote(value_matched)]
/// pub enum SelfUpdate {
///     #[compote(variant = true | "true" | "yes" | 1)]
///     True,
///     #[compote(variant = false | "false" | "no" | 0)]
///     False,
///     #[compote(variant = "nocheck")]
///     NoCheck,
///     #[compote(fallback)]
///     Ask,
/// }
/// ```
fn generate_value_matched_enum_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    skip_serialize: bool,
    impl_generics: syn::ImplGenerics,
    extended_impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> TokenStream {
    // Find the fallback variant (if any)
    let fallback_variant: Option<&Variant> = variants
        .iter()
        .find(|v| parse_variant_attributes(&v.attrs).fallback);

    // Validate: only one fallback variant allowed
    let fallback_count = variants
        .iter()
        .filter(|v| parse_variant_attributes(&v.attrs).fallback)
        .count();
    if fallback_count > 1 {
        panic!("Only one variant can have #[compote(fallback)] attribute");
    }

    // Validate variant forms - value_matched variants must be unit variants
    for variant in variants.iter() {
        match &variant.fields {
            Fields::Unit => {} // OK
            _ => panic!(
                "value_matched enum variants must be unit variants (no fields), but '{}' is not",
                variant.ident
            ),
        }
    }

    // Collect all variant match conditions (excluding fallback)
    let variant_match_arms: Vec<proc_macro2::TokenStream> = variants.iter()
        .filter(|v| {
            let attrs = parse_variant_attributes(&v.attrs);
            !attrs.fallback
        })
        .map(|variant| {
            let variant_attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;

            // Check if this variant has any match conditions
            if variant_attrs.variant_matches.is_empty() && variant_attrs.variant_fn.is_none() {
                panic!(
                    "value_matched enum variant '{}' must have #[compote(variant = ...)] or #[compote(variant_fn = ...)] attribute",
                    variant_name
                );
            }

            // Generate the match condition by OR-ing all match values
            let match_conditions: Vec<proc_macro2::TokenStream> = variant_attrs.variant_matches.iter()
                .map(|m| m.to_match_condition())
                .collect();

            // Add variant_fn condition if present
            let variant_fn_condition = if let Some(ref fn_name) = variant_attrs.variant_fn {
                let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                Some(quote! { #fn_ident(value) })
            } else {
                None
            };

            // Combine all conditions with OR
            let combined_condition = if match_conditions.is_empty() {
                // Only variant_fn
                variant_fn_condition.expect("Must have either variant matches or variant_fn")
            } else if let Some(fn_cond) = variant_fn_condition {
                // Both variant matches and variant_fn
                quote! { (#(#match_conditions)||*) || #fn_cond }
            } else {
                // Only variant matches
                quote! { #(#match_conditions)||* }
            };

            quote! {
                if #combined_condition {
                    return Ok(#name::#variant_name);
                }
            }
        }).collect();

    // Generate fallback handling
    let fallback_handling = if let Some(fallback) = fallback_variant {
        let fallback_name = &fallback.ident;
        quote! {
            // Fallback variant accepts any unmatched value
            return Ok(#name::#fallback_name);
        }
    } else {
        // No fallback - generate error for unmatched values
        let enum_name_str = name.to_string();
        quote! {
            return Err(compote::Error::InvalidValue {
                path: tracker.current_path(),
                message: format!("no variant of {} matches the value", #enum_name_str),
            });
        }
    };

    // Generate Serialize impl (unless skip_serialize is set)
    let serialize_impl = if skip_serialize {
        quote! {}
    } else {
        generate_value_matched_enum_serialize_impl(
            name,
            variants,
            impl_generics.clone(),
            ty_generics.clone(),
            where_clause,
        )
    };

    let expanded = quote! {
        impl #extended_impl_generics compote::FromContextValue<__CompoteS, __CompoteL> for #name #ty_generics #where_clause {
            fn from_context_value(
                value: &compote::ContextValue<__CompoteS, __CompoteL>,
                tracker: &mut compote::ErrorTracker,
            ) -> Result<Self, compote::Error> {
                // Check each variant's match conditions in order
                #(#variant_match_arms)*

                // No variant matched - use fallback or error
                #fallback_handling
            }
        }

        #serialize_impl
    };

    TokenStream::from(expanded)
}

/// Generate Serialize implementation for value-matched enums.
/// Each variant serializes using its first match value (or a sensible default for truthy/falsy).
fn generate_value_matched_enum_serialize_impl(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::Token![,]>,
    impl_generics: syn::ImplGenerics,
    ty_generics: syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> proc_macro2::TokenStream {
    let variant_serialize_arms: Vec<proc_macro2::TokenStream> = variants
        .iter()
        .map(|variant| {
            let variant_attrs = parse_variant_attributes(&variant.attrs);
            let variant_name = &variant.ident;

            // Determine what value to serialize for this variant
            let serialize_value = if !variant_attrs.variant_matches.is_empty() {
                // Use the first match value for serialization
                variant_attrs.variant_matches[0].to_serialize_value()
            } else if variant_attrs.fallback {
                // For fallback, serialize the variant name in snake_case as a string
                let tag_value = to_snake_case(&variant_name.to_string());
                quote! { #tag_value }
            } else {
                // Should not happen - validate earlier
                panic!(
                    "variant '{}' has no match conditions and is not fallback",
                    variant_name
                );
            };

            quote! {
                #name::#variant_name => serializer.serialize_str(&format!("{}", #serialize_value)),
            }
        })
        .collect();

    quote! {
        impl #impl_generics serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<__S>(&self, serializer: __S) -> Result<__S::Ok, __S::Error>
            where
                __S: serde::Serializer,
            {
                match self {
                    #(#variant_serialize_arms)*
                }
            }
        }
    }
}

impl VariantMatch {
    /// Generate a token stream that evaluates to true if the match applies
    fn to_match_condition(&self) -> proc_macro2::TokenStream {
        match self {
            VariantMatch::Bool(b) => {
                quote! { matches!(value, compote::ContextValue::Bool(v, _) if *v == #b) }
            }
            VariantMatch::String(s) => {
                quote! { matches!(value, compote::ContextValue::String(v, _) if v == #s) }
            }
            VariantMatch::Int(i) => {
                quote! { matches!(value, compote::ContextValue::Int(v, _) if *v == #i) }
            }
            VariantMatch::Float(f) => {
                quote! { matches!(value, compote::ContextValue::Float(v, _) if (*v - #f).abs() < f64::EPSILON) }
            }
            VariantMatch::Truthy => {
                quote! {
                    match value {
                        compote::ContextValue::Bool(true, _) => true,
                        compote::ContextValue::String(s, _) => matches!(s.to_lowercase().as_str(), "true" | "yes" | "y" | "on" | "1"),
                        compote::ContextValue::Int(i, _) => *i != 0,
                        _ => false,
                    }
                }
            }
            VariantMatch::Falsy => {
                quote! {
                    match value {
                        compote::ContextValue::Bool(false, _) => true,
                        compote::ContextValue::String(s, _) => matches!(s.to_lowercase().as_str(), "false" | "no" | "n" | "off" | "0"),
                        compote::ContextValue::Int(0, _) => true,
                        _ => false,
                    }
                }
            }
            VariantMatch::Null => {
                quote! { matches!(value, compote::ContextValue::Null(_)) }
            }
            VariantMatch::AnyString => {
                quote! { matches!(value, compote::ContextValue::String(_, _)) }
            }
            VariantMatch::AnyInt => {
                quote! { matches!(value, compote::ContextValue::Int(_, _)) }
            }
            VariantMatch::AnyFloat => {
                quote! { matches!(value, compote::ContextValue::Float(_, _)) }
            }
            VariantMatch::AnyBool => {
                quote! { matches!(value, compote::ContextValue::Bool(_, _)) }
            }
            VariantMatch::AnyScalar => {
                quote! { matches!(value, compote::ContextValue::String(_, _) | compote::ContextValue::Int(_, _) | compote::ContextValue::Float(_, _) | compote::ContextValue::Bool(_, _)) }
            }
            VariantMatch::StartsWith(prefix) => {
                quote! {
                    matches!(value, compote::ContextValue::String(s, _) if s.starts_with(#prefix))
                }
            }
            VariantMatch::EndsWith(suffix) => {
                quote! {
                    matches!(value, compote::ContextValue::String(s, _) if s.ends_with(#suffix))
                }
            }
            VariantMatch::Contains(substring) => {
                quote! {
                    matches!(value, compote::ContextValue::String(s, _) if s.contains(#substring))
                }
            }
            VariantMatch::Range(min, max) => {
                // Try to parse as float (which covers both int and float cases)
                let min_f: f64 = min.parse().unwrap_or(f64::MIN);
                let max_f: f64 = max.parse().unwrap_or(f64::MAX);
                quote! {
                    match value {
                        compote::ContextValue::Int(i, _) => {
                            let f = *i as f64;
                            (#min_f..=#max_f).contains(&f)
                        }
                        compote::ContextValue::Float(f, _) => (#min_f..=#max_f).contains(f),
                        _ => false,
                    }
                }
            }
            #[cfg(feature = "regex")]
            VariantMatch::Regex(pattern) => {
                quote! {
                    matches!(value, compote::ContextValue::String(s, _) if {
                        // Compile regex at runtime - cached by user if needed
                        compote::regex::Regex::new(#pattern).map(|re| re.is_match(s)).unwrap_or(false)
                    })
                }
            }
            VariantMatch::Predicate(fn_name) => {
                let fn_ident = syn::Ident::new(fn_name, proc_macro2::Span::call_site());
                quote! { #fn_ident(value) }
            }
            VariantMatch::Parse(_) => {
                // Parse is special - it returns Option<T>, not bool
                // This method shouldn't be called for Parse variants
                // The match condition is handled separately in generate_external_tag_enum_impl
                quote! { false }
            }
        }
    }
    /// Returns the first match value for serialization purposes
    fn to_serialize_value(&self) -> proc_macro2::TokenStream {
        match self {
            VariantMatch::Bool(b) => quote! { #b },
            VariantMatch::String(s) => quote! { #s },
            VariantMatch::Int(i) => quote! { #i },
            VariantMatch::Float(f) => quote! { #f },
            VariantMatch::Truthy => quote! { true },
            VariantMatch::Falsy => quote! { false },
            // These predicates don't have a specific serialization value
            // They are used for matching and the actual value is serialized from the inner type
            VariantMatch::Null => quote! { () },
            VariantMatch::AnyString => quote! { "" },
            VariantMatch::AnyInt => quote! { 0i64 },
            VariantMatch::AnyFloat => quote! { 0.0f64 },
            VariantMatch::AnyBool => quote! { false },
            VariantMatch::AnyScalar => quote! { "" },
            // Built-in predicates and custom predicates/extractors serialize the inner value
            VariantMatch::StartsWith(_) => quote! { "" },
            VariantMatch::EndsWith(_) => quote! { "" },
            VariantMatch::Contains(_) => quote! { "" },
            VariantMatch::Range(_, _) => quote! { 0i64 },
            #[cfg(feature = "regex")]
            VariantMatch::Regex(_) => quote! { "" },
            VariantMatch::Predicate(_) => quote! { "" },
            VariantMatch::Parse(_) => quote! { () },
        }
    }
}

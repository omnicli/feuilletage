//! Codegen for `Vec<T>` field deserialization, including the
//! `allow_single` / `allow_map` / `transform_each` / `order_by` paths.
//! All of these end up dispatched through the unified Vec codegen here.

use quote::quote;
use syn::Type;

use crate::attrs::{DefaultValue, FieldConfigAttributes, OnErrorMode};
use crate::helpers::{get_inner_type, parse_transform_path};

pub(crate) fn generate_vec_deserialization(
    field_name: &syn::Ident,
    field_name_str: &str,
    field_type: &Type,
    attrs: &FieldConfigAttributes,
) -> proc_macro2::TokenStream {
    // Generate field lookup with aliases support
    let field_lookup = crate::generate_field_lookup(field_name_str, &attrs.aliases);

    // Extract element type from Vec<T> for explicit type annotation when sorting
    let element_type = get_inner_type(field_type);

    // Generate default value tokens if present
    let default_tokens = attrs.default_value.as_ref().map(|dv| match dv {
        DefaultValue::Explicit(expr) => expr.parse::<proc_macro2::TokenStream>().unwrap(),
        DefaultValue::UseDefault => quote! { Default::default() },
        DefaultValue::Function(fn_name) => {
            let fn_ident: proc_macro2::TokenStream = fn_name.parse().unwrap();
            quote! { #fn_ident() }
        }
    });

    // Generate missing field handling
    // Use typed default so this works for any collection type
    // (Vec, BTreeSet, HashSet, etc.)
    let missing_field_handling = if let Some(ref default_expr) = &default_tokens {
        quote! { #default_expr }
    } else {
        quote! { <#field_type as Default>::default() }
    };

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
                    let error = compote::Error::InvalidValue {
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

    // Generate normalization to array (allow_single/allow_map conversion)
    // allow_single in flag form (empty string) wraps scalar values in array
    let has_allow_single = attrs.allow_single.as_ref().is_some_and(|s| s.is_empty());
    let normalization_code = if let Some(ref allow_map_config) = attrs.allow_map {
        let key_field = &allow_map_config.key_field;

        let map_to_array_body = if let Some(ref scalar_as_field) = allow_map_config.scalar_as_field
        {
            // scalar_as_field specified - create object with key and scalar_as fields for scalar values
            quote! {
                was_from_map = true;
                let mut items = Vec::new();
                for (map_key, item) in map.iter() {
                    let augmented_item = match item {
                        compote::ContextValue::Object(inner_obj, _) => {
                            // Already an object - inject key field
                            let mut new_obj = inner_obj.clone();
                            new_obj.insert(
                                #key_field.to_string(),
                                compote::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            compote::ContextValue::object(new_obj, item.context().clone())
                        }
                        _ => {
                            // Scalar value - create object with key and scalar_as fields
                            let mut new_obj = __CompoteIndexMap::default();
                            new_obj.insert(
                                #key_field.to_string(),
                                compote::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            new_obj.insert(
                                #scalar_as_field.to_string(),
                                item.clone(),
                            );
                            compote::ContextValue::object(new_obj, item.context().clone())
                        }
                    };
                    items.push(augmented_item);
                }
                compote::ContextValue::array(items, field_value.context().clone())
            }
        } else {
            // Only key_field specified - inject key into objects only
            quote! {
                was_from_map = true;
                let mut items = Vec::new();
                for (map_key, item) in map.iter() {
                    let augmented_item = match item {
                        compote::ContextValue::Object(inner_obj, _) => {
                            // Clone the object and inject the key
                            let mut new_obj = inner_obj.clone();
                            new_obj.insert(
                                #key_field.to_string(),
                                compote::ContextValue::string(map_key.clone(), item.context().clone()),
                            );
                            compote::ContextValue::object(new_obj, item.context().clone())
                        }
                        _ => {
                            // Not an object - cannot inject key, skip with error
                            tracker.record(compote::Error::TypeMismatch {
                                path: tracker.current_path(),
                                expected: "object".to_string(),
                                actual: item.type_name().to_string(),
                            });
                            continue;
                        }
                    };
                    items.push(augmented_item);
                }
                compote::ContextValue::array(items, field_value.context().clone())
            }
        };

        // Wrap with single-item detection using AllowMapKeys trait
        let map_to_array = quote! {
            compote::ContextValue::Object(map, _) => {
                // Use AllowMapKeys trait to detect if this object is a single item
                let key_fields = <#element_type as compote::AllowMapKeys>::map_key_fields();
                let is_single_item = key_fields.iter().any(|k| map.contains_key(*k));

                if is_single_item {
                    // Object contains the key field — treat as a single item
                    compote::ContextValue::array(vec![field_value.clone()], field_value.context().clone())
                } else {
                    // Map notation — convert to array with key injection
                    #map_to_array_body
                }
            }
        };

        let single_value_arm = if has_allow_single {
            quote! {
                _ => {
                    // Single value - wrap in array (allow_single flag behavior)
                    compote::ContextValue::array(vec![field_value.clone()], field_value.context().clone())
                }
            }
        } else {
            quote! {
                _ => {
                    let error = compote::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: "array or object".to_string(),
                        actual: field_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                }
            }
        };

        quote! {
            #[allow(unused_mut, unused_variables)]
            let mut was_from_map = false;
            let array_value: compote::ContextValue<__CompoteS, __CompoteL> = match field_value {
                compote::ContextValue::Array(_, _) => field_value.clone(),
                #map_to_array
                #single_value_arm
            };
        }
    } else if attrs.allow_map_flag {
        // allow_map flag form: use inner type's AllowMapKeys trait for detection
        // Split map into single-key maps UNLESS any key matches a known field name
        let single_value_arm = if has_allow_single {
            quote! {
                _ => {
                    // Single value - wrap in array (allow_single flag behavior)
                    compote::ContextValue::array(vec![field_value.clone()], field_value.context().clone())
                }
            }
        } else {
            quote! {
                _ => {
                    let error = compote::Error::TypeMismatch {
                        path: tracker.current_path(),
                        expected: "array or object".to_string(),
                        actual: field_value.type_name().to_string(),
                    };
                    tracker.record(error.clone());
                    tracker.pop();
                    return Err(error);
                }
            }
        };

        quote! {
            #[allow(unused_mut, unused_variables)]
            let mut was_from_map = false;
            let array_value: compote::ContextValue<__CompoteS, __CompoteL> = match field_value {
                compote::ContextValue::Array(_, _) => field_value.clone(),
                compote::ContextValue::Object(map, _) => {
                    // Use AllowMapKeys trait for detection
                    let key_fields = <#element_type as compote::AllowMapKeys>::map_key_fields();
                    let is_single_item = key_fields.iter().any(|k| map.contains_key(*k));

                    if is_single_item {
                        // Map has a key matching a field name - treat as single item
                        compote::ContextValue::array(vec![field_value.clone()], field_value.context().clone())
                    } else {
                        // No key matches - split into single-key maps
                        was_from_map = true;
                        let items: Vec<compote::ContextValue<__CompoteS, __CompoteL>> = map.iter()
                            .map(|(k, v)| {
                                let mut single_map = __CompoteIndexMap::default();
                                single_map.insert(k.clone(), v.clone());
                                compote::ContextValue::object(single_map, v.context().clone())
                            })
                            .collect();
                        compote::ContextValue::array(items, field_value.context().clone())
                    }
                }
                #single_value_arm
            };
        }
    } else if has_allow_single {
        // allow_single flag case: single value or array
        quote! {
            #[allow(unused_mut, unused_variables)]
            let mut was_from_map = false;
            let array_value: compote::ContextValue<__CompoteS, __CompoteL> = match field_value {
                compote::ContextValue::Array(_, _) => field_value.clone(),
                _ => {
                    // Single value - wrap in array
                    compote::ContextValue::array(vec![field_value.clone()], field_value.context().clone())
                }
            };
        }
    } else {
        // Should not reach here (function is only called for allow_single or allow_map)
        quote! {
            #[allow(unused_mut, unused_variables)]
            let mut was_from_map = false;
            let array_value: compote::ContextValue<__CompoteS, __CompoteL> = field_value.clone();
        }
    };

    // Generate transform code for entire array (if present)
    let array_transform_code = if let Some(ref t) = attrs.transform {
        let transform_ident: proc_macro2::TokenStream = parse_transform_path(t);
        quote! {
            let array_value = {
                let mut temp = array_value;
                let ctx = temp.context().clone();
                if let Err(e) = #transform_ident(&mut temp, &ctx) {
                    tracker.record(e);
                    tracker.pop();
                    return Err(compote::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Transform failed for field '{}'", #field_name_str),
                    });
                }
                temp
            };
        }
    } else {
        quote! {}
    };

    // Determine if sorting will be applied (affects mutability and type annotation of result)
    let has_sorting = attrs
        .allow_map
        .as_ref()
        .is_some_and(|c| c.order_by.is_some() || c.order_by_fn.is_some())
        || attrs.order_by.is_some()
        || attrs.order_by_fn.is_some();

    // Get on_error mode (default to Default which skips invalid items)
    let on_error = attrs.on_error.unwrap_or_default();

    // Generate error handling code based on on_error mode
    let (error_init, error_handle_transform, error_handle_deser, post_loop_check) = match on_error {
        OnErrorMode::Fail => (
            quote! {},
            // Transform failure: return immediately
            quote! {
                tracker.pop();
                return Err(e);
            },
            // Deserialization failure: return immediately
            quote! {
                tracker.pop();
                return Err(e);
            },
            quote! {},
        ),
        OnErrorMode::Default => (
            // Track if any error occurred
            quote! { let mut __vec_had_error = false; },
            // Transform failure: record and mark error, continue
            quote! {
                tracker.record(e);
                __vec_had_error = true;
                tracker.pop();
                continue;
            },
            // Deserialization failure: record and mark error
            quote! {
                tracker.record(e.clone());
                __vec_had_error = true;
            },
            // After loop: if any error, use default
            {
                let fallback = if let Some(ref default_expr) = &default_tokens {
                    quote! { #default_expr }
                } else {
                    quote! { Vec::new() }
                };
                quote! {
                    if __vec_had_error {
                        result = #fallback;
                    }
                }
            },
        ),
        OnErrorMode::Skip => (
            quote! {},
            // Transform failure: record and skip (current behavior)
            quote! {
                tracker.record(e);
                tracker.pop();
                continue;
            },
            // Deserialization failure: record and skip (current behavior)
            quote! {
                tracker.record(e.clone());
            },
            quote! {},
        ),
    };

    // Generate transform_each_after code (post-deserialization per-element transform)
    let transform_each_after_code = if let Some(ref transform_fn) = attrs.transform_each_after {
        let transform_fn_ident: proc_macro2::TokenStream = transform_fn.parse().unwrap();
        let error_handling = match on_error {
            OnErrorMode::Fail => quote! {
                tracker.pop();
                return Err(e);
            },
            OnErrorMode::Default => quote! {
                tracker.record(e);
                __vec_had_error = true;
                tracker.pop();
                continue;
            },
            OnErrorMode::Skip => quote! {
                tracker.record(e);
                tracker.pop();
                continue;
            },
        };
        Some(quote! {
            if let Err(e) = #transform_fn_ident(&mut __elem_value) {
                #error_handling
            }
        })
    } else {
        None
    };

    // Generate the element push code with optional transform_each_after
    let element_push_code = if let Some(ref transform_after) = transform_each_after_code {
        quote! {
            {
                let mut __elem_value = v;
                #transform_after
                vec.push(__elem_value);
            }
        }
    } else {
        quote! { vec.push(v) }
    };

    // Generate deserialization with optional transform_each
    // When sorting is used, we need explicit type annotation because the sort closure needs to know the type
    let deserialization_code = if let Some(ref t) = attrs.transform_each {
        let transform_each_ident: proc_macro2::TokenStream = parse_transform_path(t);
        if has_sorting {
            // Use explicit type when sorting - needed for type inference with sort_by closure
            let elem_ty = element_type.expect("Vec should have inner type");
            quote! {
                #error_init
                #[allow(unused_mut)]
                let mut result: Vec<#elem_ty> = match array_value {
                    compote::ContextValue::Array(arr, _) => {
                        let mut vec = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            tracker.push_index(i);
                            // Apply transform_each to each item
                            let transformed_item = {
                                let mut temp = item.clone();
                                let ctx = temp.context().clone();
                                if let Err(e) = #transform_each_ident(&mut temp, &ctx) {
                                    #error_handle_transform
                                }
                                temp
                            };
                            match compote::FromContextValue::from_context_value(&transformed_item, tracker) {
                                Ok(v) => #element_push_code,
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
                    compote::ContextValue::Array(arr, _) => {
                        let mut vec = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            tracker.push_index(i);
                            // Apply transform_each to each item
                            let transformed_item = {
                                let mut temp = item.clone();
                                let ctx = temp.context().clone();
                                if let Err(e) = #transform_each_ident(&mut temp, &ctx) {
                                    #error_handle_transform
                                }
                                temp
                            };
                            match compote::FromContextValue::from_context_value(&transformed_item, tracker) {
                                Ok(v) => #element_push_code,
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
        }
    } else {
        if has_sorting {
            // Use explicit type when sorting - needed for type inference with sort_by closure
            let elem_ty = element_type.expect("Vec should have inner type");
            quote! {
                #error_init
                #[allow(unused_mut)]
                let mut result: Vec<#elem_ty> = match array_value {
                    compote::ContextValue::Array(arr, _) => {
                        let mut vec = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            tracker.push_index(i);
                            match compote::FromContextValue::from_context_value(item, tracker) {
                                Ok(v) => #element_push_code,
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
                    compote::ContextValue::Array(arr, _) => {
                        let mut vec = Vec::new();
                        for (i, item) in arr.iter().enumerate() {
                            tracker.push_index(i);
                            match compote::FromContextValue::from_context_value(item, tracker) {
                                Ok(v) => #element_push_code,
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
        }
    };

    // Generate sorting code (for allow_map with order_by or order_by_fn, or standalone order_by/order_by_fn)
    let sorting_code = if let Some(ref allow_map_config) = attrs.allow_map {
        // Explicit allow_map config - check its order_by/order_by_fn
        if let Some(ref order_by_field) = allow_map_config.order_by {
            // Sort by field name (ascending)
            let field_ident: proc_macro2::TokenStream = order_by_field.parse().unwrap();
            quote! {
                if was_from_map {
                    result.sort_by(|a, b| a.#field_ident.cmp(&b.#field_ident));
                }
            }
        } else if let Some(ref order_by_fn_name) = allow_map_config.order_by_fn {
            // Sort using custom function
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
        let #field_name = if let Some(field_value) = #field_lookup {
            // Check if it's explicitly null
            if field_value.is_null() {
                #missing_field_handling
            } else {
                tracker.push_field(#field_name_str);

                // mutable_by check
                #mutable_by_check

                // Step 1: Normalize to array (allow_single/allow_map logic)
                #normalization_code

                // Step 2: Apply transform to entire array (if present)
                #array_transform_code

                // Step 3: Deserialize items with optional transform_each
                #deserialization_code

                // Step 4: Sort result if input was from map and order_by/order_by_fn specified
                #sorting_code

                tracker.pop();
                // Convert Vec to target collection type (Vec, BTreeSet, HashSet, etc.)
                result.into_iter().collect()
            }
        } else {
            #missing_field_handling
        };
    }
}

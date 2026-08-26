//! Codegen for the validation attribute family.
//!
//! Generates the runtime validation checks for `#[feuilletage(range(..))]`,
//! `#[feuilletage(regex = ..)]`, `#[feuilletage(length(..))]`,
//! `#[feuilletage(validate = ..)]`, `#[feuilletage(absolute_path)]`, etc.
//! Called from the unified deserialization codegen.

use quote::quote;

use crate::attrs::{FieldConfigAttributes, OnErrorMode};

pub(crate) fn generate_validation_code(
    _field_name: &syn::Ident,
    field_name_str: &str,
    attrs: &FieldConfigAttributes,
    default_expr: Option<&proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
    let mut validations = Vec::new();
    let is_secret = attrs.secret;
    let on_error = attrs.on_error.unwrap_or_default();

    // Helper: generate validation failure action based on on_error mode
    // - fail: always return error
    // - default/skip: if there's a default, record error and use default; otherwise return error
    let fail_action = |error_var: &str| -> proc_macro2::TokenStream {
        let error_ident: proc_macro2::TokenStream = error_var.parse().unwrap();
        match on_error {
            OnErrorMode::Fail => {
                // Always fail, even if there's a default
                quote! {
                    tracker.record(#error_ident.clone());
                    tracker.pop();
                    return Err(#error_ident);
                }
            }
            OnErrorMode::Default | OnErrorMode::Skip => {
                // Use default if available, otherwise fail
                if let Some(default) = default_expr {
                    quote! {
                        tracker.record(#error_ident);
                        __temp_value = #default;
                    }
                } else {
                    quote! {
                        tracker.record(#error_ident.clone());
                        tracker.pop();
                        return Err(#error_ident);
                    }
                }
            }
        }
    };

    // Range validation
    if let Some(ref range) = attrs.range {
        if let Some(ref min) = range.min {
            let min_val: proc_macro2::TokenStream = min.parse().unwrap();
            let error_msg = if is_secret {
                quote! { format!("Field '{}' value is less than minimum {}", #field_name_str, #min_val) }
            } else {
                quote! { format!("Field '{}' value {} is less than minimum {}", #field_name_str, __temp_value, #min_val) }
            };
            let on_fail = fail_action("error");
            validations.push(quote! {
                if (__temp_value as f64) < (#min_val as f64) {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: #error_msg,
                    };
                    #on_fail
                }
            });
        }
        if let Some(ref max) = range.max {
            let max_val: proc_macro2::TokenStream = max.parse().unwrap();
            let error_msg = if is_secret {
                quote! { format!("Field '{}' value is greater than maximum {}", #field_name_str, #max_val) }
            } else {
                quote! { format!("Field '{}' value {} is greater than maximum {}", #field_name_str, __temp_value, #max_val) }
            };
            let on_fail = fail_action("error");
            validations.push(quote! {
                if (__temp_value as f64) > (#max_val as f64) {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: #error_msg,
                    };
                    #on_fail
                }
            });
        }
    }

    // Regex validation
    if let Some(ref pattern) = attrs.regex {
        let error_msg = if is_secret {
            quote! { format!("Field '{}' value does not match pattern '{}'", #field_name_str, #pattern) }
        } else {
            quote! { format!("Field '{}' value '{}' does not match pattern '{}'", #field_name_str, value_str, #pattern) }
        };
        let on_fail = fail_action("error");
        validations.push(quote! {
            {
                let re = feuilletage::regex::Regex::new(#pattern).map_err(|e| {
                    feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Invalid regex pattern '{}': {}", #pattern, e),
                    }
                })?;
                let value_str = __temp_value.to_string();
                if !re.is_match(&value_str) {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: #error_msg,
                    };
                    #on_fail
                }
            }
        });
    }

    // Length validation (for strings and collections)
    if let Some(ref length) = attrs.length {
        if let Some(ref min) = length.min {
            let min_val: proc_macro2::TokenStream = min.parse().unwrap();
            let on_fail = fail_action("error");
            validations.push(quote! {
                if __temp_value.len() < #min_val {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Field '{}' length {} is less than minimum {}", #field_name_str, __temp_value.len(), #min_val),
                    };
                    #on_fail
                }
            });
        }
        if let Some(ref max) = length.max {
            let max_val: proc_macro2::TokenStream = max.parse().unwrap();
            let on_fail = fail_action("error");
            validations.push(quote! {
                if __temp_value.len() > #max_val {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: format!("Field '{}' length {} is greater than maximum {}", #field_name_str, __temp_value.len(), #max_val),
                    };
                    #on_fail
                }
            });
        }
    }

    // Custom validation function
    if let Some(ref validate_fn) = attrs.validate {
        let fn_ident: proc_macro2::TokenStream = validate_fn.parse().unwrap();
        let on_fail = fail_action("error");
        validations.push(quote! {
            if let Err(msg) = #fn_ident(&__temp_value) {
                let error = feuilletage::Error::InvalidValue {
                    path: tracker.current_path(),
                    message: format!("Field '{}' validation failed: {}", #field_name_str, msg),
                };
                #on_fail
            }
        });
    }

    // Absolute path validation
    if attrs.absolute_path {
        let error_msg = if is_secret {
            quote! { format!("Field '{}' must be an absolute path", #field_name_str) }
        } else {
            quote! { format!("Field '{}' must be an absolute path, got: {}", #field_name_str, path.display()) }
        };
        let on_fail = fail_action("error");
        validations.push(quote! {
            {
                let path = std::path::Path::new(&__temp_value);
                if !path.is_absolute() {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: #error_msg,
                    };
                    #on_fail
                }
            }
        });
    }

    // Date/time format validation using chrono (requires chrono feature)
    if let Some(ref format_str) = attrs.datetime {
        let error_msg = if is_secret {
            quote! { format!("Field '{}' does not match date/time format '{}'", #field_name_str, #format_str) }
        } else {
            quote! { format!("Field '{}' value '{}' does not match date/time format '{}'", #field_name_str, value_str, #format_str) }
        };
        let on_fail = fail_action("error");
        validations.push(quote! {
            {
                let value_str: String = __temp_value.to_string();
                if feuilletage::chrono::NaiveDateTime::parse_from_str(&value_str, #format_str).is_err()
                    && feuilletage::chrono::NaiveDate::parse_from_str(&value_str, #format_str).is_err()
                    && feuilletage::chrono::NaiveTime::parse_from_str(&value_str, #format_str).is_err()
                {
                    let error = feuilletage::Error::InvalidValue {
                        path: tracker.current_path(),
                        message: #error_msg,
                    };
                    #on_fail
                }
            }
        });
    }

    if validations.is_empty() {
        quote! {}
    } else {
        quote! {
            #(#validations)*
        }
    }
}

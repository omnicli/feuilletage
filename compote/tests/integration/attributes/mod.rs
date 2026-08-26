//! Attribute tests for the Compote configuration library.
//!
//! These tests validate various field and struct attributes including:
//! - Structural: flatten, aliases, allow_map, allow_single, rename, required, vec_attrs
//! - Defaults: default_fn
//! - Environment: env, mutable_by
//! - Transforms: coerce, duration, relative_path, transform
//! - Validators: range, length, regex, custom, absolute_path, combined, datetime
//! - Serialization: skip_serialization (skip, skip_if_empty, skip_if_default, skip_if, skip_if_empty_recursive)
//! - Metadata: deprecated, secret
//! - Template: template interpolation

// ============================================================================
// Structural attributes
// ============================================================================

mod aliases_test;
mod allow_list_test;
mod allow_map_test;
mod allow_single_test;
mod fallback_test;
mod flatten_test;
mod order_by_test;
mod post_process_test;
mod rename_test;
mod required_test;
mod vec_attrs_test;

// ============================================================================
// Default attributes
// ============================================================================

mod default_fn_params_test;
mod default_fn_test;
mod default_probe_test;

// ============================================================================
// Environment attributes
// ============================================================================

mod env_test;
mod mutable_by_test;

// ============================================================================
// Serialization attributes
// ============================================================================

mod skip_serialization_test;
mod transparent_test;

// ============================================================================
// Transform attributes
// ============================================================================

mod coerce_test;
mod container_transform_test;
mod duration_test;
mod normalize_path_test;
mod relative_path_test;
mod transform_after_test;
mod transform_test;

// ============================================================================
// Validation attributes
// ============================================================================

mod absolute_path_test;
mod combined_test;
mod custom_test;
mod length_test;
mod range_test;

#[cfg(feature = "regex")]
mod regex_test;

#[cfg(feature = "chrono")]
mod datetime_test;

// ============================================================================
// Metadata attributes
// ============================================================================

mod deprecated_test;
mod secret_test;

// ============================================================================
// Error handling attributes
// ============================================================================

mod on_error_test;

// ============================================================================
// Context injection attributes
// ============================================================================

mod from_context_fn_test;
mod from_context_test;

// ============================================================================
// Template attributes
// ============================================================================

mod template_test;

// ============================================================================
// Enum attributes
// ============================================================================

mod attribute_reference_test;
mod enum_allow_single_test;
mod external_tag_test;
mod rename_all_test;
mod variant_value_test;

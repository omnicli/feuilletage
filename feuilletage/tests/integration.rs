//! Integration tests for the Feuilletage configuration library.
//!
//! These tests validate the public API and behavior of Feuilletage
//! through real-world usage patterns.
//!
//! Test categories:
//! - `attributes/` - All macro attribute tests (structural, transforms, validators, serialization, env, mutable_by)
//! - `config/` - Config operations tests (basics, modifiers, file_ops, edit_api, format_features)
//! - `types/` - Type tests (primitives, collections, options, enums, type_mismatch)
//! - `error_tracker_test` - Error tracker functionality

#![allow(
    clippy::approx_constant,
    clippy::collapsible_match,
    clippy::enum_variant_names,
    clippy::ptr_arg
)]

// ============================================================================
// Attribute tests (structural, transforms, validators, serialization, env, mutable_by)
// ============================================================================

#[path = "integration/attributes/mod.rs"]
mod attributes;

// ============================================================================
// Config operations tests
// ============================================================================

#[path = "integration/config/mod.rs"]
mod config;

// ============================================================================
// Type tests
// ============================================================================

#[path = "integration/types/mod.rs"]
mod types;

// ============================================================================
// Error tracker tests
// ============================================================================

#[path = "integration/error_tracker_test.rs"]
mod error_tracker_test;

// ============================================================================
// Error codes tests
// ============================================================================

#[path = "integration/error_codes_test.rs"]
mod error_codes_test;

// ============================================================================
// Custom Source/Level types tests
// ============================================================================

#[path = "integration/custom_types_test.rs"]
mod custom_types_test;

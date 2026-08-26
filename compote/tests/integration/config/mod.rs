//! Config tests for the Compote configuration library.
//!
//! Tests for:
//! - basics_test: Basic deserialization, nested merge, multi-format loading
//! - modifiers_test: Merge modifiers (__toreplace, __toappend, __toprepend, __tokeep)
//! - file_ops_test: File operations (write_file, edit_file, edit_first_existing)
//! - edit_api_test: Edit API for modifying config values
//! - format_features_test: Format feature flags (yaml, json, toml)
//! - roundtrip_test: Round-trip serialization/deserialization for YAML, JSON, TOML

mod basics_test;
mod edit_api_test;
mod file_ops_test;
mod format_features_test;
mod modifiers_test;
mod roundtrip_test;

//! Unit tests extracted from source files.
//!
//! These tests were originally inline tests in the source modules under `src/`.
//! They test internal/private functionality and are organized by source module.
//!
//! This file includes all unit test modules from the `unit/` directory.

#![allow(clippy::approx_constant, clippy::collapsible_match)]

#[path = "unit/coerce_test.rs"]
mod coerce_test;

#[cfg(feature = "json")]
#[path = "unit/config_test.rs"]
mod config_test;

#[path = "unit/context_test.rs"]
mod context_test;

#[path = "unit/de_test.rs"]
mod de_test;

#[path = "unit/edit_test.rs"]
mod edit_test;

#[path = "unit/loader_test.rs"]
mod loader_test;

#[path = "unit/merge_test.rs"]
mod merge_test;

#[path = "unit/ser_test.rs"]
mod ser_test;

#[path = "unit/template_test.rs"]
mod template_test;

#[path = "unit/transform_test.rs"]
mod transform_test;

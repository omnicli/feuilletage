//! Unit tests for merge module (merge strategies and mutability).
//!
//! Extracted from feuilletage/src/merge.rs

use feuilletage::error::ErrorTracker;
use feuilletage::merge::merge_values;
use feuilletage::value::MergeModifier;
use feuilletage::{Context, ContextValue, Level, MutabilityConstraint, Source};
use indexmap::IndexMap;

fn test_context() -> Context {
    Context::new(Source::Programmatic, Level::User)
}

#[test]
fn test_merge_primitives() {
    let mut tracker = ErrorTracker::new();
    let mut base = ContextValue::int(42, test_context());
    let new = ContextValue::int(100, test_context());

    merge_values(&mut base, new, MergeModifier::Default, &mut tracker);

    assert!(matches!(&base, ContextValue::Int(100, _)));
    assert!(!tracker.has_errors());
}

#[test]
fn test_merge_objects() {
    let mut tracker = ErrorTracker::new();

    let mut base_map = IndexMap::new();
    base_map.insert("a".to_string(), ContextValue::int(1, test_context()));
    base_map.insert("b".to_string(), ContextValue::int(2, test_context()));
    let mut base = ContextValue::object(base_map, test_context());

    let mut new_map = IndexMap::new();
    new_map.insert("b".to_string(), ContextValue::int(3, test_context()));
    new_map.insert("c".to_string(), ContextValue::int(4, test_context()));
    let new = ContextValue::object(new_map, test_context());

    merge_values(&mut base, new, MergeModifier::Default, &mut tracker);

    if let ContextValue::Object(map, _) = &base {
        assert_eq!(map.len(), 3);
        assert!(matches!(map.get("a").unwrap(), ContextValue::Int(1, _)));
        assert!(matches!(map.get("b").unwrap(), ContextValue::Int(3, _)));
        assert!(matches!(map.get("c").unwrap(), ContextValue::Int(4, _)));
    } else {
        panic!("Expected object");
    }
    assert!(!tracker.has_errors());
}

#[test]
fn test_immutable_blocks_override() {
    let mut tracker = ErrorTracker::new();

    let mut base = ContextValue::int(
        42,
        test_context().with_mutability_constraint(MutabilityConstraint::Immutable),
    );
    let new = ContextValue::int(100, test_context());

    merge_values(&mut base, new, MergeModifier::Default, &mut tracker);

    // Value should not have changed
    assert!(matches!(&base, ContextValue::Int(42, _)));
    // Should have recorded an error
    assert!(tracker.has_errors());
    assert_eq!(tracker.errors().len(), 1);
}

#[test]
fn test_tokeep_modifier() {
    let mut tracker = ErrorTracker::new();

    let mut base = ContextValue::int(42, test_context());
    let new = ContextValue::int(100, test_context());

    merge_values(&mut base, new, MergeModifier::ToKeep, &mut tracker);

    // Value should not have changed
    assert!(matches!(&base, ContextValue::Int(42, _)));
    assert!(!tracker.has_errors());
}

#[test]
fn test_toappend_modifier() {
    let mut tracker = ErrorTracker::new();

    let mut base = ContextValue::array(
        vec![
            ContextValue::int(1, test_context()),
            ContextValue::int(2, test_context()),
        ],
        test_context(),
    );
    let new = ContextValue::int(3, test_context());

    merge_values(&mut base, new, MergeModifier::ToAppend, &mut tracker);

    if let ContextValue::Array(arr, _) = &base {
        assert_eq!(arr.len(), 3);
        assert!(matches!(&arr[2], ContextValue::Int(3, _)));
    } else {
        panic!("Expected array");
    }
    assert!(!tracker.has_errors());
}

#[test]
fn test_toprepend_modifier() {
    let mut tracker = ErrorTracker::new();

    let mut base = ContextValue::array(
        vec![
            ContextValue::int(1, test_context()),
            ContextValue::int(2, test_context()),
        ],
        test_context(),
    );
    let new = ContextValue::int(3, test_context());

    merge_values(&mut base, new, MergeModifier::ToPrepend, &mut tracker);

    if let ContextValue::Array(arr, _) = &base {
        assert_eq!(arr.len(), 3);
        assert!(matches!(&arr[0], ContextValue::Int(3, _)));
    } else {
        panic!("Expected array");
    }
    assert!(!tracker.has_errors());
}

#[test]
fn test_toreplace_modifier() {
    let mut tracker = ErrorTracker::new();

    let mut base_map = IndexMap::new();
    base_map.insert("a".to_string(), ContextValue::int(1, test_context()));
    base_map.insert("b".to_string(), ContextValue::int(2, test_context()));
    let mut base = ContextValue::object(base_map, test_context());

    let mut new_map = IndexMap::new();
    new_map.insert("c".to_string(), ContextValue::int(3, test_context()));
    let new = ContextValue::object(new_map, test_context());

    merge_values(&mut base, new, MergeModifier::ToReplace, &mut tracker);

    if let ContextValue::Object(map, _) = &base {
        // Should only have 'c', 'a' and 'b' should be gone
        assert_eq!(map.len(), 1);
        assert!(matches!(map.get("c").unwrap(), ContextValue::Int(3, _)));
        assert!(map.get("a").is_none());
        assert!(map.get("b").is_none());
    } else {
        panic!("Expected object");
    }
    assert!(!tracker.has_errors());
}

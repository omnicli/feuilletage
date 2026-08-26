#[cfg(not(feature = "std"))]
use alloc::{
    format,
    string::{String, ToString},
};

use crate::{
    __private::IndexMap,
    context::{LevelType, SourceType},
    error::ErrorTracker,
    value::{parse_key_modifier, ContextValue, MergeModifier},
};

// Use hashbrown HashMap for MutabilityConstraints - works in both std and no_std
use hashbrown::HashMap as MutabilityHashMap;

/// Merge `new` into `base` in place, applying a [`MergeModifier`].
///
/// Objects are merged key-by-key; everything else is replaced. Mutability
/// constraints on `base.context()` are honored — an override from a level
/// that cannot modify `base` is recorded as an error on `tracker` and the
/// merge skips that value.
///
/// ```
/// use compote::{Context, ContextValue, Level, OrderedMap, Source};
/// use compote::error::ErrorTracker;
/// use compote::merge::merge_values;
/// use compote::value::MergeModifier;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
///
/// let mut base = ContextValue::Object({
///     let mut m = OrderedMap::default();
///     m.insert("a".to_string(), ContextValue::Int(1, ctx.clone()));
///     m
/// }, ctx.clone());
///
/// let new = ContextValue::Object({
///     let mut m = OrderedMap::default();
///     m.insert("b".to_string(), ContextValue::Int(2, ctx.clone()));
///     m
/// }, ctx);
///
/// let mut tracker = ErrorTracker::new();
/// merge_values(&mut base, new, MergeModifier::Default, &mut tracker);
///
/// // Both keys are present after the merge.
/// if let ContextValue::Object(m, _) = &base {
///     assert!(m.contains_key("a"));
///     assert!(m.contains_key("b"));
/// }
/// ```
pub fn merge_values<S: SourceType, L: LevelType>(
    base: &mut ContextValue<S, L>,
    new: ContextValue<S, L>,
    modifier: MergeModifier,
    tracker: &mut ErrorTracker,
) {
    // Check mutability constraints - if the new level cannot modify the base, record error and skip
    if !base.context().can_be_overridden_by(&new.context().level) {
        tracker.record_immutable_override(new.context().source.display_name());
        return;
    }

    match modifier {
        // Never override - keep the existing value
        MergeModifier::ToKeep => {}
        MergeModifier::ToReplace => {
            // Full replacement - replace entirely regardless of type
            *base = new;
        }
        MergeModifier::ToAppend => {
            // Append to array, or replace if not an array
            match base {
                ContextValue::Array(ref mut arr, _) => {
                    arr.push(new);
                }
                _ => {
                    // Not an array, replace
                    *base = new;
                }
            }
        }
        MergeModifier::ToPrepend => {
            // Prepend to array, or replace if not an array
            match base {
                ContextValue::Array(ref mut arr, _) => {
                    arr.insert(0, new);
                }
                _ => {
                    // Not an array, replace
                    *base = new;
                }
            }
        }
        MergeModifier::Default => {
            // Default merge strategy
            match (base, &new) {
                (
                    ContextValue::Object(ref mut base_map, _),
                    ContextValue::Object(ref new_map, _),
                ) => {
                    // Merge objects recursively
                    merge_objects(base_map, new_map, new.context(), tracker);
                }
                (base_val, _) => {
                    // For all other types (including arrays), replace
                    *base_val = new;
                }
            }
        }
    }
}

/// Merge two objects recursively
fn merge_objects<S: SourceType, L: LevelType>(
    base: &mut IndexMap<String, ContextValue<S, L>>,
    new: &IndexMap<String, ContextValue<S, L>>,
    new_context: &crate::context::Context<S, L>,
    tracker: &mut ErrorTracker,
) {
    for (raw_key, new_value) in new {
        // Parse key for merge modifier
        let (key, modifier) = parse_key_modifier(raw_key);

        tracker.push_field(&key);

        if let Some(base_value) = base.get_mut(&key) {
            // Key exists - check for mutability at this level or parent levels
            if is_override_blocked(base_value, &key, &modifier, &new_context.level) {
                tracker.record_immutable_override(new_context.source.display_name());
            } else {
                // Merge the values
                merge_values(base_value, new_value.clone(), modifier, tracker);
            }
        } else {
            // Key doesn't exist - add it
            base.insert(key.clone(), new_value.clone());
        }

        tracker.pop();
    }
}

/// Check if an override is blocked by mutability constraints
fn is_override_blocked<S: SourceType, L: LevelType>(
    base: &ContextValue<S, L>,
    _key: &str,
    modifier: &MergeModifier,
    new_level: &L,
) -> bool {
    // If the value itself cannot be overridden by this level, override is blocked
    if !base.context().can_be_overridden_by(new_level) {
        return true;
    }

    // For object types with constrained children, check if this would cause a parent-level override
    if matches!(modifier, MergeModifier::ToReplace) {
        // ToReplace would delete all existing fields, which would affect constrained children
        if let ContextValue::Object(ref map, _) = base {
            // Check if any child has mutability constraints that would block this
            return has_constrained_descendant(map, new_level);
        }
    }

    false
}

/// Recursively check if any descendant in an object tree has mutability constraints
/// that would block modification by the given level
fn has_constrained_descendant<S: SourceType, L: LevelType>(
    map: &IndexMap<String, ContextValue<S, L>>,
    new_level: &L,
) -> bool {
    for value in map.values() {
        if !value.context().can_be_overridden_by(new_level) {
            return true;
        }
        if let ContextValue::Object(ref child_map, _) = value {
            if has_constrained_descendant(child_map, new_level) {
                return true;
            }
        }
    }
    false
}

/// Check if a merge at a parent level would affect an immutable descendant
pub fn would_affect_immutable_descendant<S: SourceType, L: LevelType>(
    base: &ContextValue<S, L>,
    path_components: &[String],
) -> Option<String> {
    if path_components.is_empty() {
        if matches!(
            base.context().mutability,
            crate::context::MutabilityConstraint::Immutable
        ) {
            return Some(String::new());
        }
        return None;
    }

    if let ContextValue::Object(ref map, _) = base {
        let key = &path_components[0];
        if let Some(child) = map.get(key) {
            if let Some(subpath) = would_affect_immutable_descendant(child, &path_components[1..]) {
                if subpath.is_empty() {
                    return Some(key.clone());
                } else {
                    return Some(format!("{}.{}", key, subpath));
                }
            }
        }
    }

    None
}

/// Type alias for mutability constraints map (field name -> allowed levels)
pub type MutabilityConstraints = MutabilityHashMap<String, &'static [&'static str]>;

/// Merge a new ContextValue into a base ContextValue, enforcing mutability constraints.
///
/// This function is similar to `merge_values`, but it additionally checks if the
/// incoming value's config level is allowed by the struct-level `mutable_by` constraint
/// for each field. If a field has a `mutable_by` constraint and the incoming value's
/// level is not in the allowed list, the value is SKIPPED (not merged) and a warning
/// is recorded.
///
/// # Arguments
///
/// * `base` - The base ContextValue to merge into (typically an object)
/// * `new` - The new ContextValue to merge (typically an object)
/// * `level` - The Level of the new value's source
/// * `constraints` - Map of field names to their allowed Levels
/// * `tracker` - ErrorTracker for recording warnings
///
/// # How it works
///
/// For each field in the new config:
/// 1. Check if the field has a mutability constraint in `constraints`
/// 2. If it does, check if `level` is in the allowed levels
/// 3. If level is NOT allowed, skip the field and record a warning
/// 4. If level IS allowed (or no constraint exists), merge normally
pub fn merge_with_mutability_constraints<S: SourceType, L: LevelType>(
    base: &mut ContextValue<S, L>,
    new: ContextValue<S, L>,
    level: &L,
    constraints: &MutabilityConstraints,
    tracker: &mut ErrorTracker,
) {
    // Handle the case where both are objects (the common case for config merging)
    match (base, &new) {
        (ContextValue::Object(ref mut base_map, _), ContextValue::Object(ref new_map, _)) => {
            merge_objects_with_constraints(
                base_map,
                new_map,
                new.context(),
                level,
                constraints,
                "",
                tracker,
            );
        }
        (base_val, _) => {
            // For non-objects at the root level, just do a standard merge
            // (constraints don't apply to the root itself)
            merge_values(base_val, new, MergeModifier::Default, tracker);
        }
    }
}

/// Merge two objects with mutability constraints checking at the field level.
fn merge_objects_with_constraints<S: SourceType, L: LevelType>(
    base: &mut IndexMap<String, ContextValue<S, L>>,
    new: &IndexMap<String, ContextValue<S, L>>,
    new_context: &crate::context::Context<S, L>,
    level: &L,
    constraints: &MutabilityConstraints,
    path_prefix: &str,
    tracker: &mut ErrorTracker,
) {
    for (raw_key, new_value) in new {
        // Parse key for merge modifier
        let (key, modifier) = parse_key_modifier(raw_key);
        let field_path = if path_prefix.is_empty() {
            key.clone()
        } else {
            format!("{path_prefix}.{key}")
        };

        // Check if this field has a mutable_by constraint
        if let Some(allowed_level_names) = constraints.get(field_path.as_str()) {
            // Check if the incoming level is allowed
            if !is_level_name_allowed(level.name(), allowed_level_names) {
                // Level not allowed - skip this field and record a warning
                tracker.record_mutability_warning(&field_path, level.name(), allowed_level_names);
                continue; // Skip this field
            }
        }

        tracker.push_field(&key);

        if let Some(base_value) = base.get_mut(&key) {
            // Key exists - check for mutability at this level or parent levels
            if is_override_blocked(base_value, &key, &modifier, &new_context.level) {
                tracker.record_immutable_override(new_context.source.display_name());
            } else if replacement_erases_nested_values(base_value, new_value, &modifier)
                && record_blocked_existing_nested_constraints(
                    base_value,
                    constraints,
                    &field_path,
                    level.name(),
                    tracker,
                )
            {
                // A shape-changing replacement cannot preserve only part of the
                // old value, so retain the complete parent when any protected
                // descendant would otherwise be erased.
            } else if matches!(modifier, MergeModifier::Default) {
                match (base_value, new_value) {
                    (
                        ContextValue::Object(ref mut base_map, _),
                        ContextValue::Object(ref new_map, _),
                    ) if has_nested_constraints(constraints, &field_path) => {
                        merge_objects_with_constraints(
                            base_map,
                            new_map,
                            new_value.context(),
                            level,
                            constraints,
                            &field_path,
                            tracker,
                        );
                    }
                    (base_value, ContextValue::Object(new_map, context))
                        if has_nested_constraints(constraints, &field_path) =>
                    {
                        let mut filtered = IndexMap::default();
                        merge_objects_with_constraints(
                            &mut filtered,
                            new_map,
                            context,
                            level,
                            constraints,
                            &field_path,
                            tracker,
                        );
                        *base_value = ContextValue::Object(filtered, context.clone());
                    }
                    (base_value, _) => {
                        merge_values(base_value, new_value.clone(), modifier, tracker);
                    }
                }
            } else if let Some((blocked_path, allowed_level_names)) =
                first_blocked_nested_constraint(constraints, &field_path, level.name())
            {
                tracker.record_mutability_warning(blocked_path, level.name(), allowed_level_names);
            } else {
                // Merge the values
                merge_values(base_value, new_value.clone(), modifier, tracker);
            }
        } else {
            // A nested object still needs filtering when its parent has not
            // appeared in an earlier source.
            if let ContextValue::Object(new_map, context) = new_value {
                if has_nested_constraints(constraints, &field_path) {
                    let mut filtered = IndexMap::default();
                    merge_objects_with_constraints(
                        &mut filtered,
                        new_map,
                        context,
                        level,
                        constraints,
                        &field_path,
                        tracker,
                    );
                    base.insert(key.clone(), ContextValue::Object(filtered, context.clone()));
                } else {
                    base.insert(key.clone(), new_value.clone());
                }
            } else {
                base.insert(key.clone(), new_value.clone());
            }
        }

        tracker.pop();
    }
}

fn replacement_erases_nested_values<S: SourceType, L: LevelType>(
    base: &ContextValue<S, L>,
    new: &ContextValue<S, L>,
    modifier: &MergeModifier,
) -> bool {
    match modifier {
        MergeModifier::ToKeep => false,
        MergeModifier::Default => {
            matches!(base, ContextValue::Object(..)) && !matches!(new, ContextValue::Object(..))
        }
        MergeModifier::ToAppend | MergeModifier::ToPrepend => {
            !matches!(base, ContextValue::Array(..))
        }
        MergeModifier::ToReplace => true,
    }
}

fn record_blocked_existing_nested_constraints<S: SourceType, L: LevelType>(
    base: &ContextValue<S, L>,
    constraints: &MutabilityConstraints,
    path: &str,
    level_name: &str,
    tracker: &mut ErrorTracker,
) -> bool {
    let prefix = format!("{path}.");
    let mut blocked = false;

    for (constraint_path, allowed_levels) in constraints {
        let Some(relative_path) = constraint_path.strip_prefix(&prefix) else {
            continue;
        };
        if !is_level_name_allowed(level_name, allowed_levels)
            && value_contains_path(base, relative_path)
        {
            tracker.record_mutability_warning(constraint_path, level_name, allowed_levels);
            blocked = true;
        }
    }

    blocked
}

fn value_contains_path<S: SourceType, L: LevelType>(
    value: &ContextValue<S, L>,
    path: &str,
) -> bool {
    let mut current = value;
    for component in path.split('.') {
        let ContextValue::Object(map, _) = current else {
            return false;
        };
        let Some(child) = map.get(component) else {
            return false;
        };
        current = child;
    }
    true
}

fn has_nested_constraints(constraints: &MutabilityConstraints, path: &str) -> bool {
    let mut prefix = path.to_string();
    prefix.push('.');
    constraints.keys().any(|key| key.starts_with(&prefix))
}

fn first_blocked_nested_constraint<'a>(
    constraints: &'a MutabilityConstraints,
    path: &str,
    level_name: &str,
) -> Option<(&'a str, &'a [&'static str])> {
    let mut prefix = path.to_string();
    prefix.push('.');
    constraints.iter().find_map(|(key, allowed)| {
        (key.starts_with(&prefix) && !is_level_name_allowed(level_name, allowed))
            .then_some((key.as_str(), *allowed))
    })
}

/// Check if a level name is in the list of allowed level names.
///
/// This is used for mutable_by constraint checking where we compare
/// level names as strings.
fn is_level_name_allowed(level_name: &str, allowed_names: &[&str]) -> bool {
    allowed_names.contains(&level_name)
}

// Unit tests have been moved to compote/tests/unit/merge_test.rs

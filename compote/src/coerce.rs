//! Liberal type-coercion helpers.
//!
//! These functions accept a [`ContextValue`] and try to extract a value of the
//! requested primitive type, performing common conversions that the strict
//! deserializer would reject. They return [`None`] when no reasonable
//! conversion exists.
//!
//! The macro's `#[compote(coerce)]` attribute uses these functions under the
//! hood.

#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

use crate::context::{LevelType, SourceType};
use crate::value::ContextValue;

/// Coerce a [`ContextValue`] to a [`String`].
///
/// Accepts string, bool, integer, and float values. Returns [`None`] for
/// arrays, objects, and null.
///
/// ```
/// use compote::{Context, ContextValue, Source, Level};
/// use compote::coerce::coerce_to_string;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(coerce_to_string(&ContextValue::Int(42, ctx.clone())), Some("42".to_string()));
/// assert_eq!(coerce_to_string(&ContextValue::Bool(true, ctx.clone())), Some("true".to_string()));
/// assert_eq!(coerce_to_string(&ContextValue::Null(ctx)), None);
/// ```
pub fn coerce_to_string<S: SourceType, L: LevelType>(value: &ContextValue<S, L>) -> Option<String> {
    match value {
        ContextValue::String(s, _) => Some(s.to_string()),
        ContextValue::Bool(b, _) => Some(b.to_string()),
        ContextValue::Int(i, _) => Some(i.to_string()),
        ContextValue::Float(f, _) => Some(f.to_string()),
        _ => None,
    }
}

/// Coerce a [`ContextValue`] to a [`bool`].
///
/// Accepts:
/// - native bool values,
/// - the strings `"true"`/`"false"`/`"yes"`/`"no"`/`"1"`/`"0"` (case-insensitive),
/// - the integers `0` (→ false) and `1` (→ true).
///
/// ```
/// use compote::{Context, ContextValue, Source, Level};
/// use compote::coerce::coerce_to_bool;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(coerce_to_bool(&ContextValue::String("yes".into(), ctx.clone())), Some(true));
/// assert_eq!(coerce_to_bool(&ContextValue::Int(0, ctx.clone())), Some(false));
/// assert_eq!(coerce_to_bool(&ContextValue::String("maybe".into(), ctx)), None);
/// ```
pub fn coerce_to_bool<S: SourceType, L: LevelType>(value: &ContextValue<S, L>) -> Option<bool> {
    match value {
        ContextValue::Bool(b, _) => Some(*b),
        ContextValue::String(s, _) => match s.to_lowercase().as_str() {
            "true" | "yes" | "y" | "on" | "1" => Some(true),
            "false" | "no" | "n" | "off" | "0" => Some(false),
            _ => None,
        },
        ContextValue::Int(i, _) => Some(*i != 0),
        ContextValue::Float(f, _) => Some(*f != 0.0),
        _ => None,
    }
}

/// Coerce a [`ContextValue`] to an [`i64`].
///
/// Accepts integers, floats with no fractional part, parseable numeric
/// strings, and bools (`true` → `1`, `false` → `0`).
///
/// ```
/// use compote::{Context, ContextValue, Source, Level};
/// use compote::coerce::coerce_to_i64;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(coerce_to_i64(&ContextValue::String("  42 ".into(), ctx.clone())), Some(42));
/// assert_eq!(coerce_to_i64(&ContextValue::Float(3.0, ctx.clone())), Some(3));
/// assert_eq!(coerce_to_i64(&ContextValue::Float(3.5, ctx)), None);
/// ```
pub fn coerce_to_i64<S: SourceType, L: LevelType>(value: &ContextValue<S, L>) -> Option<i64> {
    match value {
        ContextValue::Int(i, _) => Some(*i),
        ContextValue::Float(f, _) if *f == (*f as i64) as f64 => Some(*f as i64),
        ContextValue::String(s, _) => s.trim().parse().ok(),
        ContextValue::Bool(b, _) => Some(if *b { 1 } else { 0 }),
        _ => None,
    }
}

/// Coerce a [`ContextValue`] to a [`u64`].
///
/// Same rules as [`coerce_to_i64`] but rejects negative numbers.
///
/// ```
/// use compote::{Context, ContextValue, Source, Level};
/// use compote::coerce::coerce_to_u64;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(coerce_to_u64(&ContextValue::Int(7, ctx.clone())), Some(7));
/// assert_eq!(coerce_to_u64(&ContextValue::Int(-1, ctx)), None);
/// ```
pub fn coerce_to_u64<S: SourceType, L: LevelType>(value: &ContextValue<S, L>) -> Option<u64> {
    match value {
        ContextValue::Int(i, _) if *i >= 0 => Some(*i as u64),
        ContextValue::Float(f, _) if *f >= 0.0 && *f == (*f as u64) as f64 => Some(*f as u64),
        ContextValue::String(s, _) => s.trim().parse().ok(),
        ContextValue::Bool(b, _) => Some(if *b { 1 } else { 0 }),
        _ => None,
    }
}

/// Coerce a [`ContextValue`] to an [`f64`].
///
/// Accepts integers, floats, parseable numeric strings, and bools.
///
/// ```
/// use compote::{Context, ContextValue, Source, Level};
/// use compote::coerce::coerce_to_f64;
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
/// assert_eq!(coerce_to_f64(&ContextValue::Int(2, ctx.clone())), Some(2.0));
/// assert_eq!(coerce_to_f64(&ContextValue::String("1.5".into(), ctx)), Some(1.5));
/// ```
pub fn coerce_to_f64<S: SourceType, L: LevelType>(value: &ContextValue<S, L>) -> Option<f64> {
    match value {
        ContextValue::Float(f, _) => Some(*f),
        ContextValue::Int(i, _) => Some(*i as f64),
        ContextValue::String(s, _) => s.trim().parse().ok(),
        ContextValue::Bool(b, _) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

// Unit tests have been moved to compote/tests/unit/coerce_test.rs

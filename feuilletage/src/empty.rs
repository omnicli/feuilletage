//! Empty checking trait for serialization control.
//!
//! This module provides the [`IsEmpty`] trait which allows types to indicate
//! whether they should be considered "empty" for serialization purposes.
//!
//! # Usage
//!
//! The `IsEmpty` trait is used by the `#[feuilletage(skip_if_empty)]` attribute
//! to determine whether a field should be skipped during serialization.
//!
//! # Built-in Implementations
//!
//! The following types have `IsEmpty` implementations:
//!
//! | Type | Empty when |
//! |------|------------|
//! | `Vec<T>` | Length is 0 |
//! | `Option<T>` | Is `None` |
//! | `String` | Length is 0 |
//! | `HashMap<K, V>` | Length is 0 |
//! | `HashSet<T>` | Length is 0 |
//! | `BTreeMap<K, V>` | Length is 0 |
//! | `BTreeSet<T>` | Length is 0 |
//!
//! # Custom Implementations
//!
//! You can implement `IsEmpty` for your own types to make them work with
//! `skip_if_empty`:
//!
//! ```
//! use feuilletage::IsEmpty;
//!
//! struct MyCollection {
//!     items: Vec<String>,
//!     metadata: Option<String>,
//! }
//!
//! impl IsEmpty for MyCollection {
//!     fn is_empty(&self) -> bool {
//!         self.items.is_empty() && self.metadata.is_none()
//!     }
//! }
//! ```

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[cfg(not(feature = "std"))]
use hashbrown::{HashMap, HashSet};

use core::hash::Hash;

/// Trait for checking if a value should be considered "empty".
///
/// This trait is used by the `#[feuilletage(skip_if_empty)]` attribute to
/// determine whether a field should be skipped during serialization.
///
/// # Example
///
/// ```
/// use feuilletage::IsEmpty;
///
/// // Built-in types implement IsEmpty
/// assert!(Vec::<i32>::new().is_empty());
/// assert!(!vec![1, 2, 3].is_empty());
///
/// assert!(Option::<String>::None.is_empty());
/// assert!(!Some("hello".to_string()).is_empty());
///
/// assert!(String::new().is_empty());
/// assert!(!"hello".to_string().is_empty());
/// ```
pub trait IsEmpty {
    /// Returns `true` if the value should be considered empty.
    fn is_empty(&self) -> bool;
}

// Implementation for Vec<T>
impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for Option<T>
impl<T> IsEmpty for Option<T> {
    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

// Implementation for String
impl IsEmpty for String {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for &str
impl IsEmpty for &str {
    fn is_empty(&self) -> bool {
        (*self).is_empty()
    }
}

// Implementation for HashMap
impl<K, V> IsEmpty for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for HashSet
impl<T> IsEmpty for HashSet<T>
where
    T: Eq + Hash,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for BTreeMap
impl<K, V> IsEmpty for BTreeMap<K, V>
where
    K: Ord,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for BTreeSet
impl<T> IsEmpty for BTreeSet<T>
where
    T: Ord,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for slices
impl<T> IsEmpty for [T] {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

// Implementation for arrays (always non-empty if N > 0)
impl<T, const N: usize> IsEmpty for [T; N] {
    fn is_empty(&self) -> bool {
        N == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_is_empty() {
        assert!(Vec::<i32>::new().is_empty());
        assert!(!vec![1, 2, 3].is_empty());
    }

    #[test]
    fn test_option_is_empty() {
        assert!(Option::<String>::None.is_empty());
        assert!(!Some("hello".to_string()).is_empty());
    }

    #[test]
    fn test_string_is_empty() {
        assert!(String::new().is_empty());
        assert!(!"hello".to_string().is_empty());
    }

    #[test]
    fn test_hashmap_is_empty() {
        let empty: HashMap<String, i32> = HashMap::new();
        assert!(empty.is_empty());

        let mut non_empty: HashMap<String, i32> = HashMap::new();
        non_empty.insert("key".to_string(), 42);
        assert!(!non_empty.is_empty());
    }
}

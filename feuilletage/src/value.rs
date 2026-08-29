//! Configuration value types.
//!
//! This module provides the core value types used to represent configuration data:
//!
//! - [`ContextValue`]: An enum with context embedded at each variant (the primary type)
//! - [`Value`]: A simpler contextless value type for internal operations
//! - [`MergeModifier`]: Controls how values are merged during configuration loading
//!
//! # Value Tree Structure
//!
//! Configuration is stored as a tree where each node is a [`ContextValue`].
//! The tree structure mirrors the hierarchical nature of configuration files:
//!
//! ```text
//! ContextValue (root object)
//! ├── "server" -> ContextValue (object)
//! │   ├── "host" -> ContextValue (string: "localhost")
//! │   └── "port" -> ContextValue (int: 8080)
//! └── "features" -> ContextValue (array)
//!     ├── [0] -> ContextValue (string: "auth")
//!     └── [1] -> ContextValue (string: "logging")
//! ```
//!
//! # Merge Modifiers
//!
//! When loading configuration, special key suffixes control merging behavior:
//!
//! - `key__tokeep`: Only set if key doesn't already exist
//! - `key__toappend`: Append new items to existing array
//! - `key__toprepend`: Prepend new items to existing array
//! - `key__toreplace`: Replace value entirely (even for objects)
//!
//! See [`crate::value::parse_key_modifier`] for parsing these suffixes from key names.

#[cfg(not(feature = "std"))]
use alloc::{string::String, string::ToString, vec::Vec};

use serde::{Deserialize, Serialize};

use crate::__private::IndexMap;
use crate::context::{Context, Level, LevelType, Source, SourceType};

/// A configuration value with context embedded at each variant.
///
/// This is the primary type for representing configuration data. Each variant
/// contains the value data along with a [`Context`] that tracks metadata about
/// the value's origin, priority level, and mutability constraints.
///
/// # Type Parameters
///
/// - `S`: Source type, defaults to [`Source`]
/// - `L`: Level type, defaults to [`Level`]
///
/// # Examples
///
/// ```
/// use feuilletage::{ContextValue, Context, Level, Source};
///
/// let ctx = Context::new(Source::Programmatic, Level::User);
///
/// // Create different types of values
/// let string_val = ContextValue::string("hello", ctx.clone());
/// let int_val = ContextValue::int(42, ctx.clone());
/// let bool_val = ContextValue::bool(true, ctx.clone());
/// let array_val = ContextValue::array(vec![int_val.clone()], ctx.clone());
///
/// assert_eq!(string_val.type_name(), "string");
/// assert_eq!(int_val.type_name(), "int");
/// assert_eq!(bool_val.type_name(), "bool");
/// assert_eq!(string_val.as_str(), Some("hello"));
/// assert_eq!(int_val.as_i64(), Some(42));
/// assert_eq!(array_val.as_array().unwrap().len(), 1);
///
/// // Access the context
/// assert_eq!(string_val.context().level, Level::User);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    bound = "S: SourceType + Serialize + for<'a> Deserialize<'a>, L: LevelType + Serialize + for<'a> Deserialize<'a>"
)]
pub enum ContextValue<S: SourceType = Source, L: LevelType = Level> {
    /// Null value with context
    Null(Context<S, L>),
    /// Boolean value with context
    Bool(bool, Context<S, L>),
    /// Integer value with context
    Int(i64, Context<S, L>),
    /// Float value with context
    Float(f64, Context<S, L>),
    /// String value with context
    String(String, Context<S, L>),
    /// Array value with context (children have their own contexts)
    Array(Vec<ContextValue<S, L>>, Context<S, L>),
    /// Object value with context (children have their own contexts)
    Object(IndexMap<String, ContextValue<S, L>>, Context<S, L>),
}

impl<S: SourceType, L: LevelType> ContextValue<S, L> {
    // =========================================================================
    // Constructors
    // =========================================================================

    /// Creates a null value with the given context.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::null(Context::new(Source::Programmatic, Level::Local));
    /// assert!(value.is_null());
    /// ```
    pub fn null(context: Context<S, L>) -> Self {
        ContextValue::Null(context)
    }

    /// Creates a boolean value with the given context.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::bool(true, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_bool(), Some(true));
    /// ```
    pub fn bool(b: bool, context: Context<S, L>) -> Self {
        ContextValue::Bool(b, context)
    }

    /// Creates an integer value with the given context.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::int(8080, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_i64(), Some(8080));
    /// ```
    pub fn int(i: i64, context: Context<S, L>) -> Self {
        ContextValue::Int(i, context)
    }

    /// Creates a floating-point value with the given context.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::float(0.75, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_f64(), Some(0.75));
    /// ```
    pub fn float(f: f64, context: Context<S, L>) -> Self {
        ContextValue::Float(f, context)
    }

    /// Creates a string value with the given context.
    ///
    /// Accepts any type that implements `Into<String>`.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::string("localhost", Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_str(), Some("localhost"));
    /// ```
    pub fn string(s: impl Into<String>, context: Context<S, L>) -> Self {
        ContextValue::String(s.into(), context)
    }

    /// Creates an array value with the given context.
    ///
    /// The items should be ContextValue instances with their own contexts.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let value = ContextValue::array(vec![ContextValue::int(1, context.clone())], context);
    /// assert_eq!(value.as_array().unwrap()[0].as_i64(), Some(1));
    /// ```
    pub fn array(items: Vec<ContextValue<S, L>>, context: Context<S, L>) -> Self {
        ContextValue::Array(items, context)
    }

    /// Creates an object value with the given context.
    ///
    /// Uses [`IndexMap`] to preserve insertion order of keys.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, OrderedMap, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let mut fields = OrderedMap::default();
    /// fields.insert("port".to_string(), ContextValue::int(8080, context.clone()));
    /// let value = ContextValue::object(fields, context);
    /// assert_eq!(value.as_object().unwrap()["port"].as_i64(), Some(8080));
    /// ```
    pub fn object(fields: IndexMap<String, ContextValue<S, L>>, context: Context<S, L>) -> Self {
        ContextValue::Object(fields, context)
    }

    /// Creates a new value from a contextless [`Value`] and a context.
    ///
    /// This converts a contextless [`Value`] into a [`ContextValue`] with the specified context.
    /// Nested values receive the same context.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source, Value};
    ///
    /// let context = Context::new(Source::Programmatic, Level::User);
    /// let value = ContextValue::new(Value::Array(vec![Value::Int(1)]), context);
    /// let child = &value.as_array().unwrap()[0];
    /// assert_eq!(child.as_i64(), Some(1));
    /// assert_eq!(child.context().level, Level::User);
    /// ```
    pub fn new(value: Value, context: Context<S, L>) -> Self {
        match value {
            Value::Null => ContextValue::Null(context),
            Value::Bool(b) => ContextValue::Bool(b, context),
            Value::Int(i) => ContextValue::Int(i, context),
            Value::Float(f) => ContextValue::Float(f, context),
            Value::String(s) => ContextValue::String(s, context),
            Value::Array(items) => {
                // Convert each Value item to ContextValue with the same context
                let config_items: Vec<ContextValue<S, L>> = items
                    .into_iter()
                    .map(|v| ContextValue::new(v, context.clone()))
                    .collect();
                ContextValue::Array(config_items, context)
            }
            Value::Object(fields) => {
                // Convert each Value field to ContextValue with the same context
                let config_fields: IndexMap<String, ContextValue<S, L>> = fields
                    .into_iter()
                    .map(|(k, v)| (k, ContextValue::new(v, context.clone())))
                    .collect();
                ContextValue::Object(config_fields, context)
            }
        }
    }

    // =========================================================================
    // Context access
    // =========================================================================

    /// Returns a reference to the context of this value.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::int(1, Context::new(Source::Programmatic, Level::User));
    /// assert_eq!(value.context().level, Level::User);
    /// ```
    pub fn context(&self) -> &Context<S, L> {
        match self {
            ContextValue::Null(ctx) => ctx,
            ContextValue::Bool(_, ctx) => ctx,
            ContextValue::Int(_, ctx) => ctx,
            ContextValue::Float(_, ctx) => ctx,
            ContextValue::String(_, ctx) => ctx,
            ContextValue::Array(_, ctx) => ctx,
            ContextValue::Object(_, ctx) => ctx,
        }
    }

    /// Returns a mutable reference to the context of this value.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let mut value = ContextValue::int(1, Context::new(Source::Programmatic, Level::System));
    /// value.context_mut().level = Level::Local;
    /// assert_eq!(value.context().level, Level::Local);
    /// ```
    pub fn context_mut(&mut self) -> &mut Context<S, L> {
        match self {
            ContextValue::Null(ctx) => ctx,
            ContextValue::Bool(_, ctx) => ctx,
            ContextValue::Int(_, ctx) => ctx,
            ContextValue::Float(_, ctx) => ctx,
            ContextValue::String(_, ctx) => ctx,
            ContextValue::Array(_, ctx) => ctx,
            ContextValue::Object(_, ctx) => ctx,
        }
    }

    // =========================================================================
    // Type checking
    // =========================================================================

    /// Returns the type name of this value for use in error messages.
    ///
    /// Returns one of: "null", "bool", "int", "float", "string", "array", "object".
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::float(1.5, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.type_name(), "float");
    /// ```
    pub fn type_name(&self) -> &'static str {
        match self {
            ContextValue::Null(_) => "null",
            ContextValue::Bool(_, _) => "bool",
            ContextValue::Int(_, _) => "int",
            ContextValue::Float(_, _) => "float",
            ContextValue::String(_, _) => "string",
            ContextValue::Array(_, _) => "array",
            ContextValue::Object(_, _) => "object",
        }
    }

    /// Returns `true` if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, ContextValue::Null(_))
    }

    /// Returns `true` if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, ContextValue::Bool(_, _))
    }

    /// Returns `true` if this value is an integer.
    pub fn is_int(&self) -> bool {
        matches!(self, ContextValue::Int(_, _))
    }

    /// Returns `true` if this value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, ContextValue::Float(_, _))
    }

    /// Returns `true` if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, ContextValue::String(_, _))
    }

    /// Returns `true` if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, ContextValue::Array(_, _))
    }

    /// Returns `true` if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, ContextValue::Object(_, _))
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Returns a reference to the inner object if this value is an Object variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, OrderedMap, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let mut fields = OrderedMap::default();
    /// fields.insert("enabled".to_string(), ContextValue::bool(true, context.clone()));
    /// let value = ContextValue::object(fields, context);
    /// assert_eq!(value.as_object().unwrap()["enabled"].as_bool(), Some(true));
    /// ```
    pub fn as_object(&self) -> Option<&IndexMap<String, ContextValue<S, L>>> {
        match self {
            ContextValue::Object(map, _) => Some(map),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner object if this value is an Object variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, OrderedMap, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let mut value = ContextValue::object(OrderedMap::default(), context.clone());
    /// value.as_object_mut().unwrap().insert(
    ///     "port".to_string(),
    ///     ContextValue::int(8080, context),
    /// );
    /// assert_eq!(value.as_object().unwrap()["port"].as_i64(), Some(8080));
    /// ```
    pub fn as_object_mut(&mut self) -> Option<&mut IndexMap<String, ContextValue<S, L>>> {
        match self {
            ContextValue::Object(map, _) => Some(map),
            _ => None,
        }
    }

    /// Returns a reference to the inner array if this value is an Array variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let value = ContextValue::array(vec![ContextValue::string("a", context.clone())], context);
    /// assert_eq!(value.as_array().unwrap()[0].as_str(), Some("a"));
    /// ```
    pub fn as_array(&self) -> Option<&Vec<ContextValue<S, L>>> {
        match self {
            ContextValue::Array(arr, _) => Some(arr),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner array if this value is an Array variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let mut value = ContextValue::array(Vec::new(), context.clone());
    /// value.as_array_mut().unwrap().push(ContextValue::int(2, context));
    /// assert_eq!(value.as_array().unwrap()[0].as_i64(), Some(2));
    /// ```
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<ContextValue<S, L>>> {
        match self {
            ContextValue::Array(arr, _) => Some(arr),
            _ => None,
        }
    }

    /// Returns the inner string if this value is a String variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::string("dev", Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_str(), Some("dev"));
    /// assert_eq!(
    ///     ContextValue::<Source, Level>::int(1, Context::default()).as_str(),
    ///     None,
    /// );
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ContextValue::String(s, _) => Some(s),
            _ => None,
        }
    }

    /// Returns the inner boolean if this value is a Bool variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::bool(false, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_bool(), Some(false));
    /// assert_eq!(
    ///     ContextValue::<Source, Level>::null(Context::default()).as_bool(),
    ///     None,
    /// );
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ContextValue::Bool(b, _) => Some(*b),
            _ => None,
        }
    }

    /// Returns the inner integer if this value is an Int variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::int(-3, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_i64(), Some(-3));
    /// assert_eq!(
    ///     ContextValue::<Source, Level>::float(3.0, Context::default()).as_i64(),
    ///     None,
    /// );
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ContextValue::Int(i, _) => Some(*i),
            _ => None,
        }
    }

    /// Returns the inner float if this value is a Float variant.
    ///
    /// ```
    /// use feuilletage::{Context, ContextValue, Level, Source};
    ///
    /// let value = ContextValue::float(2.5, Context::new(Source::Programmatic, Level::Local));
    /// assert_eq!(value.as_f64(), Some(2.5));
    /// assert_eq!(
    ///     ContextValue::<Source, Level>::int(2, Context::default()).as_f64(),
    ///     None,
    /// );
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ContextValue::Float(f, _) => Some(*f),
            _ => None,
        }
    }
}

/// Convert a [`Value`] to a [`ContextValue`] with default context.
///
/// ```
/// use feuilletage::{ContextValue, Value};
///
/// let value: ContextValue = Value::Int(7).into();
/// assert_eq!(value.as_i64(), Some(7));
/// assert_eq!(value.context(), &Default::default());
/// ```
impl<S: SourceType + Default, L: LevelType + Default> From<Value> for ContextValue<S, L> {
    fn from(value: Value) -> Self {
        ContextValue::new(value, Context::default())
    }
}

/// Convert a [`ContextValue`] to a [`Value`] (owned conversion, strips context).
///
/// ```
/// use feuilletage::{Context, ContextValue, Level, Source, Value};
///
/// let contextual = ContextValue::string(
///     "localhost",
///     Context::new(Source::Programmatic, Level::User),
/// );
/// let value = Value::from(contextual);
/// assert_eq!(value, Value::String("localhost".to_string()));
/// ```
impl<S: SourceType, L: LevelType> From<ContextValue<S, L>> for Value {
    fn from(cv: ContextValue<S, L>) -> Self {
        match cv {
            ContextValue::Null(_) => Value::Null,
            ContextValue::Bool(b, _) => Value::Bool(b),
            ContextValue::Int(i, _) => Value::Int(i),
            ContextValue::Float(f, _) => Value::Float(f),
            ContextValue::String(s, _) => Value::String(s),
            ContextValue::Array(arr, _) => Value::Array(arr.into_iter().map(Value::from).collect()),
            ContextValue::Object(map, _) => {
                Value::Object(map.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Convert a [`ContextValue`] reference to a [`Value`] (borrowed conversion, strips context).
///
/// ```
/// use feuilletage::{Context, ContextValue, Level, Source, Value};
///
/// let contextual = ContextValue::int(
///     42,
///     Context::new(Source::Programmatic, Level::User),
/// );
/// let value = Value::from(&contextual);
/// assert_eq!(value, Value::Int(42));
/// assert_eq!(contextual.as_i64(), Some(42));
/// ```
impl<S: SourceType, L: LevelType> From<&ContextValue<S, L>> for Value {
    fn from(cv: &ContextValue<S, L>) -> Self {
        match cv {
            ContextValue::Null(_) => Value::Null,
            ContextValue::Bool(b, _) => Value::Bool(*b),
            ContextValue::Int(i, _) => Value::Int(*i),
            ContextValue::Float(f, _) => Value::Float(*f),
            ContextValue::String(s, _) => Value::String(s.clone()),
            ContextValue::Array(arr, _) => Value::Array(arr.iter().map(Value::from).collect()),
            ContextValue::Object(map, _) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Value::from(v)))
                    .collect(),
            ),
        }
    }
}

/// The contextless configuration data type.
///
/// Represents the different types of values that can appear in configuration,
/// without any associated context metadata. This is useful for:
/// - Internal operations that don't need context tracking
/// - Converting to/from external formats
/// - Simple value comparisons
///
/// For most configuration operations, use [`ContextValue`] instead.
///
/// # Examples
///
/// ```
/// use feuilletage::Value;
///
/// let null = Value::Null;
/// let boolean = Value::Bool(true);
/// let integer = Value::Int(42);
/// let float = Value::Float(3.14);
/// let string = Value::String("hello".to_string());
///
/// assert_eq!(null.type_name(), "null");
/// assert_eq!(boolean.type_name(), "bool");
/// assert_eq!(integer.type_name(), "int");
/// assert_eq!(float.type_name(), "float");
/// assert_eq!(string.type_name(), "string");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value (i64)
    Int(i64),
    /// Floating point value (f64)
    Float(f64),
    /// String value
    String(String),
    /// Array/list of values (self-referential)
    Array(Vec<Value>),
    /// Object/map of key-value pairs (preserving insertion order, self-referential)
    Object(IndexMap<String, Value>),
}

impl Value {
    /// Gets the type name for error messages.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::Array(Vec::new()).type_name(), "array");
    /// assert_eq!(Value::Object(Default::default()).type_name(), "object");
    /// ```
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// Returns `true` if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns `true` if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` if this value is an integer.
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Returns `true` if this value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Returns `true` if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Returns `true` if this value is an object.
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// Returns a reference to the inner object if this value is an Object variant.
    ///
    /// ```
    /// use feuilletage::{OrderedMap, Value};
    ///
    /// let mut fields = OrderedMap::default();
    /// fields.insert("workers".to_string(), Value::Int(4));
    /// let value = Value::Object(fields);
    /// assert_eq!(value.as_object().unwrap()["workers"], Value::Int(4));
    /// ```
    pub fn as_object(&self) -> Option<&IndexMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner object if this value is an Object variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// let mut value = Value::Object(Default::default());
    /// value
    ///     .as_object_mut()
    ///     .unwrap()
    ///     .insert("debug".to_string(), Value::Bool(true));
    /// assert_eq!(value.as_object().unwrap()["debug"].as_bool(), Some(true));
    /// ```
    pub fn as_object_mut(&mut self) -> Option<&mut IndexMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Returns a reference to the inner array if this value is an Array variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// let value = Value::Array(vec![Value::String("json".to_string())]);
    /// assert_eq!(value.as_array().unwrap()[0].as_str(), Some("json"));
    /// ```
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner array if this value is an Array variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// let mut value = Value::Array(Vec::new());
    /// value.as_array_mut().unwrap().push(Value::Int(1));
    /// assert_eq!(value.as_array().unwrap(), &[Value::Int(1)]);
    /// ```
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Returns the inner string if this value is a String variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::String("prod".to_string()).as_str(), Some("prod"));
    /// assert_eq!(Value::Int(1).as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the inner boolean if this value is a Bool variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::Bool(true).as_bool(), Some(true));
    /// assert_eq!(Value::Null.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the inner integer if this value is an Int variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::Int(10).as_i64(), Some(10));
    /// assert_eq!(Value::Float(10.0).as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Returns the inner float if this value is a Float variant.
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::Float(1.25).as_f64(), Some(1.25));
    /// assert_eq!(Value::Int(1).as_f64(), None);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Converts this Value to a ContextValue with the given context.
    ///
    /// The context is attached recursively to every nested value.
    ///
    /// ```
    /// use feuilletage::{Context, Level, Source, Value};
    ///
    /// let context = Context::new(Source::Programmatic, Level::Local);
    /// let value = Value::Array(vec![Value::Int(1)]).with_context(context);
    /// let child = value.as_array().unwrap().first().unwrap();
    ///
    /// assert_eq!(value.context().level, Level::Local);
    /// assert_eq!(child.context().level, Level::Local);
    /// ```
    pub fn with_context<S: SourceType, L: LevelType>(
        self,
        context: Context<S, L>,
    ) -> ContextValue<S, L> {
        ContextValue::new(self, context)
    }
}

impl Default for Value {
    /// The default `Value` is `Value::Null`.
    ///
    /// This lets `Value` be used as a `#[feuilletage(default)]` field type
    /// (the field will resolve to `Value::Null` when missing).
    ///
    /// ```
    /// use feuilletage::Value;
    ///
    /// assert_eq!(Value::default(), Value::Null);
    /// ```
    fn default() -> Self {
        Value::Null
    }
}

/// Converts a JSON value recursively while preserving scalar number types.
///
/// ```
/// use feuilletage::Value;
///
/// let value = Value::from(serde_json::json!({"enabled": true, "ports": [80, 443]}));
/// let object = value.as_object().unwrap();
/// assert_eq!(object["enabled"].as_bool(), Some(true));
/// assert_eq!(object["ports"].as_array().unwrap()[1].as_i64(), Some(443));
/// ```
impl From<serde_json::Value> for Value {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i)
                } else if let Some(f) = n.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Null
                }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(obj) => {
                Value::Object(obj.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
        }
    }
}

/// Merge modifier that can be applied to configuration keys.
///
/// When loading configuration, keys can have special suffixes that control
/// how they are merged with existing values:
///
/// - `key__tokeep`: Only set if `key` doesn't exist
/// - `key__toappend`: Append to existing array at `key`
/// - `key__toprepend`: Prepend to existing array at `key`
/// - `key__toreplace`: Replace value at `key` entirely (even for objects)
///
/// # Examples
///
/// ```
/// use feuilletage::value::parse_key_modifier;
/// use feuilletage::MergeModifier;
///
/// let (key, modifier) = parse_key_modifier("items__toappend");
/// assert_eq!(key, "items");
/// assert_eq!(modifier, MergeModifier::ToAppend);
///
/// let (key, modifier) = parse_key_modifier("config__toreplace");
/// assert_eq!(key, "config");
/// assert_eq!(modifier, MergeModifier::ToReplace);
///
/// let (key, modifier) = parse_key_modifier("normal_key");
/// assert_eq!(key, "normal_key");
/// assert_eq!(modifier, MergeModifier::Default);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeModifier {
    /// Default merge behavior (recursive for objects, replace for primitives/arrays)
    Default,
    /// Only set if not already present (`__tokeep` suffix)
    ToKeep,
    /// Append to array (`__toappend` suffix)
    ToAppend,
    /// Prepend to array (`__toprepend` suffix)
    ToPrepend,
    /// Full replacement, removing unspecified fields (`__toreplace` suffix)
    ToReplace,
}

/// Parses a key and extracts any merge modifier suffix.
///
/// Keys without a recognized suffix use [`MergeModifier::Default`].
///
/// ```
/// use feuilletage::value::parse_key_modifier;
/// use feuilletage::MergeModifier;
///
/// assert_eq!(parse_key_modifier("host"), ("host".to_string(), MergeModifier::Default));
/// assert_eq!(parse_key_modifier("host__tokeep").1, MergeModifier::ToKeep);
/// assert_eq!(parse_key_modifier("items__toappend").1, MergeModifier::ToAppend);
/// assert_eq!(parse_key_modifier("items__toprepend").1, MergeModifier::ToPrepend);
/// assert_eq!(parse_key_modifier("server__toreplace").1, MergeModifier::ToReplace);
/// ```
pub fn parse_key_modifier(key: &str) -> (String, MergeModifier) {
    if let Some(base) = key.strip_suffix("__tokeep") {
        (base.to_string(), MergeModifier::ToKeep)
    } else if let Some(base) = key.strip_suffix("__toappend") {
        (base.to_string(), MergeModifier::ToAppend)
    } else if let Some(base) = key.strip_suffix("__toprepend") {
        (base.to_string(), MergeModifier::ToPrepend)
    } else if let Some(base) = key.strip_suffix("__toreplace") {
        (base.to_string(), MergeModifier::ToReplace)
    } else {
        (key.to_string(), MergeModifier::Default)
    }
}

use crate::error::ValueAccessError;
use std::collections::BTreeMap;
use std::ops::{Index, IndexMut};

/// Time value, semantically identical to the lexical time literal.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TimeValue {
    LocalDate(String),
    LocalTime(String),
    LocalDateTime(String),
    OffsetDateTime(String),
}

/// A runtime-friendly TyCL value.
///
/// Maps are stored as ordered `BTreeMap` for deterministic serialization
/// and comparison. Lists and tuples both use `Vec<Value>`; the caller
/// can distinguish them via the enclosing `Value` variant.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Time(TimeValue),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Record(BTreeMap<String, Value>),
    Tuple(Vec<Value>),
}

/// A parsed TyCL document.
///
/// The top-level is an ordered map because TyCL documents are
/// fundamentally key-value pairs. Preserving order is useful for
/// round-tripping and error reporting.
#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub root: BTreeMap<String, Value>,
}

impl Document {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.root.get(key)
    }
}

impl Index<&str> for Document {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        self.root.get(key).expect("key not found")
    }
}

impl IndexMut<&str> for Document {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.root.get_mut(key).expect("key not found")
    }
}

// ========================================================================
// Value type checking
// ========================================================================

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    pub fn is_time(&self) -> bool {
        matches!(self, Value::Time(_))
    }
    pub fn is_list(&self) -> bool {
        matches!(self, Value::List(_) | Value::Tuple(_))
    }
    pub fn is_map(&self) -> bool {
        matches!(self, Value::Map(_) | Value::Record(_))
    }
    pub fn is_record(&self) -> bool {
        matches!(self, Value::Record(_))
    }
    pub fn is_tuple(&self) -> bool {
        matches!(self, Value::Tuple(_))
    }
}

// ========================================================================
// Value access with null semantics
// ========================================================================

impl Value {
    pub fn as_bool(&self) -> Result<Option<&bool>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Bool(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "bool",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_bool_mut(&mut self) -> Result<Option<&mut bool>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Bool(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "bool",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_integer(&self) -> Result<Option<&i64>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Integer(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "int",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_integer_mut(&mut self) -> Result<Option<&mut i64>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Integer(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "int",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_float(&self) -> Result<Option<&f64>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Float(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "float",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_float_mut(&mut self) -> Result<Option<&mut f64>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Float(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "float",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_string(&self) -> Result<Option<&str>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::String(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "str",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_string_mut(&mut self) -> Result<Option<&mut str>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::String(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "str",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_time(&self) -> Result<Option<&TimeValue>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Time(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "time",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_time_mut(&mut self) -> Result<Option<&mut TimeValue>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Time(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "time",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_list(&self) -> Result<Option<&Vec<Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::List(inner) | Value::Tuple(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "list",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_list_mut(&mut self) -> Result<Option<&mut Vec<Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::List(inner) | Value::Tuple(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "list",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_map(&self) -> Result<Option<&BTreeMap<String, Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Map(inner) | Value::Record(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "map",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_map_mut(&mut self) -> Result<Option<&mut BTreeMap<String, Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Map(inner) | Value::Record(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "map",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_record(&self) -> Result<Option<&BTreeMap<String, Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Record(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "record",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_record_mut(
        &mut self,
    ) -> Result<Option<&mut BTreeMap<String, Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Record(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "record",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_tuple(&self) -> Result<Option<&Vec<Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Tuple(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "tuple",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_tuple_mut(&mut self) -> Result<Option<&mut Vec<Value>>, ValueAccessError> {
        match self {
            Value::Null => Ok(None),
            Value::Tuple(inner) => Ok(Some(inner)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "tuple",
                found: self.type_name(),
            }),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Integer(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "str",
            Value::Time(_) => "time",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Record(_) => "record",
            Value::Tuple(_) => "tuple",
        }
    }
}

// ========================================================================
// Generic conversion accessors
// ========================================================================

impl Value {
    pub fn as_list_t<T>(&self) -> Result<Option<Vec<T>>, ValueAccessError>
    where
        T: TryFromValue,
    {
        match self {
            Value::Null => Ok(None),
            Value::List(items) | Value::Tuple(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(T::try_from_value(item.clone())?);
                }
                Ok(Some(out))
            }
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "list",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_map_t<T>(&self) -> Result<Option<BTreeMap<String, T>>, ValueAccessError>
    where
        T: TryFromValue,
    {
        match self {
            Value::Null => Ok(None),
            Value::Map(map) | Value::Record(map) => {
                let mut out = BTreeMap::new();
                for (k, v) in map {
                    out.insert(k.clone(), T::try_from_value(v.clone())?);
                }
                Ok(Some(out))
            }
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "map",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_record_t<T>(&self) -> Result<Option<T>, ValueAccessError>
    where
        T: TryFromValue,
    {
        match self {
            Value::Null => Ok(None),
            Value::Record(_) => Ok(Some(T::try_from_value(self.clone())?)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "record",
                found: self.type_name(),
            }),
        }
    }

    pub fn as_tuple_t<T>(&self) -> Result<Option<T>, ValueAccessError>
    where
        T: TryFromValue,
    {
        match self {
            Value::Null => Ok(None),
            Value::Tuple(_) => Ok(Some(T::try_from_value(self.clone())?)),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "tuple",
                found: self.type_name(),
            }),
        }
    }
}

// ========================================================================
// Key/value access
// ========================================================================

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(map) | Value::Record(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Map(map) | Value::Record(map) => map.get_mut(key),
            _ => None,
        }
    }
}

// ========================================================================
// Indexing
// ========================================================================

impl Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found in map/record")
    }
}

impl IndexMut<&str> for Value {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.get_mut(key).expect("key not found in map/record")
    }
}

impl Index<usize> for Value {
    type Output = Value;
    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Value::List(v) | Value::Tuple(v) => v.get(idx).expect("index out of bounds"),
            _ => panic!("cannot index non-list/non-tuple value"),
        }
    }
}

impl IndexMut<usize> for Value {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        match self {
            Value::List(v) | Value::Tuple(v) => v.get_mut(idx).expect("index out of bounds"),
            _ => panic!("cannot index non-list/non-tuple value"),
        }
    }
}

// ========================================================================
// TryFromValue trait
// ========================================================================

pub trait TryFromValue: Sized {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError>;
}

// ========================================================================
// TryFromValue implementations for primitives and identity
// ========================================================================

impl TryFromValue for Value {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        Ok(value)
    }
}

impl TryFromValue for bool {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Bool(v) => Ok(v),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "bool",
                found: value.type_name(),
            }),
        }
    }
}

impl TryFromValue for i64 {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Integer(v) => Ok(v),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "int",
                found: value.type_name(),
            }),
        }
    }
}

impl TryFromValue for f64 {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Float(v) => Ok(v),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "float",
                found: value.type_name(),
            }),
        }
    }
}

impl TryFromValue for String {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::String(v) => Ok(v),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "str",
                found: value.type_name(),
            }),
        }
    }
}

impl TryFromValue for TimeValue {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Time(v) => Ok(v),
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "time",
                found: value.type_name(),
            }),
        }
    }
}

// ========================================================================
// Standard TryFrom implementations for concrete types
// ========================================================================

macro_rules! impl_try_from_value {
    ($ty:ty) => {
        impl TryFrom<Value> for $ty {
            type Error = ValueAccessError;
            fn try_from(value: Value) -> Result<Self, Self::Error> {
                <$ty>::try_from_value(value)
            }
        }
    };
}

impl_try_from_value!(bool);
impl_try_from_value!(i64);
impl_try_from_value!(f64);
impl_try_from_value!(String);
impl_try_from_value!(TimeValue);
impl_try_from_value!(Vec<Value>);
impl_try_from_value!(BTreeMap<String, Value>);

// ========================================================================
// TryFromValue implementations for containers
// ========================================================================

impl<T: TryFromValue> TryFromValue for Vec<T> {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::List(items) | Value::Tuple(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(T::try_from_value(item)?);
                }
                Ok(out)
            }
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "list",
                found: value.type_name(),
            }),
        }
    }
}

impl<T: TryFromValue> TryFromValue for BTreeMap<String, T> {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Map(map) | Value::Record(map) => {
                let mut out = BTreeMap::new();
                for (k, v) in map {
                    out.insert(k, T::try_from_value(v)?);
                }
                Ok(out)
            }
            _ => Err(ValueAccessError::TypeMismatch {
                expected: "map",
                found: value.type_name(),
            }),
        }
    }
}

impl<T: TryFromValue> TryFromValue for Option<T> {
    fn try_from_value(value: Value) -> Result<Self, ValueAccessError> {
        match value {
            Value::Null => Ok(None),
            other => T::try_from_value(other).map(Some),
        }
    }
}

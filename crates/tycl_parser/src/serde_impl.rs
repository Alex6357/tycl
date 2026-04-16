use crate::value::{Document, TimeValue, Value};
use serde::de::IntoDeserializer;
use serde::{
    Deserialize, Deserializer, Serializer,
    de::{
        self, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
        Visitor,
    },
    ser::{self, Serialize, SerializeMap, SerializeSeq},
};
use std::collections::BTreeMap;
use std::fmt;

/// Error type for serde operations.
#[derive(Debug, Clone)]
pub struct SerdeError(String);

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for SerdeError {}

impl ser::Error for SerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerdeError(msg.to_string())
    }
}

impl de::Error for SerdeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        SerdeError(msg.to_string())
    }
}

// ========================================================================
// Serialize for TimeValue
// ========================================================================

impl Serialize for TimeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TimeValue::LocalDate(s)
            | TimeValue::LocalTime(s)
            | TimeValue::LocalDateTime(s)
            | TimeValue::OffsetDateTime(s) => serializer.serialize_str(s),
        }
    }
}

struct TimeValueVisitor;

impl<'de> serde::de::Visitor<'de> for TimeValueVisitor {
    type Value = TimeValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a time string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let colon_count = v.matches(':').count();
        if v.contains('T') {
            if colon_count == 3 || v.ends_with('Z') {
                Ok(TimeValue::OffsetDateTime(v.to_owned()))
            } else if colon_count == 2 {
                Ok(TimeValue::LocalDateTime(v.to_owned()))
            } else {
                Err(E::custom(format!("invalid datetime format: {}", v)))
            }
        } else if colon_count == 2 {
            Ok(TimeValue::LocalTime(v.to_owned()))
        } else if v.len() == 10 && v.matches('-').count() == 2 {
            Ok(TimeValue::LocalDate(v.to_owned()))
        } else {
            Err(E::custom(format!("invalid time format: {}", v)))
        }
    }
}

impl<'de> Deserialize<'de> for TimeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TimeValueVisitor)
    }
}

// ========================================================================
// Serialize
// ========================================================================

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Null => serializer.serialize_none(),
            Value::Bool(v) => serializer.serialize_bool(*v),
            Value::Integer(v) => serializer.serialize_i64(*v),
            Value::Float(v) => serializer.serialize_f64(*v),
            Value::String(v) => serializer.serialize_str(v),
            Value::Time(v) => v.serialize(serializer),
            Value::List(v) | Value::Tuple(v) => {
                let mut seq = serializer.serialize_seq(Some(v.len()))?;
                for item in v {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Value::Map(v) | Value::Record(v) => {
                let mut map = serializer.serialize_map(Some(v.len()))?;
                for (k, val) in v {
                    map.serialize_entry(k, val)?;
                }
                map.end()
            }
        }
    }
}

impl Serialize for Document {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.root.serialize(serializer)
    }
}

// ========================================================================
// Deserialize
// ========================================================================

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid TyCL value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Value::Integer(v))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E> {
        Ok(Value::Integer(v as i64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v > i64::MAX as u64 {
            Err(E::custom(format!("u64 value {} exceeds i64::MAX", v)))
        } else {
            Ok(Value::Integer(v as i64))
        }
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Value::Float(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Value::Float(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::String(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(elem) = seq.next_element()? {
            vec.push(elem);
        }
        Ok(Value::List(vec))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut bt = BTreeMap::new();
        while let Some((k, v)) = map.next_entry()? {
            bt.insert(k, v);
        }
        Ok(Value::Map(bt))
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> serde::Deserialize<'de> for Document {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let root = BTreeMap::deserialize(deserializer)?;
        Ok(Document { root })
    }
}

// ========================================================================
// Deserializer for &Value
// ========================================================================

impl<'de> Deserializer<'de> for &'de Value {
    type Error = SerdeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            Value::Bool(v) => visitor.visit_bool(*v),
            Value::Integer(v) => visitor.visit_i64(*v),
            Value::Float(v) => visitor.visit_f64(*v),
            Value::String(v) => visitor.visit_str(v),
            Value::Time(v) => visitor.visit_str(match v {
                TimeValue::LocalDate(s)
                | TimeValue::LocalTime(s)
                | TimeValue::LocalDateTime(s)
                | TimeValue::OffsetDateTime(s) => s.as_str(),
            }),
            Value::List(v) => visitor.visit_seq(SeqRefDeserializer::new(v)),
            Value::Tuple(v) => visitor.visit_seq(SeqRefDeserializer::new(v)),
            Value::Map(v) => visitor.visit_map(MapRefDeserializer::new(v)),
            Value::Record(v) => visitor.visit_map(MapRefDeserializer::new(v)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Bool(v) => visitor.visit_bool(*v),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Integer(v) => visitor.visit_i64(*v),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Integer(v) => {
                if *v < 0 {
                    Err(de::Error::custom(
                        "negative i64 cannot be deserialized as u64",
                    ))
                } else {
                    visitor.visit_u64(*v as u64)
                }
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Float(v) => visitor.visit_f64(*v),
            Value::Integer(v) => visitor.visit_f64(*v as f64),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::String(v) => visitor.visit_str(v),
            Value::Time(v) => visitor.visit_str(match v {
                TimeValue::LocalDate(s)
                | TimeValue::LocalTime(s)
                | TimeValue::LocalDateTime(s)
                | TimeValue::OffsetDateTime(s) => s.as_str(),
            }),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null => visitor.visit_unit(),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::List(v) | Value::Tuple(v) => visitor.visit_seq(SeqRefDeserializer::new(v)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Map(v) | Value::Record(v) => visitor.visit_map(MapRefDeserializer::new(v)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Record(v) => visitor.visit_map(MapRefDeserializer::new(v)),
            Value::Map(v) => visitor.visit_map(MapRefDeserializer::new(v)),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::String(v) => visitor.visit_enum(v.as_str().into_deserializer()),
            Value::Map(v) | Value::Record(v) => {
                let mut iter = v.iter();
                let (key, value) = iter
                    .next()
                    .ok_or_else(|| de::Error::custom("empty map for enum"))?;
                if iter.next().is_some() {
                    return Err(de::Error::custom("map with more than one entry for enum"));
                }
                visitor.visit_enum(EnumRefDeserializer::new(key, value))
            }
            _ => Err(de::Error::custom("expected string or map for enum")),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct SeqRefDeserializer<'de> {
    iter: std::slice::Iter<'de, Value>,
}

impl<'de> SeqRefDeserializer<'de> {
    fn new(vec: &'de [Value]) -> Self {
        Self { iter: vec.iter() }
    }
}

impl<'de> SeqAccess<'de> for SeqRefDeserializer<'de> {
    type Error = SerdeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct MapRefDeserializer<'de> {
    iter: std::collections::btree_map::Iter<'de, String, Value>,
    value: Option<&'de Value>,
}

impl<'de> MapRefDeserializer<'de> {
    fn new(map: &'de BTreeMap<String, Value>) -> Self {
        Self {
            iter: map.iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapRefDeserializer<'de> {
    type Error = SerdeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("value missing")),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct EnumRefDeserializer<'de> {
    key: &'de str,
    value: &'de Value,
}

impl<'de> EnumRefDeserializer<'de> {
    fn new(key: &'de str, value: &'de Value) -> Self {
        Self { key, value }
    }
}

impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de> {
    type Error = SerdeError;
    type Variant = VariantRefDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.key.into_deserializer())?;
        Ok((variant, VariantRefDeserializer::new(self.value)))
    }
}

struct VariantRefDeserializer<'de> {
    value: &'de Value,
}

impl<'de> VariantRefDeserializer<'de> {
    fn new(value: &'de Value) -> Self {
        Self { value }
    }
}

impl<'de> VariantAccess<'de> for VariantRefDeserializer<'de> {
    type Error = SerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            Value::Null => Ok(()),
            _ => Err(de::Error::custom("expected unit variant")),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.value)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::List(v) | Value::Tuple(v) => visitor.visit_seq(SeqRefDeserializer::new(v)),
            _ => Err(de::Error::custom("expected tuple variant")),
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Value::Map(v) | Value::Record(v) => visitor.visit_map(MapRefDeserializer::new(v)),
            _ => Err(de::Error::custom("expected struct variant")),
        }
    }
}

// ========================================================================
// Serializer (Rust type -> Value)
// ========================================================================

struct ValueSerializer;

impl Serializer for ValueSerializer {
    type Ok = Value;
    type Error = SerdeError;
    type SerializeSeq = SerSeq;
    type SerializeTuple = SerTuple;
    type SerializeTupleStruct = SerTupleStruct;
    type SerializeTupleVariant = SerTupleVariant;
    type SerializeMap = SerMap;
    type SerializeStruct = SerStruct;
    type SerializeStructVariant = SerStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Integer(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if v > i64::MAX as u64 {
            Err(ser::Error::custom(format!(
                "u64 value {} exceeds i64::MAX",
                v
            )))
        } else {
            Ok(Value::Integer(v as i64))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.to_owned()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        let list: Vec<Value> = v.iter().map(|b| Value::Integer(*b as i64)).collect();
        Ok(Value::List(list))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(variant.to_owned()))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let mut map = BTreeMap::new();
        map.insert(variant.to_owned(), value.serialize(ValueSerializer)?);
        Ok(Value::Record(map))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerSeq::default())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerTuple::default())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerTupleStruct::default())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerTupleVariant::new(variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerMap::default())
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerStruct::default())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerStructVariant::new(variant))
    }
}

#[derive(Default)]
struct SerSeq {
    vec: Vec<Value>,
}

impl ser::SerializeSeq for SerSeq {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.vec.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::List(self.vec))
    }
}

#[derive(Default)]
struct SerTuple {
    vec: Vec<Value>,
}

impl ser::SerializeTuple for SerTuple {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.vec.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(self.vec))
    }
}

#[derive(Default)]
struct SerTupleStruct {
    vec: Vec<Value>,
}

impl ser::SerializeTupleStruct for SerTupleStruct {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.vec.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Tuple(self.vec))
    }
}

struct SerTupleVariant {
    name: String,
    vec: Vec<Value>,
}

impl SerTupleVariant {
    fn new(name: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            vec: Vec::new(),
        }
    }
}

impl ser::SerializeTupleVariant for SerTupleVariant {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.vec.push(value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = BTreeMap::new();
        map.insert(self.name, Value::Tuple(self.vec));
        Ok(Value::Record(map))
    }
}

#[derive(Default)]
struct SerMap {
    map: BTreeMap<String, Value>,
    key: Option<String>,
}

impl ser::SerializeMap for SerMap {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let value = key.serialize(ValueSerializer)?;
        match value {
            Value::String(s) => {
                self.key = Some(s);
                Ok(())
            }
            _ => Err(ser::Error::custom("map key must be a string")),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| ser::Error::custom("value missing"))?;
        self.map.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(self.map))
    }
}

#[derive(Default)]
struct SerStruct {
    map: BTreeMap<String, Value>,
}

impl ser::SerializeStruct for SerStruct {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.map
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Record(self.map))
    }
}

struct SerStructVariant {
    name: String,
    map: BTreeMap<String, Value>,
}

impl SerStructVariant {
    fn new(name: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            map: BTreeMap::new(),
        }
    }
}

impl ser::SerializeStructVariant for SerStructVariant {
    type Ok = Value;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.map
            .insert(key.to_owned(), value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map = BTreeMap::new();
        map.insert(self.name, Value::Record(self.map));
        Ok(Value::Record(map))
    }
}

// ========================================================================
// Public API
// ========================================================================

/// Serialize a Rust value into a TyCL `Value`.
pub fn to_value<T: Serialize>(value: T) -> Result<Value, SerdeError> {
    value.serialize(ValueSerializer)
}

/// Deserialize a TyCL `Value` into a Rust value.
pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, SerdeError> {
    T::deserialize(&value)
}

/// Serialize a Rust value into a TyCL `Document`.
pub fn to_document<T: Serialize>(value: T) -> Result<Document, SerdeError> {
    let value = value.serialize(ValueSerializer)?;
    match value {
        Value::Map(map) | Value::Record(map) => Ok(Document { root: map }),
        other => {
            let mut map = BTreeMap::new();
            map.insert("value".to_owned(), other);
            Ok(Document { root: map })
        }
    }
}

/// Deserialize a TyCL `Document` into a Rust value.
pub fn from_document<T: DeserializeOwned>(doc: Document) -> Result<T, SerdeError> {
    T::deserialize(&Value::Record(doc.root))
}

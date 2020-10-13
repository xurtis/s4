//! Wrapper for invocations of CMake

use std::collections::{BTreeMap, BTreeSet};
use serde::{Deserialize, Deserializer, de};
use std::fmt;

/// Definition of a configuration option
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Flag {
    name: FlagId,
    description: String,
    /// Flag is passed to CMake
    #[serde(default)]
    variable: Option<String>,
    #[serde(default)]
    requires: BTreeSet<BTreeMap<FlagId, Requirement>>,
}

/// Identifier of an option that can be supplied to CMake
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct FlagId(String);

/// A required setting for a particular flag
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Requirement {
    /// Requires that a flag be set to a specific value
    Single(Value),
    /// Requires that a flag be set to any of a set of values
    Any(BTreeSet<Value>),
}

struct RequirementVisitor;

impl RequirementVisitor {
    fn from_bool<E>(v: bool) -> Result<Requirement, E> {
        Ok(Requirement::Single(Value::Boolean(v)))
    }

    fn from_string<E>(v: impl ToString) -> Result<Requirement, E> {
        Ok(Requirement::Single(Value::Text(v.to_string())))
    }
}

impl<'de> de::Visitor<'de> for RequirementVisitor {
    type Value = Requirement;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a boolean or string value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Self::from_bool(v)
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Self::from_string(v)
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut values = BTreeSet::new();
        while let Some(next) = seq.next_element()? {
            values.insert(next);
        }
        Ok(Requirement::Any(values))
    }
}

impl<'de> Deserialize<'de> for Requirement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RequirementVisitor)
    }
}

/// Value assigned to an option
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Boolean(bool),
    Text(String),
}

struct ValueVisitor;

impl<'de> de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a boolean or string value")
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
        Ok(Value::Boolean(v))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_owned()))
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Value::Text(v))
    }

    fn visit_i8<E: de::Error>(self, v: i8) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_i16<E: de::Error>(self, v: i16) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_u8<E: de::Error>(self, v: u8) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_u16<E: de::Error>(self, v: u16) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
        Ok(Value::Text(v.to_string()))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Setting a set of options to particular values
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Setting(#[serde(default)] BTreeMap<FlagId, Value>);

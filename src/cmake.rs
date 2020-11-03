//! Wrapper for invocations of CMake

use crate::{Merge, MergeId, NameRef, Named};
use anyhow::{bail, Result};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::iter::FromIterator;
use std::process::Command;

/// Definition of a configuration option
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Flag {
    description: String,
    /// Flag is passed to CMake
    #[serde(default)]
    variable: Option<String>,
    #[serde(default)]
    requires: BTreeSet<BTreeMap<FlagId, Requirement>>,
}

impl Merge for Flag {
    fn merge(&mut self, other: Self) {
        self.variable.merge(other.variable);
        self.requires.merge(other.requires);
    }
}

impl Named for Flag {
    type Id = FlagId;
}

impl Flag {
    /// Check that a flag can be set to the given value
    pub fn validate(self_ref: NameRef<Self>, setting: &Setting, value: &Value) -> Result<()> {
        if self_ref.requires.len() > 0 {
            match value {
                Value::Boolean(true) => Self::check_requirements(self_ref, setting),
                Value::Boolean(false) => Ok(()),
                _ => {
                    bail!(
                        "Cannot set flag {} with requirements to non-boolean value: {}",
                        self_ref.name(),
                        value
                    );
                }
            }
        } else {
            Ok(())
        }
    }

    /// Check that requirements are met in a given setting for the flag to be set to true
    fn check_requirements(self_ref: NameRef<Self>, setting: &Setting) -> Result<()> {
        let satisfied = self_ref.requires.iter().any(|required| {
            required
                .iter()
                .all(|(flag, requirement)| requirement.check(&setting.flag(flag)))
        });

        if !satisfied {
            bail!(
                "None of the requirement sets for the flag {} could be satisfied",
                self_ref.name()
            );
        } else {
            Ok(())
        }
    }

    /// Set the CMake flag for a build directory
    pub fn cmake_flag(&self, command: &mut Command, value: &Value) {
        if let Some(variable) = &self.variable {
            command.arg(format!("-D{}={}", variable, value.cmake_str()));
        }
    }
}

/// Identifier of an option that can be supplied to CMake
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FlagId(String);

impl fmt::Display for FlagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl From<String> for FlagId {
    fn from(s: String) -> Self {
        FlagId(s)
    }
}

impl From<&str> for FlagId {
    fn from(s: &str) -> Self {
        FlagId(s.to_owned())
    }
}

impl AsRef<str> for FlagId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// A required setting for a particular flag
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum Requirement {
    /// Requires that a flag be set to a specific value
    Single(Value),
    /// Requires that a flag be set to any of a set of values
    Any(BTreeSet<Value>),
}

impl Requirement {
    fn check(&self, value: &Value) -> bool {
        match self {
            Requirement::Single(required) => value == required,
            Requirement::Any(requirement) => requirement.contains(value),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Value {
    Boolean(bool),
    Text(String),
}

impl Value {
    pub fn is_bool(&self) -> bool {
        match self {
            Value::Boolean(_) => true,
            _ => false,
        }
    }

    fn cmake_str(&self) -> &str {
        match self {
            Value::Boolean(true) => "ON",
            Value::Boolean(false) => "OFF",
            Value::Text(text) => text.as_str(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Boolean(value) => fmt::Display::fmt(value, f),
            Value::Text(value) => fmt::Display::fmt(value, f),
        }
    }
}

impl MergeId for Value {}

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
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Setting(#[serde(default)] BTreeMap<FlagId, Value>);

impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        let mut empty = true;
        for (id, value) in self.0.iter() {
            if empty {
                empty = false;
            } else {
                write!(f, ",")?;
            }
            write!(f, " {}: {}", id, value)?;
        }
        if !empty {
            write!(f, " ")?;
        }
        write!(f, "}}")
    }
}

impl FromIterator<(FlagId, Value)> for Setting {
    fn from_iter<T: IntoIterator<Item = (FlagId, Value)>>(iter: T) -> Self {
        Setting(iter.into_iter().collect())
    }
}

impl Merge for Setting {
    fn merge(&mut self, other: Self) {
        self.0.merge(other.0);
    }
}

impl Setting {
    const PLATFORM_FLAG: &'static str = "platform";
    const KERNEL_PLATFORM_FLAG: &'static str = "kernel-platform";

    /// Get the setting of all of the flags
    pub fn flags(&self) -> impl Iterator<Item = (&FlagId, &Value)> {
        self.0.iter()
    }

    /// Get the setting of a particular flag
    pub fn flag(&self, flag: &FlagId) -> Value {
        self.0.get(flag).cloned().unwrap_or(Value::Boolean(false))
    }

    /// Set a particular setting to a boolean value
    pub fn set_bool(&mut self, flag: impl Into<FlagId>, value: bool) {
        self.0.insert(flag.into(), Value::Boolean(value));
    }

    /// Set a particular setting to a text value
    pub fn set_text(&mut self, flag: impl Into<FlagId>, value: impl AsRef<str>) {
        self.0
            .insert(flag.into(), Value::Text(value.as_ref().to_owned()));
    }

    pub fn set_platform(&mut self, platform: impl AsRef<str>) {
        self.set_text(Self::PLATFORM_FLAG, platform);
    }

    pub fn set_kernel_platform(&mut self, platform: impl AsRef<str>) {
        self.set_text(Self::KERNEL_PLATFORM_FLAG, platform);
    }
}

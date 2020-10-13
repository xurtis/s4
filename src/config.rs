//! Configuration of the tool

use crate::{Flag, Platform, Project, Sel4Architecture, Setting};
use anyhow::Result;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use toml;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
/// Configuration for the whole s4 tool
pub struct Config {
    /// Common flags
    #[serde(default, rename = "flag")]
    flags: BTreeSet<Flag>,
    /// Known platforms
    #[serde(default, rename = "platform")]
    platforms: BTreeSet<Platform>,
    /// Architecture-specific flags
    #[serde(default, rename = "architecture", alias = "arch")]
    architectures: BTreeMap<Sel4Architecture, Setting>,
    /// Known projects
    #[serde(default, rename = "project")]
    porjects: BTreeSet<Project>,
}

impl Config {
    /// The default builtin configuration
    pub const BUILTIN_TOML: &'static [u8] = include_bytes!("config.toml");

    /// Parse the builtin configuration file
    pub fn builtin() -> Result<Self> {
        toml::from_slice(Self::BUILTIN_TOML).map_err(|e| e.into())
    }
}

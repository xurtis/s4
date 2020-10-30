//! Descriptions of projects

use crate::{FlagId, Merge, Named, Setting};
use anyhow::{bail, Error, Result};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    repository: Repository,
    /// Path to the CMake source directory
    #[serde(alias = "source-dir")]
    source_directory: PathBuf,
    /// Flags to make available via the command line when configuring a build directory
    #[serde(alias = "cmdline")]
    command_line: BTreeSet<FlagId>,
    #[serde(flatten)]
    setting: Setting,
}

impl Merge for Project {
    fn merge(&mut self, other: Self) {
        self.command_line.merge(other.command_line);
        self.setting.merge(other.setting);
    }
}

impl Named for Project {
    type Id = ProjectId;
}

/// Identifier of a project
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ProjectId(String);

/// Repository of project
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct Repository(String, String);

impl FromStr for Repository {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string.split("/").collect::<Vec<_>>().as_slice() {
            [organisation, repository] if !repository.ends_with(".git") => {
                Ok(Repository(organisation.to_string(), repository.to_string()))
            }
            _ => bail!("Malformed repository: {}", string),
        }
    }
}

impl TryFrom<String> for Repository {
    type Error = Error;

    fn try_from(string: String) -> Result<Self, Self::Error> {
        string.parse()
    }
}

impl Into<String> for Repository {
    fn into(self) -> String {
        format!("{}", self)
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

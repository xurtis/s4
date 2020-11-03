//! Descriptions of projects

use crate::util::*;
use crate::{Apps, BuildContext, Context, FlagId, Merge, Named, Setting, CACHE_SUBDIR};
use anyhow::{bail, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
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

impl Project {
    pub fn setting(&self) -> &Setting {
        &self.setting
    }
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

impl Project {
    pub const WORKSPACE_DOCKER_DIR: &'static str = "/workspace";
    pub const BUILD_DOCKER_DIR: &'static str = "/build";
    pub const CMAKE_CACHE_FILE: &'static str = "settings.cmake";

    pub fn init(&self, workspace_root: impl AsRef<Path>, apps: &Apps) -> Result<()> {
        in_dir(workspace_root, || {
            if !apps.repo_init(&self.repository)?.success() {
                bail!("Failed to initialise project")
            }
            if !apps.repo().arg("sync").status()?.success() {
                bail!("Failed to sync project")
            }
            Ok(())
        })
    }

    pub fn init_build(
        &self,
        context: &BuildContext,
        apps: &Apps,
        args: impl Fn(&mut Command),
    ) -> Result<Command> {
        let mut command = apps
            .docker()?
            .mount(Self::WORKSPACE_DOCKER_DIR, context.workspace_root())?
            .mount(Self::BUILD_DOCKER_DIR, context.build_root())?
            .work_dir(Self::BUILD_DOCKER_DIR)?
            .run("cmake");

        // Alwayse generate ninja builds
        command.arg("-G").arg("Ninja");

        // Use the workspace cache directory
        command.arg(format!(
            "-DSEL4_CACHE_DIR={}/{}",
            Self::WORKSPACE_DOCKER_DIR,
            CACHE_SUBDIR
        ));

        // Add the command line arguments to be set directly
        args(&mut command);

        // Use the build directory as mapped into docker
        command.arg("-B").arg(Self::BUILD_DOCKER_DIR);

        // Use the source directory as mapped into docker
        let mut source_dir = PathBuf::new();
        source_dir.push(Self::WORKSPACE_DOCKER_DIR);
        source_dir.push(&self.source_directory);
        command.arg("-S").arg(&source_dir);

        // Use the cache file from the source directory
        source_dir.push(Self::CMAKE_CACHE_FILE);
        command.arg("-C").arg(source_dir);

        Ok(command)
    }
}

/// Identifier of a project
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Special project name to use project defined in workspace repository
    const AUTO: &'static str = "auto";

    /// The special project identifier that is used to define a project with its own repository
    pub fn auto() -> Self {
        ProjectId(Self::AUTO.to_owned())
    }
}

impl From<String> for ProjectId {
    fn from(s: String) -> Self {
        ProjectId(s)
    }
}

impl From<&str> for ProjectId {
    fn from(s: &str) -> Self {
        ProjectId(s.to_owned())
    }
}

impl AsRef<str> for ProjectId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

/// Repository of project
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

//! Descriptions of projects

use crate::util::*;
use crate::{Apps, BuildContext, Config, Context, FlagId, Merge, Named, Setting, CACHE_SUBDIR};
use anyhow::{bail, format_err, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::str::FromStr;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Project {
    repository: Repository,
    /// Path to the CMake source directory
    #[serde(alias = "source-dir")]
    source_directory: Option<PathBuf>,
    /// Name of the root server binary
    #[serde(alias = "rootserver")]
    root_server: Option<String>,
    /// Phrase used to indicate the root server has completed
    exit_phrase: Option<String>,
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
        config: &Config,
    ) -> Result<ExitStatus> {
        let mut command = self.cmake(context, apps, config)?;

        // Alwayse generate ninja builds
        command.arg("-G").arg("Ninja");

        // Use the workspace cache directory
        command.arg(format!(
            "-DSEL4_CACHE_DIR={}/{}",
            Self::WORKSPACE_DOCKER_DIR,
            CACHE_SUBDIR
        ));

        // Use the build directory as mapped into docker
        command.arg("-B").arg(Self::BUILD_DOCKER_DIR);

        // Use the source directory as mapped into docker
        let mut source_dir = PathBuf::new();
        source_dir.push(Self::WORKSPACE_DOCKER_DIR);
        let source_directory = self
            .source_directory
            .as_ref()
            .cloned()
            .map(Ok)
            .unwrap_or(context.inferred_source())?;
        source_dir.push(source_directory);
        command.arg("-S").arg(&source_dir);

        // Use the cache file from the source directory
        source_dir.push(Self::CMAKE_CACHE_FILE);
        command.arg("-C").arg(source_dir);

        println!("{:?}", command);
        Ok(command.status()?)
    }

    pub fn update_build(
        &self,
        context: &BuildContext,
        apps: &Apps,
        config: &Config,
    ) -> Result<ExitStatus> {
        let mut command = self.cmake(context, apps, config)?;
        command.arg(Self::BUILD_DOCKER_DIR);
        Ok(command.status()?)
    }

    fn cmake(&self, context: &BuildContext, apps: &Apps, config: &Config) -> Result<Command> {
        // Make sure we can actually build with the given settings
        config.check_setting(context.setting())?;
        context.save()?;

        let mut command = apps
            .docker()?
            .mount(Self::WORKSPACE_DOCKER_DIR, context.workspace_root())?
            .mount(Self::BUILD_DOCKER_DIR, context.build_root())?
            .work_dir(Self::BUILD_DOCKER_DIR)?
            .run("cmake");

        // Add the command line arguments to be set directly
        config.cmake_args(&context.setting(), &mut command);

        Ok(command)
    }

    pub fn mq_run(
        &self,
        context: &BuildContext,
        config: &Config,
        apps: &Apps,
        system: Option<&str>,
    ) -> Result<()> {
        let systems = system
            .map(|sys| Ok(vec![sys.to_owned()]))
            .unwrap_or_else(|| {
                apps.machine_queue_match_system(context.platform(), context.variation())
            })?;

        for system in systems {
            let result = self.try_mq_run(context, config, apps, system)?;

            if result.success() {
                return Ok(());
            }
        }

        bail!("Could not run on any available system");
    }

    fn try_mq_run(
        &self,
        context: &BuildContext,
        config: &Config,
        apps: &Apps,
        system: String,
    ) -> Result<ExitStatus> {
        let mut command = apps.machine_queue()?;
        command.arg("run");
        command.arg("-c").arg(
            self.exit_phrase
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(config.defaults().exit_phrase()),
        );
        command.arg("-s").arg(system);

        if context.architecture().architecture() == crate::X86 {
            command.arg("-f").arg(context.kernel_image_path()?);
        }

        let root_server = self
            .root_server
            .as_ref()
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| context.inferred_root_server())?;
        command.arg("-f").arg(context.image_path(&root_server)?);

        command.current_dir(context.build_root());

        println!("{:?}", command);
        Ok(command.status()?)
    }

    /// Flags that should appear on the command-line
    pub fn command_line_flags(&self) -> impl Iterator<Item = &FlagId> {
        self.command_line.iter()
    }
}

/// Identifier of a project
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProjectId(String);

impl ProjectId {
    /// Special project name to use project defined in workspace repository
    const AUTO: &'static str = "auto";

    /// Name given to projects without an explicit configuration
    pub const UNNAMED: Self = ProjectId(String::new());

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
#[derive(Debug, Default, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

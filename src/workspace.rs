//! Project workspaces

use crate::util::*;
use crate::{
    Apps, Config, Docker, Merge, PlatformId, Project, ProjectId, Sel4Architecture, Setting,
    VariationId,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env::current_dir;
use std::fs::{create_dir_all, read_dir};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Inferred execution context
pub trait Context {
    /// The path of the workspace directory
    fn workspace_root(&self) -> &Path;

    /// The path of the build directory (if in a build directory)
    fn maybe_build_root(&self) -> Option<&Path> {
        None
    }

    /// Obtain only the workspace context
    fn workspace(&self) -> &WorkspaceContext;

    /// The identifier for the project
    fn project(&self) -> &ProjectId;

    /// Create a new build context
    fn create_build(
        self: Box<Self>,
        config: &Config,
        path: &Path,
        platform: PlatformId,
        variation: Option<VariationId>,
        architecture: Sel4Architecture,
        setting: Setting,
    ) -> Result<BuildContext> {
        BuildContext::create(
            config,
            self.workspace(),
            platform,
            variation,
            architecture,
            setting,
            path,
        )
    }

    /// Create docker environment for a context
    fn docker<'c>(&self, apps: &'c Apps) -> Result<Docker<'c>> {
        let mut docker = apps
            .docker()?
            .mount(Project::WORKSPACE_DOCKER_DIR, self.workspace_root())?;

        if let Some(build_root) = self.maybe_build_root() {
            docker = docker.mount(Project::BUILD_DOCKER_DIR, build_root)?;
        }

        Ok(docker)
    }
}

pub fn find_context() -> Result<Option<Box<dyn Context>>> {
    let mut path = current_dir()?;

    while path.parent().is_some() {
        path.push(Build::FILENAME);
        if path.exists() {
            let build: Build = toml_load(&path)?;
            path.pop();
            let mut workspace_root = build.workspace_root.clone();
            workspace_root.push(Workspace::FILENAME);
            let workspace: Workspace = toml_load(&workspace_root)?;
            let build_root = path;
            let mut workspace_root = build_root.clone();
            workspace_root.push(&build.workspace_root);
            let workspace = WorkspaceContext {
                workspace_root,
                workspace,
            };
            let context = Box::new(BuildContext {
                workspace,
                build,
                build_root,
            });
            return Ok(Some(context));
        } else {
            path.pop();
            path.push(Workspace::FILENAME);
            if path.exists() {
                let workspace: Workspace = toml_load(&path)?;
                path.pop();
                let workspace_root = path;
                let context = Box::new(WorkspaceContext {
                    workspace,
                    workspace_root,
                });
                return Ok(Some(context));
            } else {
                path.pop();
            }
        }
    }

    Ok(None)
}

/// Working context
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct WorkspaceContext {
    workspace: Workspace,
    workspace_root: PathBuf,
}

impl Context for WorkspaceContext {
    fn workspace_root(&self) -> &Path {
        self.workspace_root.as_path()
    }

    fn workspace(&self) -> &WorkspaceContext {
        self
    }

    fn project(&self) -> &ProjectId {
        &self.workspace.project
    }
}

/// Directory within the root of a workspace used to cache artifacts
pub const CACHE_SUBDIR: &'static str = ".sel4_cache";

impl WorkspaceContext {
    /// Create a new workspace directory
    pub fn create(project: ProjectId, path: impl AsRef<Path>) -> Result<Self> {
        let workspace = Workspace {
            project,
            builds: BTreeSet::new(),
        };

        let mut workspace_root = path.as_ref().to_owned();
        if workspace_root.is_dir() && !read_dir(&workspace_root)?.count() != 0 {
            bail!(
                "Workspace directory {} is not empty",
                workspace_root.display()
            );
        } else if workspace_root.exists() {
            bail!(
                "Workspace directory path {} already exists",
                workspace_root.display()
            );
        } else {
            create_dir_all(&workspace_root)?;
        }

        // Create a cache directory for the workspace
        workspace_root.push(CACHE_SUBDIR);
        create_dir_all(&workspace_root)?;
        workspace_root.pop();

        workspace_root.push(Workspace::FILENAME);
        toml_save(&workspace, &workspace_root)?;
        workspace_root.pop();

        Ok(WorkspaceContext {
            workspace,
            workspace_root,
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let mut workspace_root = path.as_ref().to_owned();
        workspace_root.push(Workspace::FILENAME);
        let workspace = toml_load(&workspace_root)?;
        workspace_root.pop();

        Ok(WorkspaceContext {
            workspace,
            workspace_root,
        })
    }

    /// Get all of the build contexts for a given workspace
    pub fn builds<'w>(&'w self) -> impl Iterator<Item = Result<BuildContext>> + 'w {
        self.workspace.builds.iter().flat_map(move |build| {
            let mut path = self.workspace_root.clone();
            path.push(build);
            // Skip non-existing builds
            let build = if path.exists() {
                Some(self.load_build(path))
            } else {
                None
            };
            build.into_iter()
        })
    }

    /// Load an existing build directory
    fn load_build(&self, path: impl AsRef<Path>) -> Result<BuildContext> {
        BuildContext::load(self, path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct BuildContext {
    workspace: WorkspaceContext,
    build: Build,
    build_root: PathBuf,
}

impl Context for BuildContext {
    fn workspace_root(&self) -> &Path {
        self.workspace.workspace_root()
    }

    fn maybe_build_root(&self) -> Option<&Path> {
        Some(&self.build_root)
    }

    fn project(&self) -> &ProjectId {
        self.workspace.project()
    }

    fn workspace(&self) -> &WorkspaceContext {
        &self.workspace
    }
}

impl BuildContext {
    /// Create a new build directory for a workspace
    pub fn create(
        config: &Config,
        workspace: &WorkspaceContext,
        platform: PlatformId,
        variation: Option<VariationId>,
        architecture: Sel4Architecture,
        added_setting: Setting,
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        let WorkspaceContext {
            mut workspace,
            mut workspace_root,
            ..
        } = workspace.clone();

        let mut build_root = path.as_ref().to_owned();
        if build_root.is_dir() && !read_dir(&build_root)?.count() != 0 {
            bail!("Build directory {} is not empty", build_root.display());
        } else if build_root.exists() {
            bail!(
                "Build directory path {} already exists",
                build_root.display()
            );
        } else {
            create_dir_all(&build_root)?;
        }

        // Construct all settings
        let mut setting = config.platform_setting(
            &workspace.project,
            &platform,
            variation.as_ref(),
            architecture,
        )?;
        setting.merge(added_setting);

        // Get relative path to workspace root
        let build = Build::new(
            relative_path(&build_root, &workspace_root)?,
            platform,
            variation,
            architecture,
            setting,
        );
        workspace
            .builds
            .insert(relative_path(&workspace_root, &build_root)?);

        build_root.push(Build::FILENAME);
        toml_save(&build, &build_root)?;
        build_root.pop();

        workspace_root.push(Workspace::FILENAME);
        toml_save(&workspace, &workspace_root)?;
        workspace_root.pop();

        let workspace = WorkspaceContext {
            workspace,
            workspace_root,
        };

        Ok(BuildContext {
            workspace,
            build,
            build_root,
        })
    }

    /// Load an existing build directory with a given workspace
    pub fn load(workspace: &WorkspaceContext, path: impl AsRef<Path>) -> Result<Self> {
        let workspace = workspace.clone();
        let mut build_root = path.as_ref().to_owned();

        build_root.push(Build::FILENAME);
        let build = toml_load(&build_root)?;
        build_root.pop();

        Ok(BuildContext {
            workspace,
            build,
            build_root,
        })
    }

    pub fn build_root(&self) -> &Path {
        &self.build_root
    }

    pub fn ninja(&self, apps: &Apps) -> Result<Command> {
        let command = self
            .docker(apps)?
            .work_dir(Project::BUILD_DOCKER_DIR)?
            .run("ninja");
        Ok(command)
    }

    pub fn setting(&self) -> &Setting {
        &self.build.setting
    }

    pub fn setting_mut(&mut self) -> &mut Setting {
        &mut self.build.setting
    }

    pub fn update_setting(&mut self, setting: Setting) {
        self.build.setting.merge(setting);
    }

    pub fn save(&self) -> Result<()> {
        let mut build_root = self.build_root.clone();
        build_root.push(Build::FILENAME);
        toml_save(&self.build, &build_root)?;
        Ok(())
    }

    pub fn platform(&self) -> &PlatformId {
        &self.build.platform
    }

    pub fn variation(&self) -> Option<&VariationId> {
        self.build.variation.as_ref()
    }

    pub fn architecture(&self) -> Sel4Architecture {
        self.build.architecture
    }
}

/// Workspace directory for a project
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Workspace {
    /// Project associated with workspace
    project: ProjectId,
    /// Build directories
    builds: BTreeSet<PathBuf>,
}

impl Workspace {
    /// Filename used to indicate a workspace directory
    pub const FILENAME: &'static str = ".s4-workspace.toml";
}

/// Build directory configuration
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Build {
    /// Root directory of workspace
    workspace_root: PathBuf,
    /// Configured platform
    #[serde(rename = "build-platform")]
    platform: PlatformId,
    /// Configure variation (if any)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "build-variation"
    )]
    variation: Option<VariationId>,
    /// Configured architecture
    #[serde(rename = "build-architecture")]
    architecture: Sel4Architecture,
    /// Settings for the build directory
    #[serde(flatten)]
    setting: Setting,
}

impl Build {
    /// Filename used to indicate a build directory
    pub const FILENAME: &'static str = ".s4-build.toml";

    fn new(
        workspace_root: PathBuf,
        platform: PlatformId,
        variation: Option<VariationId>,
        architecture: Sel4Architecture,
        setting: Setting,
    ) -> Self {
        Build {
            workspace_root,
            platform,
            variation,
            architecture,
            setting,
        }
    }
}

//! Project workspaces

use crate::util::*;
use crate::{
    Apps, Config, Docker, Flag, Merge, NamedMap, PlatformId, Project, ProjectId, Sel4Architecture,
    Setting, Type, VariationId,
};
use anyhow::{bail, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env::current_dir;
use std::fs::{create_dir_all, read_dir, File};
use std::io::{BufRead, BufReader};
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

    fn easy_settings(&self) -> Result<NamedMap<Flag>> {
        let mut flags = NamedMap::default();

        // Regex to match a setting
        let setting_match = Regex::new(
            "^set\\((?P<variable>[A-Za-z][A-Za-z0-9_]*)( [^ ]+){2} (?P<type>[A-Z]+) \"(?P<description>[^\"]*)\"\\)$",
        )?;

        let mut easy_settings = self.workspace_root().to_owned();
        easy_settings.push(Workspace::EASY_SETTINGS);

        // No flags if no file
        if !easy_settings.is_file() {
            return Ok(flags);
        }

        let easy_settings = BufReader::new(File::open(easy_settings)?);

        for line in easy_settings.lines() {
            let line = line?;
            if let Some(matches) = setting_match.captures(line.trim()) {
                let variable = &matches["variable"];
                let description = &matches["description"];
                let identifier: String = if variable.chars().all(|c| c.is_uppercase() || c == '_') {
                    // SCREAMING_SNAKE_CASE
                    variable
                        .chars()
                        .flat_map(|c| {
                            if c == '_' {
                                '-'.to_lowercase()
                            } else {
                                c.to_lowercase()
                            }
                        })
                        .collect()
                } else {
                    // PascalCase
                    let mut first = true;
                    variable
                        .chars()
                        .flat_map(move |c| {
                            if c.is_uppercase() && !first {
                                vec!['-'].into_iter().chain(c.to_lowercase())
                            } else {
                                first = false;
                                vec![].into_iter().chain(c.to_lowercase())
                            }
                        })
                        .collect()
                };
                let type_ = match &matches["type"] {
                    "STRING" => Some(Type::Text),
                    "BOOL" => Some(Type::Boolean),
                    _ => None,
                };

                flags.insert(
                    identifier.into(),
                    Flag::new(description, Some(variable), type_),
                );
            }
        }

        Ok(flags)
    }

    /// Infer the path to the source directory
    fn inferred_source(&self) -> Result<PathBuf> {
        let workspace_root = self.workspace_root().canonicalize()?;
        let mut hint_path = workspace_root.clone();
        hint_path.push(Workspace::EASY_SETTINGS);

        if hint_path.exists() {
            hint_path = hint_path.canonicalize()?;
            hint_path.pop();
            relative_path(workspace_root, hint_path)
        } else {
            bail!("Could not infer source directory");
        }
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

    pub fn kernel_image_path(&self) -> Result<PathBuf> {
        self.in_image_dir(format!("kernel-{}", self.plat_image_name()))
    }

    pub fn image_path(&self, root_server: impl AsRef<str>) -> Result<PathBuf> {
        self.in_image_dir(format!(
            "{}-image-{}",
            root_server.as_ref(),
            self.plat_image_name()
        ))
    }

    fn plat_image_name(&self) -> String {
        match self.architecture().architecture() {
            crate::X86 => format!("{}-{}", self.architecture(), self.platform().as_ref()),
            architecture => format!("{}-{}", architecture, self.platform().as_ref()),
        }
    }

    fn in_image_dir(&self, filename: impl AsRef<Path>) -> Result<PathBuf> {
        let mut path = PathBuf::new();
        path.push("images");
        path.push(filename);

        in_dir(&self.build_root, || {
            if path.exists() {
                Ok(path)
            } else {
                bail!("Image file missing: {}", path.display())
            }
        })
    }

    pub fn inferred_root_server(&self) -> Result<String> {
        in_dir(&self.build_root, || {
            if Path::new("images").is_dir() {
                let image_tail = format!("-image-{}", self.plat_image_name());
                for file in read_dir("images")? {
                    let file = file?;
                    if let Some(name) = file.file_name().to_str() {
                        if name.ends_with(&image_tail) {
                            return Ok(name[..name.len() - image_tail.len()].to_owned());
                        }
                    }
                }
                bail!("no rootserver image in images directory")
            } else {
                bail!("images directory is missing")
            }
        })
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
    const FILENAME: &'static str = ".s4-workspace.toml";

    /// Hint file used to indicate the location of the project source directory
    const EASY_SETTINGS: &'static str = "easy-settings.cmake";
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

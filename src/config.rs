//! Configuration of the tool

use crate::{Flag, Platform, Project, Sel4Architecture, Setting};
use anyhow::Result;
use dirs::{config_dir, home_dir};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use toml;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
/// Configuration for the whole s4 tool
pub struct Config {
    /// Global default configuration
    #[serde(flatten)]
    defaults: Defaults,
    /// Common flags
    #[serde(default, rename = "flag")]
    flags: NamedMap<Flag>,
    /// Known platforms
    #[serde(default, rename = "platform")]
    platforms: NamedMap<Platform>,
    /// Architecture-specific flags
    #[serde(default, rename = "architecture", alias = "arch")]
    architectures: BTreeMap<Sel4Architecture, Setting>,
    /// Known projects
    #[serde(default, rename = "project")]
    projects: NamedMap<Project>,
}

impl Config {
    /// The default builtin configuration
    const BUILTIN_TOML: &'static [u8] = include_bytes!("config.toml");

    /// Configuration for s4
    const CONFIG_FILES: &'static [&'static str] = &[".s4", ".s4.toml", "s4.toml"];

    /// Parse the builtin configuration file
    pub fn builtin() -> Result<Self> {
        toml::from_slice(Self::BUILTIN_TOML).map_err(|e| e.into())
    }

    /// Load the configuration
    pub fn load() -> Result<Self> {
        let mut configuration = Self::builtin()?;

        fn all_config_files(directory: PathBuf) -> impl Iterator<Item = PathBuf> {
            Config::CONFIG_FILES.iter().map(move |file| {
                let mut path = directory.clone();
                path.push(file);
                path
            })
        };

        home_dir()
            .into_iter()
            .chain(config_dir().into_iter())
            .flat_map(all_config_files)
            .try_for_each(|path| -> Result<()> {
                if path.exists() {
                    configuration.merge(toml_load(path)?);
                }
                Ok(())
            })?;

        Ok(configuration)
    }

    /// Get the defaults from the config
    pub fn defaults(&self) -> &Defaults {
        &self.defaults
    }
}

impl Merge for Config {
    fn merge(&mut self, other: Self) {
        self.defaults.merge(other.defaults);
        self.flags.merge(other.flags);
        self.platforms.merge(other.platforms);
        self.architectures.merge(other.architectures);
        self.projects.merge(other.projects);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Defaults {
    /// Server to use for repo manifests
    git_server: Option<String>,
    /// Docker image for build tools
    docker_image: Option<String>,
    /// URL to download repo script
    repo_url: Option<String>,
    /// Git branch to check out with repo
    repo_branch: Option<String>,
    /// Repo manifest file to check out
    repo_manifest: Option<String>,
}

impl Defaults {
    /// Default git server for manifests
    const GIT_SERVER: &'static str = "https://github.com";

    /// Default docker image for build tools
    const DOCKER_IMAGE: &'static str = "docker.io/trustworthysystems/camkes-riscv";

    /// Default URL to download repo
    const REPO_URL: &'static str = "https://storage.googleapis.com/git-repo-downloads/repo";

    /// Get the git server base URL
    pub fn git_server(&self) -> &str {
        option_fallback(&self.git_server, Self::GIT_SERVER)
    }

    /// Docker image to execute for build tools
    pub fn docker_image(&self) -> &str {
        option_fallback(&self.docker_image, Self::DOCKER_IMAGE)
    }

    /// URL to download repo
    pub fn repo_url(&self) -> &str {
        option_fallback(&self.repo_url, Self::REPO_URL)
    }

    /// Branch to check out for repo
    pub fn repo_branch(&self) -> Option<&str> {
        option_ref(&self.repo_branch)
    }

    /// Manifest to checkou out for repo
    pub fn repo_manifest(&self) -> Option<&str> {
        option_ref(&self.repo_manifest)
    }
}

impl Merge for Defaults {
    fn merge(&mut self, other: Self) {
        self.git_server.merge(other.git_server);
        self.docker_image.merge(other.docker_image);
        self.repo_url.merge(other.repo_url);
        self.repo_branch.merge(other.repo_branch);
        self.repo_manifest.merge(other.repo_manifest);
    }
}

/// Make reference option
fn option_ref<T: AsRef<R>, R: ?Sized>(option: &Option<T>) -> Option<&R> {
    option.as_ref().map(|s| s.as_ref())
}

/// Use a fallback reference if option is not set
fn option_fallback<'t, T: AsRef<R>, R: ?Sized>(option: &'t Option<T>, fallback: &'t R) -> &'t R {
    option_ref(option).unwrap_or(fallback)
}

/// Merge instances of configuration structures together
pub trait Merge<Other = Self> {
    /// Update the current structure using a new instance
    fn merge(&mut self, other: Other);

    /// Update the current structure using a potential new instance
    fn maybe_merge(&mut self, other: Option<Other>) {
        if let Some(other) = other {
            self.merge(other);
        }
    }
}

impl<T: Clone + Merge<T>, K: Ord> Merge for BTreeMap<K, T> {
    fn merge(&mut self, other: BTreeMap<K, T>) {
        for (key, other) in other.into_iter() {
            if self.contains_key(&key) {
                self.get_mut(&key).unwrap().merge(other);
            } else {
                self.insert(key, other);
            }
        }
    }
}

impl<T: Ord> Merge for BTreeSet<T> {
    fn merge(&mut self, other: BTreeSet<T>) {
        for value in other {
            self.insert(value);
        }
    }
}

impl<T> Merge for Option<T> {
    fn merge(&mut self, other: Option<T>) {
        if let Some(other) = other {
            *self = Some(other)
        }
    }
}

/// Items that merge to themselves
pub trait MergeId {}

impl MergeId for String {}

impl<T: MergeId> Merge for T {
    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

/// Items that have a named identifier
pub trait Named {
    type Id;
}

pub struct NameRef<'t, T: Named> {
    inner: &'t T,
    name: &'t T::Id,
}

impl<'t, T: Named> NameRef<'t, T> {
    pub fn new(inner: &'t T, name: &'t T::Id) -> Self {
        NameRef { inner, name }
    }

    pub fn name(&self) -> &T::Id {
        self.name
    }
}

impl<'t, T: Named> Deref for NameRef<'t, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

/// Mapping of name identifiers to items
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(transparent)]
pub struct NamedMap<T: Named>
where
    T::Id: Ord,
    T::Id: for<'nde> Deserialize<'nde>,
{
    map: BTreeMap<T::Id, T>,
}

impl<T: Named> Default for NamedMap<T>
where
    T::Id: Ord,
    T::Id: for<'nde> Deserialize<'nde>,
{
    fn default() -> Self {
        NamedMap {
            map: BTreeMap::default(),
        }
    }
}

impl<T: Named> NamedMap<T>
where
    T::Id: Ord,
    T::Id: for<'nde> Deserialize<'nde>,
{
    /// Get an object with its name from the map
    pub fn get(&self, index: impl AsRef<T::Id>) -> Option<NameRef<T>> {
        let index = index.as_ref();
        self.map
            .get_key_value(index)
            .map(move |(k, v)| NameRef::new(v, k))
    }

    /// Get all of the objects with names from the map
    pub fn all(&self) -> impl Iterator<Item = NameRef<T>> {
        self.map.iter().map(|(k, v)| NameRef::new(v, k))
    }
}

impl<T: Named> Merge for NamedMap<T>
where
    T::Id: Ord,
    T::Id: for<'nde> Deserialize<'nde>,
    T: Clone + Merge<T>,
{
    fn merge(&mut self, other: Self) {
        self.map.merge(other.map)
    }
}

fn toml_load<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let mut data = Vec::new();
    File::open(path.as_ref())?.read_to_end(&mut data)?;
    toml::from_slice(&data).map_err(|e| e.into())
}

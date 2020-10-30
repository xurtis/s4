//! Configuration of the tool

use crate::{Flag, Platform, Project, Sel4Architecture, Setting};
use anyhow::Result;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use toml;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
/// Configuration for the whole s4 tool
pub struct Config {
    /// Global default configuration
    #[serde(flatten)]
    defaults: Defaults,
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
    projects: BTreeSet<Project>,
}

impl Config {
    /// The default builtin configuration
    pub const BUILTIN_TOML: &'static [u8] = include_bytes!("config.toml");

    /// Parse the builtin configuration file
    pub fn builtin() -> Result<Self> {
        toml::from_slice(Self::BUILTIN_TOML).map_err(|e| e.into())
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

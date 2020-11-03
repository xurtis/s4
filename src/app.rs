//! Hooks into finding and running command-line applications

use crate::{Defaults, Repository};
use anyhow::{bail, format_err, Result};
use reqwest::blocking::get;
use std::collections::BTreeMap;
use std::env::{current_dir, var};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::copy;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use users::{get_current_username, get_effective_gid, get_effective_uid};

/// Wrapper around command line apps called by s4
pub struct Apps<'d> {
    defaults: &'d Defaults,
    /// Path to repo executable
    repo: PathBuf,
    /// Path to docker executable
    docker: PathBuf,
    /// Docker is actually podman
    docker_impl: DockerImpl,
}

impl<'d> Apps<'d> {
    /// Try and find all dependent apps
    pub fn try_new(defaults: &'d Defaults) -> Result<Self> {
        let repo = find_or_download("repo", defaults.repo_url())?;
        let docker = find_app_path("docker")
            .ok_or(format_err!("docker or podman-docker must be installed"))?;

        let docker_version = Command::new(&docker).arg("--version").output()?.stdout;
        let docker_version = String::from_utf8(docker_version)?;
        let docker_impl = if docker_version.contains("podman") {
            Podman
        } else {
            Docker
        };

        Ok(Apps {
            defaults,
            repo,
            docker,
            docker_impl,
        })
    }

    /// Create an invocation of the repo command
    pub fn repo(&self) -> Command {
        Command::new(&self.repo)
    }

    /// Create a new invocation of the repo init command
    pub fn repo_init(&self, project: &Repository) -> Result<ExitStatus> {
        let mut repo = self.repo();

        let url = self.defaults.git_repo_url(project);

        repo.arg("init");
        repo.arg("--manifest-url").arg(url);

        if let Some(branch) = self.defaults.repo_branch() {
            repo.arg("--manifest-branch").arg(branch);
        }

        if let Some(manifest) = self.defaults.repo_manifest() {
            repo.arg("--manifest-name").arg(manifest);
        }

        Ok(repo.status()?)
    }

    /// Create an invocation of the docker command
    pub fn docker(&'d self) -> Result<Docker<'d>> {
        Docker::new(self)
    }

    /// Check if docker is actually podman
    pub fn docker_impl(&self) -> DockerImpl {
        self.docker_impl
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerImpl {
    Docker,
    Podman,
}
pub use DockerImpl::*;

/// Find an app in the path or maybe download a copy of the script
fn find_or_maybe_download(app: impl AsRef<Path>, url: Option<&str>) -> Result<Option<PathBuf>> {
    match url {
        Some(url) => find_or_download(app, url).map(Some),
        None => Ok(find_app_path(app)),
    }
}

pub struct Docker<'a> {
    /// Reference to app config
    apps: &'a Apps<'a>,
    /// Addittional mounts to add to the system
    mounts: BTreeMap<PathBuf, PathBuf>,
    /// The path to the working directory relative to the host directory
    work_dir: PathBuf,
}

impl<'a> Docker<'a> {
    const HOST_DIR: &'static str = "/host";

    /// Create a new docker command invocation
    pub fn new(apps: &'a Apps<'a>) -> Result<Self> {
        let mut mounts = BTreeMap::new();
        mounts.insert(Self::HOST_DIR.into(), current_dir()?.canonicalize()?);
        let docker = Docker {
            apps,
            mounts,
            work_dir: Self::HOST_DIR.into(),
        };
        Ok(docker)
    }

    /// Set the host path for the command
    pub fn mount(mut self, internal: impl AsRef<Path>, external: impl AsRef<Path>) -> Result<Self> {
        self.mounts.insert(
            internal.as_ref().to_owned(),
            external.as_ref().canonicalize()?,
        );
        Ok(self)
    }

    pub fn host_dir(self, external: impl AsRef<Path>) -> Result<Self> {
        self.mount(Self::HOST_DIR, external)
    }

    /// Set the working directory for the command
    pub fn work_dir(mut self, path: impl AsRef<Path>) -> Result<Self> {
        self.work_dir = path.as_ref().to_owned();
        Ok(self)
    }

    /// Run a command in an image
    pub fn run(self, program: impl AsRef<OsStr>) -> Command {
        let mut command = self.command();
        command
            .arg("run")
            .args(&["-it", "--rm"])
            .args(&["--hostname", "s4"])
            .args(&["--volume", "/etc/localtime:/etc/localtime:ro"]);
        match self.apps.docker_impl {
            Podman => command.arg("--userns=keep-id"),
            Docker => command.args(&[
                "--user".to_owned(),
                format!("{}:{}", get_effective_uid(), get_effective_gid()),
            ]),
        };
        for (internal, external) in self.mounts.into_iter() {
            command
                .arg("--volume")
                .arg(format!("{}:{}:z", external.display(), internal.display()));
        }
        command.arg("--workdir").arg(Self::host_path(self.work_dir));
        command.arg(self.apps.defaults.docker_image());
        command.arg(program);
        command
    }

    /// Update the docker image
    pub fn update(self) -> Result<()> {
        let mut command = self.command();
        if !command
            .arg("pull")
            .arg(self.apps.defaults.docker_image())
            .status()?
            .success()
        {
            bail!(
                "Failued to update docker image: {}",
                self.apps.defaults.docker_image()
            );
        }
        Ok(())
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.apps.docker);
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        command
    }

    fn host_path(path: impl AsRef<Path>) -> PathBuf {
        Path::new(Self::HOST_DIR).join(path)
    }
}

/// Find a app somewhere in the current app path
fn find_app_path(app: impl AsRef<Path>) -> Option<PathBuf> {
    let path = var("PATH").ok()?;

    // We assume if we find a matching application that it is executable
    path.split(':')
        .map(Path::new)
        .map(|path| {
            let mut path = path.to_owned();
            path.push(app.as_ref());
            path
        })
        .filter(|path| path.exists())
        .next()
}

/// Find an app somewhere in the path or download a script from a URL
fn find_or_download(app: impl AsRef<Path>, url: &str) -> Result<PathBuf> {
    if let Some(path) = find_app_path(&app) {
        Ok(path)
    } else {
        let path = tmp_app_path(&app)?;
        if !path.exists() {
            let mut binary = get(url)?;
            if !binary.status().is_success() {
                bail!(
                    "Could not download {} from {}: {}",
                    app.as_ref().display(),
                    url,
                    binary.status()
                );
            }
            let mut dest = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .mode(0o755)
                .open(&path)?;
            copy(&mut binary, &mut dest)?;
        }
        Ok(path)
    }
}

/// A path for a temporary copy of a script
fn tmp_app_path(app: impl AsRef<Path>) -> Result<PathBuf> {
    let mut path = PathBuf::new();
    path.push(var("TMPDIR").ok().unwrap_or("/tmp".to_owned()));
    let username = get_current_username().and_then(|username| username.into_string().ok());
    let filename = app
        .as_ref()
        .file_name()
        .and_then(|f| f.to_str())
        .ok_or(format_err!("Invalid app name: {}", app.as_ref().display()))?;
    if let Some(username) = username {
        path.push(format!("{}-s4-{}", username, filename));
    } else {
        path.push(format!("{}-s4-{}", get_effective_uid(), filename));
    }
    Ok(path)
}

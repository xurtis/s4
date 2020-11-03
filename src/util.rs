//! Utilities for library

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env::{current_dir, set_current_dir};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(crate) fn toml_load<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let mut data = Vec::new();
    File::open(path.as_ref())?.read_to_end(&mut data)?;
    toml::from_slice(&data).map_err(|e| e.into())
}

pub(crate) fn toml_save<T: Serialize>(data: &T, path: impl AsRef<Path>) -> Result<()> {
    File::create(path.as_ref())?
        .write(toml::to_vec(&data)?.as_slice())
        .map_err(|e| e.into())
        .map(|_| ())
}

pub(crate) fn in_dir<T>(path: impl AsRef<Path>, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let current_dir = current_dir()?;
    set_current_dir(path.as_ref())?;
    let result = f();
    set_current_dir(current_dir)?;
    result
}

pub(crate) fn relative_path(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<PathBuf> {
    let to = to.as_ref().canonicalize()?;
    let mut to = to.components();
    let from = from.as_ref().canonicalize()?;
    let mut from = from.components();

    let mut to_next = to.next();
    let mut from_next = from.next();

    while from_next.is_some() && to_next == from_next {
        to_next = to.next();
        from_next = from.next();
    }

    let mut result = PathBuf::new();

    while let Some(_) = from_next {
        result.push("..");
        from_next = from.next();
    }

    while let Some(next) = to_next {
        result.push(next);
        to_next = to.next();
    }

    Ok(result)
}

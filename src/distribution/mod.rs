// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Distribution CLI commands: add, list, remove, update.

// Placeholder modules; dead-code warnings are temporary until each command is
// implemented in its milestone.
#![allow(dead_code)]

use std::fmt;

pub mod add;
pub mod install;
pub mod list;
pub mod network;
pub mod remove;
pub mod source;
pub mod update;

use std::path::PathBuf;

use crate::types::ProjectError;

/// Locates the nearest project root by walking up from the current directory
/// looking for `skillprism.yaml`.
pub fn find_project_root() -> Result<PathBuf, ProjectError> {
    let cwd = std::env::current_dir().map_err(|e| ProjectError::ConfigRead {
        path: ".".to_string(),
        source: e,
    })?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join("skillprism.yaml").exists() {
            return Ok(dir.to_path_buf());
        }
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            return Err(ProjectError::ConfigNotFound {
                path: cwd.join("skillprism.yaml").to_string_lossy().to_string(),
            });
        }
    }
}

/// Error type for distribution CLI commands that carries an explicit process
/// exit code separate from the wrapped diagnostic report.
#[derive(Debug)]
pub enum CommandError {
    /// A usage error (e.g. invalid flags, missing project). Exits with code 2.
    Usage(miette::Report),
    /// A runtime error. Exits with code 1.
    Runtime(miette::Report),
}

impl CommandError {
    /// Returns the exit code the CLI should use for this error.
    pub const fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Runtime(_) => 1,
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(r) | Self::Runtime(r) => fmt::Display::fmt(r, f),
        }
    }
}

impl std::error::Error for CommandError {}

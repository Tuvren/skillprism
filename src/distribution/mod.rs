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

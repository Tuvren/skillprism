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

use std::fmt;
use std::path::{Path, PathBuf};

use crate::state::{InstallScope, InstalledSkill};
use crate::types::ProjectError;

mod add;
mod detect;
mod install;
mod list;
mod network;
mod remove;
mod source;
mod update;

// Curated crate-facing API: expose only the command entrypoints and the clap
// arg type. Submodules stay private per the Module Exports guideline; siblings
// reach shared helpers via `super::`.
pub use add::{InstallScopeArg, run_add};
pub use list::run_list;
pub use remove::run_remove;
pub use update::run_update;

/// Parses a comma-separated harness list (e.g. `--harnesses claude,opencode`)
/// into trimmed, non-empty ids. Shared by add/list/remove/update so the parsing
/// rule stays in one place.
///
/// This does not apply skills CLI aliases. Call
/// [`canonical_harness_arg`] once a registry is available so a user harness
/// whose id is `droid` or `claude-code` is not rewritten to the built-in.
pub fn parse_harness_list(raw: &str) -> Vec<String> {
    normalize_harness_ids(raw.split(','))
}

/// Trims harness ids, dropping blanks and duplicates while preserving
/// first-seen order. Does not apply built-in aliases.
pub fn normalize_harness_ids<I, S>(ids: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = Vec::new();
    for id in ids {
        let trimmed = id.as_ref().trim();
        if trimmed.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Resolves aliases in one harness argument against the project registry.
///
/// Returns `None` when the argument is absent or names nothing. An exact
/// registered id wins over `claude-code` → `claude` and `droid` → `factory`.
pub fn canonical_harness_arg(raw: Option<String>) -> Result<Option<String>, miette::Report> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let root = find_project_root().ok();
    let registry =
        install::build_registry_for_harnesses(root.as_deref()).map_err(miette::Report::new)?;
    let ids = registry.selected_ids(raw.split(','));
    if ids.is_empty() {
        Ok(None)
    } else {
        Ok(Some(ids.join(",")))
    }
}

/// Merges `--harnesses` and repeatable `--agent` values into one canonical list.
///
/// Returns `None` when neither flag named a harness, so callers still prompt.
pub fn combine_harness_args(harnesses: Option<&str>, agents: &[String]) -> Option<String> {
    let mut raw = Vec::new();
    if let Some(list) = harnesses {
        raw.extend(list.split(',').map(ToString::to_string));
    }
    raw.extend(agents.iter().cloned());
    let ids = normalize_harness_ids(raw);
    if ids.is_empty() {
        None
    } else {
        Some(ids.join(","))
    }
}

/// Returns whether a skill passes the shared scope + harness filter used by
/// `list` and `update`: it must be in the requested scope (if any) and expose at
/// least one of the requested harnesses (if any).
pub fn scope_harness_matches(
    skill: &InstalledSkill,
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
    active_project_root: Option<&Path>,
) -> bool {
    let scope_matches = target.is_none_or(|t| InstallScope::from(t) == skill.scope);
    if !scope_matches {
        return false;
    }
    if !skill.matches_project_root(active_project_root) {
        return false;
    }
    harnesses.is_none_or(|h| {
        let wanted = parse_harness_list(h);
        wanted.is_empty()
            || skill
                .harnesses
                .iter()
                .any(|installed| wanted.contains(installed))
    })
}

/// Locates the nearest project root by walking up from the current directory
/// looking for `skillprism.yaml`.
pub fn find_project_root() -> Result<PathBuf, ProjectError> {
    let cwd = std::env::current_dir().map_err(|e| ProjectError::ConfigRead {
        path: ".".to_string(),
        source: e,
    })?;
    find_project_root_from(&cwd)
}

/// Locates the nearest project root by walking up from `start` directory
/// looking for `skillprism.yaml`.
pub fn find_project_root_from(start: &std::path::Path) -> Result<PathBuf, ProjectError> {
    let mut dir = start;
    loop {
        if dir.join("skillprism.yaml").exists() {
            return Ok(dir.to_path_buf());
        }
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            return Err(ProjectError::ConfigNotFound {
                path: start.join("skillprism.yaml").to_string_lossy().to_string(),
            });
        }
    }
}

/// Error type for distribution CLI commands that carries an explicit process
/// exit code separate from the wrapped diagnostic report.
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

// Forward `Debug` to the inner report so `eprintln!("{e:?}")` renders miette's
// graphical diagnostic directly, without a `Usage(...)`/`Runtime(...)` wrapper
// leaking into user-facing stderr. The exit code is derived separately.
impl fmt::Debug for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(r) | Self::Runtime(r) => fmt::Debug::fmt(r, f),
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

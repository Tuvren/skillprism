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

//! `skillprism add` command implementation.

use std::path::{Path, PathBuf};

use clap::ValueEnum;
use miette::IntoDiagnostic;

use crate::loader::ProjectLoader;
use crate::state::{InstallScope, StateStore};
use crate::types::ProjectError;

use super::CommandError;
use super::install::{InstallContext, InstallError, install_source};
use super::source::parse_source;

/// Target scope for distribution install commands (`project` or `user`).
///
/// This is distinct from [`crate::cli::TargetScope`] because `dist` is not an
/// install target.
#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum InstallScopeArg {
    /// Install into the current project.
    Project,
    /// Install into the user's global config.
    User,
}

impl From<InstallScopeArg> for InstallScope {
    fn from(scope: InstallScopeArg) -> Self {
        match scope {
            InstallScopeArg::Project => Self::Project,
            InstallScopeArg::User => Self::User,
        }
    }
}

/// Runs the `add` command.
pub fn run_add(
    source: String,
    target: InstallScopeArg,
    skill_filter: Option<String>,
    harnesses: Option<String>,
    force: bool,
) -> Result<(), CommandError> {
    let scope = InstallScope::from(target);

    let project_root = match scope {
        InstallScope::Project => Some(find_project_root().map_err(CommandError::Usage)?),
        InstallScope::User => find_project_root().ok(),
    };

    let selected_harnesses =
        determine_harnesses(project_root.as_deref(), harnesses).map_err(CommandError::Runtime)?;
    let parsed = parse_source(&source)
        .map_err(InstallError::from)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    // Combine explicit --skill flag with any skill filter embedded in the source.
    let parsed = if let Some(filter) = skill_filter {
        embed_skill_filter(parsed, &filter)
    } else {
        parsed
    };

    let ctx = InstallContext {
        source_input: source,
        parsed,
        target_scope: scope,
        harnesses: selected_harnesses,
        project_root,
        force,
    };

    let results =
        install_source(&ctx).map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    let mut store =
        StateStore::open().map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    for result in results {
        store.upsert(result.record);
    }
    store
        .save()
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    Ok(())
}

fn find_project_root() -> Result<PathBuf, miette::Report> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join("skillprism.yaml").exists() {
            return Ok(dir.to_path_buf());
        }
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            return Err(miette::miette!(
                "No skillprism.yaml found. Run `skillprism init project <name>` to create one, or cd into a skillprism project."
            ));
        }
    }
}

fn determine_harnesses(
    project_root: Option<&Path>,
    harnesses: Option<String>,
) -> Result<Vec<String>, miette::Report> {
    if let Some(list) = harnesses {
        return Ok(list
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect());
    }

    if let Some(root) = project_root {
        match ProjectLoader::load(root) {
            Ok(model) => return Ok(model.config.harnesses),
            Err(ProjectError::ConfigNotFound { .. }) => {}
            Err(e) => return Err(miette::Report::new(e)),
        }
    }

    Ok(vec![
        "claude".to_string(),
        "codex".to_string(),
        "opencode".to_string(),
        "factory".to_string(),
        "pi".to_string(),
    ])
}

fn embed_skill_filter(
    parsed: super::source::ParsedSource,
    filter: &str,
) -> super::source::ParsedSource {
    match parsed {
        super::source::ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            ..
        } => super::source::ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            skill_filter: Some(filter.to_string()),
        },
        super::source::ParsedSource::GitLab {
            url,
            r#ref,
            subpath,
            ..
        } => super::source::ParsedSource::GitLab {
            url,
            r#ref,
            subpath,
            skill_filter: Some(filter.to_string()),
        },
        other => other,
    }
}

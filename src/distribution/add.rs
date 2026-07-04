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

use std::path::Path;

use crate::loader::ProjectLoader;
use crate::state::{InstallScope, StateStore};
use crate::types::ProjectError;
use clap::ValueEnum;
use dialoguer::{MultiSelect, Select};

use super::CommandError;
use super::detect::detect_installed_agents;
use super::find_project_root;
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

const ALL_HARNESSES: &[&str] = &["claude", "codex", "opencode", "factory", "pi"];

/// Runs the `add` command.
pub fn run_add(
    source: String,
    target: Option<InstallScopeArg>,
    skill_filter: Option<String>,
    harnesses: Option<String>,
    force: bool,
) -> Result<(), CommandError> {
    let scope = resolve_scope(target, force)?;

    let project_root = match scope {
        InstallScope::Project => Some(
            find_project_root()
                .map_err(|_| {
                    CommandError::Usage(miette::miette!(
                        "No skillprism.yaml found. Run `skillprism init project <name>` to create one, or cd into a skillprism project."
                    ))
                })?,
        ),
        InstallScope::User => find_project_root().ok(),
    };

    let selected_harnesses = determine_harnesses(project_root.as_deref(), harnesses, force)
        .map_err(CommandError::Runtime)?;

    let parsed = parse_source(&source)
        .map_err(InstallError::from)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

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

/// Resolve the install scope, prompting interactively when not provided.
fn resolve_scope(
    target: Option<InstallScopeArg>,
    force: bool,
) -> Result<InstallScope, CommandError> {
    if let Some(arg) = target {
        return Ok(InstallScope::from(arg));
    }

    if force {
        return Ok(InstallScope::Project);
    }

    let selection = Select::new()
        .with_prompt("Install scope")
        .item("project")
        .item("user")
        .default(0)
        .interact()
        .map_err(|e| {
            CommandError::Runtime(miette::miette!("Failed to read scope selection: {e}"))
        })?;

    Ok(match selection {
        0 => InstallScope::Project,
        _ => InstallScope::User,
    })
}

fn determine_harnesses(
    project_root: Option<&Path>,
    harnesses: Option<String>,
    force: bool,
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

    if force {
        return Ok(ALL_HARNESSES.iter().map(ToString::to_string).collect());
    }

    let detected = detect_installed_agents();
    let checked_items: Vec<(&str, bool)> = ALL_HARNESSES
        .iter()
        .map(|h| (*h, detected.iter().any(|d| d == h)))
        .collect();

    let selections = MultiSelect::new()
        .with_prompt("Select harnesses to install to (space to toggle, enter to confirm)")
        .items_checked(checked_items)
        .interact()
        .map_err(|e| miette::miette!("Failed to prompt for harness selection: {e}"))?;

    let selected: Vec<String> = selections
        .into_iter()
        .map(|i| ALL_HARNESSES[i].to_string())
        .collect();

    if selected.is_empty() {
        return Err(miette::miette!(
            "You must select at least one harness to install to."
        ));
    }

    Ok(selected)
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

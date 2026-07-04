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
use crate::registry::BUILTIN_HARNESS_IDS;
use crate::state::{InstallScope, StateStore};
use crate::types::ProjectError;
use clap::ValueEnum;
use dialoguer::{Confirm, MultiSelect, Select};

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

/// Runs the `add` command.
pub fn run_add(
    source: String,
    target: Option<InstallScopeArg>,
    skill_filter: Option<String>,
    harnesses: Option<String>,
    force: bool,
) -> Result<(), CommandError> {
    let project_root = find_project_root().ok();
    let scope = resolve_scope(target, force, project_root.is_some())?;

    if scope == InstallScope::Project && project_root.is_none() {
        return Err(CommandError::Usage(miette::miette!(
            "No skillprism.yaml found. Run `skillprism init project <name>` to create one, use `--target user`, or cd into a skillprism project."
        )));
    }

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

    if !force && !confirm_install(&source, scope, &selected_harnesses)? {
        return Ok(());
    }

    let ctx = InstallContext {
        source_input: source,
        parsed,
        target_scope: scope,
        harnesses: selected_harnesses,
        project_root,
        force,
    };

    let mut store =
        StateStore::open().map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    let results = install_source(&ctx, |record| {
        store.upsert(record.clone());
        store.save().map_err(InstallError::State)?;
        Ok(())
    })
    .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    print_install_summary(&results, scope);

    Ok(())
}

/// Resolve the install scope, prompting interactively when not provided.
fn resolve_scope(
    target: Option<InstallScopeArg>,
    force: bool,
    has_project: bool,
) -> Result<InstallScope, CommandError> {
    if let Some(arg) = target {
        return Ok(InstallScope::from(arg));
    }

    if force {
        return Ok(if has_project {
            InstallScope::Project
        } else {
            InstallScope::User
        });
    }

    let items: Vec<&str> = if has_project {
        vec!["project", "user"]
    } else {
        vec!["user"]
    };
    let selection = Select::new()
        .with_prompt("Install scope")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| {
            CommandError::Runtime(miette::miette!("Failed to read scope selection: {e}"))
        })?;

    Ok(match items.get(selection) {
        Some(&"project") => InstallScope::Project,
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
            Ok(model) if !model.config.harnesses.is_empty() => {
                return Ok(model.config.harnesses);
            }
            Ok(_) | Err(ProjectError::ConfigNotFound { .. }) => {
                // Empty project harness list or missing config: fall back to
                // built-ins so the user can still select targets interactively
                // or via --force.
            }
            Err(e) => return Err(miette::Report::new(e)),
        }
    }

    if force {
        return Ok(BUILTIN_HARNESS_IDS
            .iter()
            .map(ToString::to_string)
            .collect());
    }

    let detected = detect_installed_agents();
    if !detected.is_empty() {
        eprintln!("Detected agents: {}", detected.join(", "));
    }

    let checked_items: Vec<(&str, bool)> =
        BUILTIN_HARNESS_IDS.iter().map(|h| (*h, false)).collect();

    let selections = MultiSelect::new()
        .with_prompt("Select harnesses to install to (space to toggle, enter to confirm)")
        .items_checked(checked_items)
        .interact()
        .map_err(|e| miette::miette!("Failed to prompt for harness selection: {e}"))?;

    let selected: Vec<String> = selections
        .into_iter()
        .map(|i| BUILTIN_HARNESS_IDS[i].to_string())
        .collect();

    if selected.is_empty() {
        return Err(miette::miette!(
            "You must select at least one harness to install to."
        ));
    }

    Ok(selected)
}

/// Shows a summary of the install and asks the user to confirm.
///
/// Returns `true` if the user confirms (or if prompts are skipped), `false` if
/// the user declines.
fn confirm_install(
    source: &str,
    scope: InstallScope,
    harnesses: &[String],
) -> Result<bool, CommandError> {
    let scope_label = match scope {
        InstallScope::Project => "project",
        InstallScope::User => "user",
    };
    let harness_list = harnesses.join(", ");

    println!("Install summary:");
    println!("  source:    {source}");
    println!("  scope:     {scope_label}");
    println!("  harnesses: {harness_list}");

    Confirm::new()
        .with_prompt("Proceed with installation")
        .default(true)
        .interact()
        .map_err(|e| CommandError::Runtime(miette::miette!("Failed to read confirmation: {e}")))
}

fn print_install_summary(results: &[super::install::InstallResult], scope: InstallScope) {
    let scope_label = match scope {
        InstallScope::Project => "project",
        InstallScope::User => "user",
    };
    let count = results.len();
    if count == 0 {
        println!("No skills installed.");
        return;
    }
    println!(
        "Installed {count} skill{scope_suffix} to {scope_label} scope:",
        scope_suffix = if count == 1 { "" } else { "s" },
    );
    for result in results {
        let harness_list = result.record.harnesses.join(", ");
        println!("  - {} -> {harness_list}", result.record.name);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_target_bypasses_prompt() {
        assert!(matches!(
            resolve_scope(Some(InstallScopeArg::User), false, true).unwrap(),
            InstallScope::User
        ));
        assert!(matches!(
            resolve_scope(Some(InstallScopeArg::Project), false, false).unwrap(),
            InstallScope::Project
        ));
    }

    #[test]
    fn force_defaults_to_project_when_available() {
        assert!(matches!(
            resolve_scope(None, true, true).unwrap(),
            InstallScope::Project
        ));
    }

    #[test]
    fn force_defaults_to_user_when_no_project() {
        assert!(matches!(
            resolve_scope(None, true, false).unwrap(),
            InstallScope::User
        ));
    }

    #[test]
    fn explicit_harnesses_bypass_prompt() {
        let got = determine_harnesses(None, Some("claude,opencode".to_string()), false).unwrap();
        assert_eq!(got, vec!["claude", "opencode"]);
    }

    #[test]
    fn force_uses_all_builtin_harnesses_when_no_config() {
        let got = determine_harnesses(None, None, true).unwrap();
        assert_eq!(got, BUILTIN_HARNESS_IDS.to_vec());
    }
}

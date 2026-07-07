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

//! `skillprism remove` command implementation.

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::Path;

use miette::IntoDiagnostic;

use crate::cli::TargetScope;
use crate::registry::HarnessRegistry;
use crate::router;
use crate::state::{InstallScope, InstalledSkill, StateStore};

use super::CommandError;
use super::add::InstallScopeArg;
use super::find_project_root;

/// Runs the `remove` command.
#[allow(clippy::too_many_lines)]
#[allow(clippy::fn_params_excessive_bools)]
pub fn run_remove(
    skills: &[String],
    target: Option<InstallScopeArg>,
    harnesses: Option<String>,
    all: bool,
    all_scopes: bool,
    force: bool,
    verbose: bool,
) -> Result<(), CommandError> {
    if all && !skills.is_empty() {
        return Err(CommandError::Usage(miette::miette!(
            "--all cannot be combined with named skills"
        )));
    }

    if all_scopes && !all && skills.is_empty() {
        return Err(CommandError::Usage(miette::miette!(
            "--all-scopes requires --all or named skills"
        )));
    }

    let scopes = determine_scopes(target, all_scopes);
    if verbose {
        eprintln!("[remove] scopes: {scopes:?}");
    }
    let harness_filter = parse_harness_filter(harnesses);

    let mut store =
        StateStore::open().map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let removals = select_removals(store.skills(), &scopes, skills, all, &harness_filter);

    if removals.is_empty() {
        let requested = if skills.is_empty() {
            "No skills selected for removal. Provide skill names or use --all.".to_string()
        } else {
            // A named skill may be installed in a scope the caller did not
            // select (named removals default to project scope). Point the user
            // at the scope where it actually lives instead of the misleading
            // "not installed".
            let other_scopes: Vec<InstallScope> = store
                .skills()
                .iter()
                .filter(|s| skills.contains(&s.name) && !scopes.contains(&s.scope))
                .map(|s| s.scope)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            let subject = if skills.len() == 1 {
                format!("Skill '{}'", skills[0])
            } else {
                format!("Skills [{}]", skills.join(", "))
            };
            if other_scopes.is_empty() {
                format!("{subject} is not installed")
            } else {
                let hint = other_scopes
                    .iter()
                    .map(|s| match s {
                        InstallScope::Project => "--target project",
                        InstallScope::User => "--target user",
                    })
                    .collect::<Vec<_>>()
                    .join(" or ");
                let where_scopes = other_scopes
                    .iter()
                    .map(|s| match s {
                        InstallScope::Project => "project",
                        InstallScope::User => "user",
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "{subject} is not installed in the selected scope(s), but is installed in the {where_scopes} scope. Re-run with {hint} or --all-scopes to remove it."
                )
            }
        };
        return Err(CommandError::Runtime(miette::miette!(requested)));
    }

    let affected = describe_affected(&removals);
    if force {
        for line in &affected {
            println!("{line}");
        }
    } else {
        prompt_confirm(&affected)?;
    }

    for (skill, harnesses_to_remove) in &removals {
        for harness_id in harnesses_to_remove {
            remove_skill_files(skill, harness_id)?;
        }
    }

    apply_removals_to_state(&mut store, removals)?;
    store
        .save()
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;

    Ok(())
}

fn determine_scopes(target: Option<InstallScopeArg>, all_scopes: bool) -> Vec<InstallScope> {
    if all_scopes {
        return vec![InstallScope::Project, InstallScope::User];
    }
    match target {
        Some(InstallScopeArg::User) => vec![InstallScope::User],
        Some(InstallScopeArg::Project) | None => vec![InstallScope::Project],
    }
}

fn parse_harness_filter(harnesses: Option<String>) -> Vec<String> {
    harnesses
        .map(|h| {
            h.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// A removal action: the skill record and the harnesses to remove from it.
type RemovalAction = (InstalledSkill, Vec<String>);

fn select_removals(
    skills: &[InstalledSkill],
    scopes: &[InstallScope],
    names: &[String],
    all: bool,
    harness_filter: &[String],
) -> Vec<RemovalAction> {
    skills
        .iter()
        .filter(|s| scopes.contains(&s.scope))
        .filter(|s| all || names.contains(&s.name))
        .map(|s| {
            let to_remove: Vec<_> = if harness_filter.is_empty() {
                s.harnesses.clone()
            } else {
                s.harnesses
                    .iter()
                    .filter(|h| harness_filter.contains(h))
                    .cloned()
                    .collect()
            };
            (s.clone(), to_remove)
        })
        .filter(|(_, to_remove)| !to_remove.is_empty())
        .collect()
}

fn describe_affected(removals: &[RemovalAction]) -> Vec<String> {
    removals
        .iter()
        .map(|(skill, harnesses)| {
            format!(
                "{name} ({scope}): {harnesses}",
                name = skill.name,
                scope = match skill.scope {
                    InstallScope::Project => "project",
                    InstallScope::User => "user",
                },
                harnesses = harnesses.join(", ")
            )
        })
        .collect()
}

fn prompt_confirm(affected: &[String]) -> Result<(), CommandError> {
    println!("The following skills will be removed:");
    for line in affected {
        println!("  {line}");
    }
    print!("Are you sure? [y/N] ");
    io::stdout()
        .flush()
        .into_diagnostic()
        .map_err(CommandError::Runtime)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .into_diagnostic()
        .map_err(CommandError::Runtime)?;

    let trimmed = input.trim().to_lowercase();
    if trimmed != "y" && trimmed != "yes" {
        return Err(CommandError::Runtime(miette::miette!("Removal cancelled")));
    }
    Ok(())
}

fn remove_skill_files(skill: &InstalledSkill, harness_id: &str) -> Result<(), CommandError> {
    let registry = HarnessRegistry::with_builtins();
    let harness = registry
        .resolve(harness_id)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let target = install_scope_to_target(skill.scope);
    let root = resolve_removal_root(skill)?;

    let skill_path = router::resolve_skill_path(root.as_ref(), &harness, &skill.name, target)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let skill_dir = skill_path
        .parent()
        .expect("skill path should have a parent directory")
        .to_path_buf();

    if skill_dir.exists() {
        std::fs::remove_dir_all(&skill_dir)
            .into_diagnostic()
            .map_err(CommandError::Runtime)?;
    }

    Ok(())
}

fn resolve_removal_root(skill: &InstalledSkill) -> Result<Cow<'_, Path>, CommandError> {
    match skill.scope {
        InstallScope::Project => skill.project_root.as_deref().map_or_else(
            || {
                find_project_root().map(Cow::Owned).map_err(|_| {
                    CommandError::Usage(miette::miette!(
                        "--target project requires being inside a skillprism project"
                    ))
                })
            },
            |root| Ok(Cow::Borrowed(Path::new(root))),
        ),
        InstallScope::User => Ok(Cow::Borrowed(Path::new("."))),
    }
}

const fn install_scope_to_target(scope: InstallScope) -> TargetScope {
    match scope {
        InstallScope::Project => TargetScope::Project,
        InstallScope::User => TargetScope::User,
    }
}

fn apply_removals_to_state(
    store: &mut StateStore,
    removals: Vec<RemovalAction>,
) -> Result<(), CommandError> {
    for (skill, harnesses_to_remove) in removals {
        if harnesses_to_remove.len() >= skill.harnesses.len() {
            store.remove(&skill.name, skill.scope);
            continue;
        }

        let mut record = skill;
        for harness_id in &harnesses_to_remove {
            remove_harness_files_from_record(&mut record, harness_id)?;
        }
        record
            .harnesses
            .retain(|h| !harnesses_to_remove.contains(h));
        store.upsert(record);
    }
    Ok(())
}

fn remove_harness_files_from_record(
    record: &mut InstalledSkill,
    harness_id: &str,
) -> Result<(), CommandError> {
    let registry = HarnessRegistry::with_builtins();
    let harness = registry
        .resolve(harness_id)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let target = install_scope_to_target(record.scope);
    let root = resolve_removal_root(record)?;

    let skill_path = router::resolve_skill_path(root.as_ref(), &harness, &record.name, target)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let skill_dir = skill_path
        .parent()
        .expect("skill path should have a parent directory")
        .to_path_buf();

    record
        .files
        .retain(|f| !Path::new(&f.path).starts_with(&skill_dir));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{InstalledFile, SkillFormat, SourceType, now_rfc3339};

    fn sample_skill(name: &str, scope: InstallScope, harnesses: &[&str]) -> InstalledSkill {
        InstalledSkill {
            name: name.to_string(),
            source: format!("owner/{name}"),
            source_url: format!("https://github.com/owner/{name}.git"),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: None,
            scope,
            harnesses: harnesses.iter().map(|h| (*h).to_string()).collect(),
            format: SkillFormat::Skillprism,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![InstalledFile {
                path: format!("{name}.md"),
                hash: "sha256:abc".to_string(),
            }],
        }
    }

    #[test]
    fn select_all_in_scope() {
        let skills = vec![
            sample_skill("alpha", InstallScope::Project, &["claude"]),
            sample_skill("beta", InstallScope::User, &["opencode"]),
        ];
        let removals = select_removals(&skills, &[InstallScope::Project], &[], true, &[]);
        assert_eq!(removals.len(), 1);
        assert_eq!(removals[0].0.name, "alpha");
        assert_eq!(removals[0].1, vec!["claude"]);
    }

    #[test]
    fn select_by_name_and_harness_filter() {
        let skills = vec![sample_skill(
            "alpha",
            InstallScope::Project,
            &["claude", "opencode"],
        )];
        let removals = select_removals(
            &skills,
            &[InstallScope::Project],
            &["alpha".to_string()],
            false,
            &["claude".to_string()],
        );
        assert_eq!(removals.len(), 1);
        assert_eq!(removals[0].1, vec!["claude"]);
    }

    #[test]
    fn select_skips_non_matching_harness() {
        let skills = vec![sample_skill("alpha", InstallScope::Project, &["claude"])];
        let removals = select_removals(
            &skills,
            &[InstallScope::Project],
            &["alpha".to_string()],
            false,
            &["opencode".to_string()],
        );
        assert!(removals.is_empty());
    }

    #[test]
    fn apply_partial_harness_removal_updates_record() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let claude_file = root.join(".claude/skills/alpha/SKILL.md");
        let opencode_file = root.join(".opencode/skills/alpha/SKILL.md");
        std::fs::create_dir_all(claude_file.parent().unwrap()).unwrap();
        std::fs::create_dir_all(opencode_file.parent().unwrap()).unwrap();
        std::fs::write(&claude_file, b"claude").unwrap();
        std::fs::write(&opencode_file, b"opencode").unwrap();

        let state_dir = tmp.path().join("state");
        let mut store = StateStore::open_at(&state_dir).unwrap();
        store.upsert(InstalledSkill {
            name: "alpha".to_string(),
            source: "owner/alpha".to_string(),
            source_url: "https://github.com/owner/alpha.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: Some(root.to_string_lossy().to_string()),
            scope: InstallScope::Project,
            harnesses: vec!["claude".to_string(), "opencode".to_string()],
            format: SkillFormat::Skillprism,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![
                InstalledFile {
                    path: claude_file.to_string_lossy().to_string(),
                    hash: "sha256:a".to_string(),
                },
                InstalledFile {
                    path: opencode_file.to_string_lossy().to_string(),
                    hash: "sha256:b".to_string(),
                },
            ],
        });
        store.save().unwrap();

        let action = (store.skills()[0].clone(), vec!["claude".to_string()]);
        apply_removals_to_state(&mut store, vec![action]).unwrap();

        assert_eq!(store.skills().len(), 1);
        let updated = &store.skills()[0];
        assert_eq!(updated.harnesses, vec!["opencode"]);
        assert_eq!(updated.files.len(), 1);
        assert!(updated.files[0].path.contains(".opencode"));
    }
}

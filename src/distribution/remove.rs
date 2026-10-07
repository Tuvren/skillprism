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
use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Component, Path, PathBuf};

use crate::registry::HarnessRegistry;

use miette::IntoDiagnostic;

use crate::state::{InstallScope, InstalledSkill, StateStore};

use super::CommandError;
use super::add::InstallScopeArg;
use super::find_project_root;

/// Runs the `remove` command.
// reason: signature mirrors the clap `remove` command surface (skills, target,
// harnesses, all, all_scopes, force, verbose); the bool flags map 1:1 to CLI
// flags, and the body is the linear select → confirm → delete → persist pipeline.
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
    let harnesses = super::canonical_harness_arg(harnesses).map_err(CommandError::Runtime)?;
    let harness_filter = parse_harness_filter(harnesses);

    let mut store =
        StateStore::open().map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    let active_project_root = super::find_project_root().ok();
    let removals = select_removals(
        store.skills(),
        &scopes,
        skills,
        all,
        &harness_filter,
        active_project_root.as_deref(),
    );

    if removals.is_empty() {
        let msg =
            format_empty_removals_message(&store, skills, &scopes, active_project_root.as_deref());
        return Err(CommandError::Runtime(miette::miette!(msg)));
    }

    let affected = describe_affected(&removals);
    // `--all` skips confirmation the same way `-y` / `--force` do.
    if force || all {
        // Diagnostics/confirmations go to stderr; stdout is reserved for the
        // `list` table and `--diff` output.
        for line in &affected {
            eprintln!("{line}");
        }
    } else {
        prompt_confirm(&affected)?;
    }

    for (skill, harnesses_to_remove) in &removals {
        remove_skill_files(skill, harnesses_to_remove)?;
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

fn format_empty_removals_message(
    store: &StateStore,
    skills: &[String],
    scopes: &[InstallScope],
    active_project_root: Option<&Path>,
) -> String {
    if skills.is_empty() {
        "No skills selected for removal. Provide skill names or use --all.".to_string()
    } else {
        let other_scopes: Vec<InstallScope> = store
            .skills()
            .iter()
            .filter(|s| {
                skills.contains(&s.name)
                    && (!scopes.contains(&s.scope) || !s.matches_project_root(active_project_root))
            })
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
                .map(|s| format!("--target {}", s.as_str()))
                .collect::<Vec<_>>()
                .join(" or ");
            let where_scopes = other_scopes
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{subject} is not installed in the selected scope(s), but is installed in the {where_scopes} scope. Re-run with {hint} or --all-scopes to remove it."
            )
        }
    }
}

fn parse_harness_filter(harnesses: Option<String>) -> Vec<String> {
    harnesses
        .map(|h| super::parse_harness_list(&h))
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
    active_project_root: Option<&Path>,
) -> Vec<RemovalAction> {
    skills
        .iter()
        .filter(|s| scopes.contains(&s.scope))
        .filter(|s| s.matches_project_root(active_project_root))
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
                scope = skill.scope.as_str(),
                harnesses = harnesses.join(", ")
            )
        })
        .collect()
}

fn prompt_confirm(affected: &[String]) -> Result<(), CommandError> {
    // Interactive prompt UI goes to stderr, keeping stdout clean for piped data.
    eprintln!("The following skills will be removed:");
    for line in affected {
        eprintln!("  {line}");
    }
    if !io::stdin().is_terminal() {
        return Err(CommandError::Usage(miette::miette!(
            "Cannot prompt for confirmation in a non-interactive environment. Pass `-y` or `--force` to skip the prompt. `remove --all` also skips it."
        )));
    }
    eprint!("Are you sure? [y/N] ");
    io::stderr()
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

fn remove_skill_files(skill: &InstalledSkill, harness_ids: &[String]) -> Result<(), CommandError> {
    let registry = load_registry(skill)?;
    let kept_dirs = kept_output_dirs(skill, harness_ids, &registry)?;
    let removed_dirs = skill_output_dirs(skill, harness_ids, &registry)?;
    // Delete recorded files that belong to a harness being removed.
    // A path a staying harness still resolves to is left in place.
    // A canonical path that leaves the installer base is skipped, not deleted.
    // The skill directory is removed only as the parent of a file this call
    // actually deleted — the longest removed directory that contains that file
    // (the trailing `opencode/skills/<skill>`, or the current/legacy directory).
    let mut deleted_dirs = HashSet::new();
    let mut skipped_paths = Vec::new();
    for file in &skill.files {
        let path = PathBuf::from(&file.path);
        if !path_is_only_under(&path, &removed_dirs, &kept_dirs) {
            continue;
        }
        if !file_is_removed(skill, &path, &removed_dirs, &kept_dirs) {
            skipped_paths.push(path);
            continue;
        }
        if path.is_file() {
            std::fs::remove_file(&path)
                .into_diagnostic()
                .map_err(CommandError::Runtime)?;
            if let Some(dir) = longest_prefixed_dir(&path, &removed_dirs) {
                deleted_dirs.insert(dir);
            }
        }
    }
    remove_deleted_skill_dirs(skill, &deleted_dirs, &kept_dirs, &skipped_paths)
}

fn file_is_removed(
    skill: &InstalledSkill,
    path: &Path,
    removed_dirs: &HashSet<PathBuf>,
    kept_dirs: &HashSet<PathBuf>,
) -> bool {
    path_is_only_under(path, removed_dirs, kept_dirs)
        && path_is_contained(path, removal_allowed_base(skill, path).as_deref())
}

/// Longest removal directory that contains `path`.
///
/// That is the skill directory at the end of the recorded path. A shorter
/// `opencode/skills/<skill>` prefix is not selected once a longer one exists.
fn longest_prefixed_dir(path: &Path, dirs: &HashSet<PathBuf>) -> Option<PathBuf> {
    dirs.iter()
        .filter(|dir| path.starts_with(dir))
        .max_by_key(|dir| dir.components().count())
        .cloned()
}

fn remove_deleted_skill_dirs(
    skill: &InstalledSkill,
    dirs: &HashSet<PathBuf>,
    kept_dirs: &HashSet<PathBuf>,
    skipped_paths: &[PathBuf],
) -> Result<(), CommandError> {
    for dir in dirs {
        if kept_dirs.contains(dir)
            || kept_dirs
                .iter()
                .any(|kept| kept.starts_with(dir) || dir.starts_with(kept))
        {
            continue;
        }
        // `remove_dir_all` would also unlink a recorded file containment skipped.
        if skipped_paths.iter().any(|path| path.starts_with(dir)) {
            continue;
        }
        if !path_is_contained(dir, removal_allowed_base(skill, dir).as_deref()) {
            continue;
        }
        if dir.exists() {
            std::fs::remove_dir_all(dir)
                .into_diagnostic()
                .map_err(CommandError::Runtime)?;
        }
    }
    Ok(())
}

fn path_is_only_under(path: &Path, removed: &HashSet<PathBuf>, kept: &HashSet<PathBuf>) -> bool {
    removed.iter().any(|dir| path.starts_with(dir)) && !kept.iter().any(|dir| path.starts_with(dir))
}

fn load_registry(skill: &InstalledSkill) -> Result<HarnessRegistry, CommandError> {
    super::install::build_registry_for_harnesses(project_root_for_registry(skill).as_deref())
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))
}

fn output_dir(
    skill: &InstalledSkill,
    harness_id: &str,
    registry: &HarnessRegistry,
) -> Result<PathBuf, CommandError> {
    let harness = registry
        .resolve(harness_id)
        .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?;
    // Lexical join, not `resolve_skill_path`. A symlinked ancestor makes that
    // helper return `PathTraversal`; removal skips the directory instead of
    // failing, after `path_is_contained` checks the canonical path.
    let scope_path = match skill.scope {
        InstallScope::Project => {
            let root = resolve_removal_root(skill)?;
            root.join(&harness.paths.project_scope_path)
        }
        InstallScope::User => crate::router::paths::user_scope_anchor(&harness)
            .map_err(|e| CommandError::Runtime(miette::Report::new(e)))?
            .join(&harness.paths.user_scope_path),
    };
    Ok(scope_path.join(&skill.name))
}

fn kept_output_dirs(
    skill: &InstalledSkill,
    removing: &[String],
    registry: &HarnessRegistry,
) -> Result<HashSet<PathBuf>, CommandError> {
    let staying: Vec<String> = skill
        .harnesses
        .iter()
        .filter(|harness_id| !removing.contains(harness_id))
        .cloned()
        .collect();
    skill_output_dirs(skill, &staying, registry)
}

/// Skill directories whose recorded files belong to `harness_ids`.
///
/// Every directory is omitted unless a recorded file path sits inside it,
/// including the current output directory. That directory is also the pre-move
/// `OpenCode` user path when `XDG_CONFIG_HOME` is unset, so an unrecorded file
/// there must survive. A retired `XDG_CONFIG_HOME` tree is included only when
/// state still records a file under `opencode/skills/<skill>`.
fn skill_output_dirs(
    skill: &InstalledSkill,
    harness_ids: &[String],
    registry: &HarnessRegistry,
) -> Result<HashSet<PathBuf>, CommandError> {
    let mut dirs = HashSet::new();
    for harness_id in harness_ids {
        let dir = output_dir(skill, harness_id, registry)?;
        if recorded_file_inside(skill, &dir) {
            dirs.insert(dir);
        }
    }
    dirs.extend(legacy_dirs_with_recorded_files(skill, harness_ids));
    dirs.extend(recorded_opencode_user_dirs(skill, harness_ids));
    Ok(dirs)
}

/// `OpenCode` user skill directories named by recorded files.
///
/// Installs made while `XDG_CONFIG_HOME` was set to a directory other than
/// `$HOME/.config` are not the current output directory and are not the
/// historical `$HOME/.config/opencode/skills/<skill>` path. The directory is
/// the trailing `opencode/skills/<skill>`, never an earlier copy of that
/// sequence that is only part of `XDG_CONFIG_HOME`.
fn recorded_opencode_user_dirs(skill: &InstalledSkill, harness_ids: &[String]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    if skill.scope != InstallScope::User || !harness_ids.iter().any(|id| id == "opencode") {
        return dirs;
    }
    for file in &skill.files {
        if let Some(dir) = opencode_user_skill_dir(Path::new(&file.path), &skill.name) {
            dirs.insert(dir);
        }
    }
    dirs
}

/// Rightmost `opencode/skills/<skill_name>` prefix of `path`.
///
/// `XDG_CONFIG_HOME` may itself be `$HOME/opencode/skills/<skill>/config`.
/// The earlier sequence is not the installed skill directory.
fn opencode_user_skill_dir(path: &Path, skill_name: &str) -> Option<PathBuf> {
    let components: Vec<Component<'_>> = path.components().collect();
    let start = components.windows(3).rposition(|window| {
        window[0].as_os_str() == "opencode"
            && window[1].as_os_str() == "skills"
            && window[2].as_os_str() == skill_name
    })?;
    Some(components[..=start + 2].iter().copied().collect())
}

fn legacy_dirs_with_recorded_files(
    skill: &InstalledSkill,
    harness_ids: &[String],
) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for harness_id in harness_ids {
        if let Some(legacy) = legacy_output_dir(skill, harness_id) {
            if recorded_file_inside(skill, &legacy) {
                dirs.insert(legacy);
            }
        }
    }
    dirs
}

fn recorded_file_inside(skill: &InstalledSkill, dir: &Path) -> bool {
    skill
        .files
        .iter()
        .any(|file| Path::new(&file.path).starts_with(dir))
}

/// Installer anchor for `path`. A canonical path that leaves this base is not
/// deleted. `$HOME`, the project root, and `$XDG_CONFIG_HOME` may be symlinks.
///
/// Project scope uses the project root. User scope uses `$HOME`, except
/// `OpenCode`. The current user directory is anchored at `$XDG_CONFIG_HOME`
/// when that variable is set and non-empty, otherwise at `$HOME/.config`.
/// Any other recorded `OpenCode` user file is anchored at the retired `XDG` root:
/// the parent of the trailing `opencode/skills/<skill>` on that path, even
/// when that root is outside `$HOME`. The default `$HOME/.config` directory
/// is still refused when it is a symlink pointing outside `$HOME`.
fn removal_allowed_base(skill: &InstalledSkill, path: &Path) -> Option<PathBuf> {
    match skill.scope {
        InstallScope::Project => skill
            .project_root
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| find_project_root().ok()),
        InstallScope::User => user_removal_base(skill, path),
    }
}

fn user_removal_base(skill: &InstalledSkill, path: &Path) -> Option<PathBuf> {
    if let Some(current) = current_opencode_user_dir(&skill.name) {
        if path.starts_with(&current) {
            return crate::router::paths::xdg_config_home().ok();
        }
    }
    if let Some(root) = retired_opencode_xdg_root(path, &skill.name) {
        return Some(root);
    }
    home_dir()
}

/// Parent of the trailing `opencode/skills/<skill>` — the `XDG` root that
/// install joined `opencode/skills` onto.
fn retired_opencode_xdg_root(path: &Path, skill_name: &str) -> Option<PathBuf> {
    let skill_dir = opencode_user_skill_dir(path, skill_name)?;
    let skills = skill_dir.parent()?;
    let opencode = skills.parent()?;
    let root = opencode.parent()?;
    if root.as_os_str().is_empty() {
        return None;
    }
    Some(root.to_path_buf())
}

fn current_opencode_user_dir(skill_name: &str) -> Option<PathBuf> {
    let anchor = crate::router::paths::xdg_config_home().ok()?;
    Some(anchor.join("opencode/skills").join(skill_name))
}

fn home_dir() -> Option<PathBuf> {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => Some(PathBuf::from(home)),
        _ => None,
    }
}

/// Both sides canonicalize, matching `validate_scope_relative`.
///
/// `$HOME`, the project root, and an explicit `$XDG_CONFIG_HOME` may be
/// symlinks; their targets are not required to stay inside the lexical parent.
/// The default `$HOME/.config` directory is the exception: when that directory
/// itself is a symlink pointing outside `$HOME`, removal does not follow it.
/// A missing path falls back to a lexical check with no `..`.
fn path_is_contained(path: &Path, allowed_base: Option<&Path>) -> bool {
    let Some(allowed_base) = allowed_base else {
        return false;
    };
    if refuses_symlinked_default_config(allowed_base) {
        return false;
    }
    match (path.canonicalize(), allowed_base.canonicalize()) {
        (Ok(resolved), Ok(base)) => resolved.starts_with(base),
        _ => lexical_path_is_contained(path, allowed_base),
    }
}

/// True when `allowed_base` is the default `$HOME/.config` (not an explicit
/// `$XDG_CONFIG_HOME` set to that same path) and the directory is a symlink
/// whose target leaves `$HOME`.
fn refuses_symlinked_default_config(allowed_base: &Path) -> bool {
    let Some(home) = home_dir() else {
        return false;
    };
    let default_config = home.join(".config");
    if allowed_base != default_config {
        return false;
    }
    if std::env::var("XDG_CONFIG_HOME")
        .is_ok_and(|xdg| !xdg.is_empty() && Path::new(&xdg) == allowed_base)
    {
        return false;
    }
    !symlink_base_stays_inside_parent(allowed_base)
}

fn symlink_base_stays_inside_parent(allowed_base: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(allowed_base) else {
        return true;
    };
    if !metadata.file_type().is_symlink() {
        return true;
    }
    let Some(parent) = allowed_base
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return false;
    };
    match (allowed_base.canonicalize(), parent.canonicalize()) {
        (Ok(target), Ok(parent_canon)) => target.starts_with(parent_canon),
        _ => false,
    }
}

fn lexical_path_is_contained(path: &Path, allowed_base: &Path) -> bool {
    if !path.starts_with(allowed_base) {
        return false;
    }
    let Ok(relative) = path.strip_prefix(allowed_base) else {
        return true;
    };
    relative
        .components()
        .all(|component| component != Component::ParentDir)
}

/// Project or user directory a harness used before the `.agents/skills` move.
fn legacy_output_dir(skill: &InstalledSkill, harness_id: &str) -> Option<PathBuf> {
    let relative = match (harness_id, skill.scope) {
        ("opencode", InstallScope::Project) => ".opencode/skills",
        ("factory", InstallScope::Project) => ".factory/skills",
        ("pi", InstallScope::Project) => ".pi/skills",
        ("pi", InstallScope::User) => ".pi/agent/skills",
        // OpenCode's user dir followed `$HOME/.config` before `$XDG_CONFIG_HOME`.
        ("opencode", InstallScope::User) => ".config/opencode/skills",
        _ => return None,
    };
    let anchor = match skill.scope {
        InstallScope::Project => skill.project_root.as_ref().map(PathBuf::from)?,
        InstallScope::User => home_dir()?,
    };
    Some(anchor.join(relative).join(&skill.name))
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
        // User scope resolves to `$HOME`-based paths inside
        // `router::resolve_skill_path`, which ignores the `root` argument for
        // `TargetScope::User`. The `"."` here is an unused placeholder, not a
        // real project root.
        InstallScope::User => Ok(Cow::Borrowed(Path::new("."))),
    }
}

fn project_root_for_registry(skill: &InstalledSkill) -> Option<PathBuf> {
    skill
        .project_root
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| find_project_root().ok())
}

fn apply_removals_to_state(
    store: &mut StateStore,
    removals: Vec<RemovalAction>,
) -> Result<(), CommandError> {
    for (skill, harnesses_to_remove) in removals {
        let registry = load_registry(&skill)?;
        let kept_dirs = kept_output_dirs(&skill, &harnesses_to_remove, &registry)?;
        let removed_dirs = skill_output_dirs(&skill, &harnesses_to_remove, &registry)?;
        let mut record = skill;
        let files = std::mem::take(&mut record.files);
        record.files = files
            .into_iter()
            .filter(|file| {
                // State follows disk: a file containment skipped stays recorded,
                // including when every harness on the skill was selected.
                !file_is_removed(&record, Path::new(&file.path), &removed_dirs, &kept_dirs)
            })
            .collect();

        // Drop the record only when nothing skipped remains. `store.remove`
        // would also forget a harness whose file is still on disk.
        if record.files.is_empty()
            && record
                .harnesses
                .iter()
                .all(|harness_id| harnesses_to_remove.contains(harness_id))
        {
            store.remove(&record.name, record.scope, record.project_root.as_deref());
            continue;
        }

        let previous_harnesses = record.harnesses.clone();
        let mut kept_harnesses = Vec::new();
        for harness_id in &previous_harnesses {
            if !harnesses_to_remove.contains(harness_id) {
                kept_harnesses.push(harness_id.clone());
                continue;
            }
            let Ok(own_dirs) =
                skill_output_dirs(&record, std::slice::from_ref(harness_id), &registry)
            else {
                kept_harnesses.push(harness_id.clone());
                continue;
            };
            if record
                .files
                .iter()
                .any(|file| path_is_only_under(Path::new(&file.path), &own_dirs, &kept_dirs))
            {
                kept_harnesses.push(harness_id.clone());
            }
        }
        record.harnesses = kept_harnesses;
        if record.harnesses.is_empty() && record.files.is_empty() {
            store.remove(&record.name, record.scope, record.project_root.as_deref());
            continue;
        }
        if record.harnesses.is_empty() {
            record.harnesses.clone_from(&harnesses_to_remove);
        }
        store.upsert(record);
    }
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
        let removals = select_removals(&skills, &[InstallScope::Project], &[], true, &[], None);
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
            None,
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
            None,
        );
        assert!(removals.is_empty());
    }

    #[test]
    fn select_filters_by_active_project_root() {
        let mut skill_a = sample_skill("shared", InstallScope::Project, &["claude"]);
        skill_a.project_root = Some("/path/to/project_a".to_string());

        let mut skill_b = sample_skill("shared", InstallScope::Project, &["claude"]);
        skill_b.project_root = Some("/path/to/project_b".to_string());

        let skills = vec![skill_a, skill_b];
        let root_a = Path::new("/path/to/project_a");
        let removals = select_removals(
            &skills,
            &[InstallScope::Project],
            &[],
            true,
            &[],
            Some(root_a),
        );
        assert_eq!(removals.len(), 1);
        assert_eq!(
            removals[0].0.project_root.as_deref(),
            Some("/path/to/project_a")
        );
    }

    #[test]
    fn apply_partial_harness_removal_updates_record() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let claude_file = root.join(".claude/skills/alpha/SKILL.md");
        let opencode_file = root.join(".agents/skills/alpha/SKILL.md");
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
        assert!(updated.files[0].path.contains(".agents"));
    }

    #[test]
    fn filtered_removal_keeps_files_owned_by_another_shared_harness() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let shared = root.join(".agents/skills/plain-skill/SKILL.md");
        std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
        std::fs::write(&shared, b"shared").unwrap();

        let skill = InstalledSkill {
            name: "plain-skill".to_string(),
            source: "owner/plain-skill".to_string(),
            source_url: "https://github.com/owner/plain-skill.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: Some(root.to_string_lossy().to_string()),
            scope: InstallScope::Project,
            harnesses: vec!["codex".to_string(), "opencode".to_string()],
            format: SkillFormat::Plain,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![InstalledFile {
                path: shared.to_string_lossy().to_string(),
                hash: "sha256:shared".to_string(),
            }],
        };

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            shared.exists(),
            "removing opencode must keep the skill file codex still owns"
        );

        let state_dir = tmp.path().join("state");
        let mut store = StateStore::open_at(&state_dir).unwrap();
        store.upsert(skill);
        let action = (store.skills()[0].clone(), vec!["opencode".to_string()]);
        apply_removals_to_state(&mut store, vec![action]).unwrap();

        assert_eq!(store.skills().len(), 1);
        assert_eq!(store.skills()[0].harnesses, vec!["codex"]);
        assert!(
            store.skills()[0]
                .files
                .iter()
                .any(|file| file.path.ends_with("SKILL.md"))
        );
        assert!(shared.exists());

        let remaining = store.skills()[0].clone();
        remove_skill_files(&remaining, &["codex".to_string()]).unwrap();
        assert!(
            !shared.exists(),
            "the shared directory is removed once no installed harness owns it"
        );
    }

    #[test]
    fn remove_deletes_recorded_legacy_directory_and_keeps_shared_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let legacy = root.join(".opencode/skills/plain-skill/SKILL.md");
        let shared = root.join(".agents/skills/plain-skill/SKILL.md");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"legacy").unwrap();
        std::fs::write(&shared, b"shared").unwrap();

        let skill = InstalledSkill {
            name: "plain-skill".to_string(),
            source: "owner/plain-skill".to_string(),
            source_url: "https://github.com/owner/plain-skill.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: Some(root.to_string_lossy().to_string()),
            scope: InstallScope::Project,
            harnesses: vec!["opencode".to_string(), "codex".to_string()],
            format: SkillFormat::Plain,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![
                InstalledFile {
                    path: legacy.to_string_lossy().to_string(),
                    hash: "sha256:legacy".to_string(),
                },
                InstalledFile {
                    path: shared.to_string_lossy().to_string(),
                    hash: "sha256:shared".to_string(),
                },
            ],
        };

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !legacy.exists(),
            "the recorded pre-move OpenCode directory must be deleted"
        );
        assert!(
            shared.exists(),
            "codex still references the shared .agents/skills file"
        );

        let state_dir = tmp.path().join("state");
        let mut store = StateStore::open_at(&state_dir).unwrap();
        store.upsert(skill);
        let action = (store.skills()[0].clone(), vec!["opencode".to_string()]);
        apply_removals_to_state(&mut store, vec![action]).unwrap();
        assert_eq!(store.skills()[0].harnesses, vec!["codex".to_string()]);
        assert!(
            store.skills()[0]
                .files
                .iter()
                .all(|file| !file.path.contains(".opencode"))
        );
        assert!(
            store.skills()[0]
                .files
                .iter()
                .any(|file| file.path.contains(".agents"))
        );
    }

    fn plain_project_skill(
        root: &Path,
        harnesses: &[&str],
        files: Vec<InstalledFile>,
    ) -> InstalledSkill {
        InstalledSkill {
            name: "plain-skill".to_string(),
            source: "owner/plain-skill".to_string(),
            source_url: "https://github.com/owner/plain-skill.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: Some(root.to_string_lossy().to_string()),
            scope: InstallScope::Project,
            harnesses: harnesses.iter().map(|id| (*id).to_string()).collect(),
            format: SkillFormat::Plain,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files,
        }
    }

    #[test]
    fn remove_keeps_unrecorded_legacy_opencode_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let legacy = root.join(".opencode/skills/plain-skill/SKILL.md");
        let current = root.join(".agents/skills/plain-skill/SKILL.md");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::create_dir_all(current.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"user file").unwrap();
        std::fs::write(&current, b"installed").unwrap();

        let skill = plain_project_skill(
            root,
            &["opencode"],
            vec![InstalledFile {
                path: current.to_string_lossy().to_string(),
                hash: "sha256:current".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            legacy.exists(),
            "an unrecorded .opencode/skills file must survive opencode removal"
        );
        assert!(
            !current.exists(),
            "the recorded install file is still removed"
        );
    }

    #[test]
    fn remove_skips_legacy_directory_symlinked_outside_the_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".opencode")).unwrap();

        let legacy = root.join(".opencode/skills/plain-skill/SKILL.md");
        let sibling = legacy.parent().unwrap().join("notes.txt");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"outside").unwrap();
        std::fs::write(&sibling, b"keep").unwrap();

        let skill = plain_project_skill(
            &root,
            &["opencode"],
            vec![InstalledFile {
                path: legacy.to_string_lossy().to_string(),
                hash: "sha256:legacy".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            legacy.exists(),
            "a recorded path reached through a symlink outside the project must not be deleted"
        );
        assert!(
            sibling.exists(),
            "the legacy directory itself must not be removed when it escapes the project"
        );
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::router::paths::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn plain_user_skill(harnesses: &[&str], files: Vec<InstalledFile>) -> InstalledSkill {
        InstalledSkill {
            name: "plain-skill".to_string(),
            source: "owner/plain-skill".to_string(),
            source_url: "https://github.com/owner/plain-skill.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: None,
            scope: InstallScope::User,
            harnesses: harnesses.iter().map(|id| (*id).to_string()).collect(),
            format: SkillFormat::Plain,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files,
        }
    }

    fn write_skill_file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"skill").unwrap();
    }

    #[test]
    fn remove_keeps_unrecorded_default_opencode_file_and_deletes_old_xdg_file() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());

        let recorded = home
            .path()
            .join("old-xdg/opencode/skills/plain-skill/SKILL.md");
        let neighbor = home.path().join("old-xdg/opencode/skills/keep.txt");
        let unrecorded = home
            .path()
            .join(".config/opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        write_skill_file(&neighbor);
        write_skill_file(&unrecorded);

        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:old".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "the recorded file under the retired XDG_CONFIG_HOME must be deleted"
        );
        assert!(
            neighbor.exists(),
            "removal must not wipe the retired XDG root outside the skill directory"
        );
        assert!(
            unrecorded.exists(),
            "an unrecorded $HOME/.config OpenCode file must survive when XDG_CONFIG_HOME is unset"
        );
    }

    #[test]
    fn remove_deletes_recorded_default_opencode_user_directory() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());

        let recorded = home
            .path()
            .join(".config/opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:current".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a recorded file under the default OpenCode user directory must be deleted"
        );
    }

    #[test]
    fn remove_keeps_unrecorded_xdg_dir_and_deletes_recorded_historical_opencode_file() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let xdg = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", xdg.path());

        let recorded = home
            .path()
            .join(".config/opencode/skills/plain-skill/SKILL.md");
        let unrecorded = xdg.path().join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        write_skill_file(&unrecorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:historical".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "the recorded historical $HOME/.config OpenCode file must be deleted"
        );
        assert!(
            unrecorded.exists(),
            "an unrecorded file under the current XDG_CONFIG_HOME must survive"
        );
    }

    #[test]
    fn remove_deletes_opencode_user_install_when_xdg_config_home_is_outside_home() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let xdg = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", xdg.path());

        let recorded = xdg.path().join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:xdg".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a recorded OpenCode install under an XDG_CONFIG_HOME outside $HOME must be deleted"
        );
    }

    #[test]
    fn remove_skips_current_output_dir_when_agents_symlinks_outside_the_project() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".agents")).unwrap();

        let recorded = root.join(".agents/skills/plain-skill/SKILL.md");
        let sibling = recorded.parent().unwrap().join("notes.txt");
        write_skill_file(&recorded);
        std::fs::write(&sibling, b"keep").unwrap();

        let skill = plain_project_skill(
            &root,
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:agents".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            recorded.exists(),
            "a current output directory reached through a symlinked .agents must not be deleted"
        );
        assert!(
            sibling.exists(),
            "remove_dir_all must not follow .agents outside the project"
        );
    }

    #[test]
    fn remove_skips_opencode_user_dir_when_config_symlinks_outside_home() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        std::os::unix::fs::symlink(outside.path(), home.path().join(".config")).unwrap();

        let recorded = home
            .path()
            .join(".config/opencode/skills/plain-skill/SKILL.md");
        let sibling = recorded.parent().unwrap().join("notes.txt");
        write_skill_file(&recorded);
        std::fs::write(&sibling, b"keep").unwrap();

        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:config".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            recorded.exists(),
            "a symlinked $HOME/.config must not widen OpenCode user removal outside its parent"
        );
        assert!(
            sibling.exists(),
            "remove_dir_all must not follow $HOME/.config outside $HOME"
        );
    }

    #[test]
    fn remove_skips_historical_opencode_symlink_and_deletes_xdg_install() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let xdg = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", xdg.path());
        std::os::unix::fs::symlink(outside.path(), home.path().join(".config")).unwrap();

        let historical = home
            .path()
            .join(".config/opencode/skills/plain-skill/SKILL.md");
        let sibling = historical.parent().unwrap().join("notes.txt");
        let current = xdg.path().join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&historical);
        std::fs::write(&sibling, b"keep").unwrap();
        write_skill_file(&current);

        let skill = plain_user_skill(
            &["opencode"],
            vec![
                InstalledFile {
                    path: historical.to_string_lossy().to_string(),
                    hash: "sha256:historical".to_string(),
                },
                InstalledFile {
                    path: current.to_string_lossy().to_string(),
                    hash: "sha256:current".to_string(),
                },
            ],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            historical.exists(),
            "the historical OpenCode path is anchored at $HOME and must not follow a symlink out"
        );
        assert!(sibling.exists());
        assert!(
            !current.exists(),
            "the current XDG_CONFIG_HOME install is outside $HOME and must still be deleted"
        );
    }

    #[test]
    fn remove_does_not_treat_xdg_config_home_prefix_as_the_skill_directory() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let xdg = home.path().join("opencode/skills/plain-skill/config");
        std::fs::create_dir_all(&xdg).unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", &xdg);

        let recorded = xdg.join("opencode/skills/plain-skill/SKILL.md");
        let keep = xdg.join("other-app/keep.txt");
        write_skill_file(&recorded);
        write_skill_file(&keep);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:nested-xdg".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "the install under XDG_CONFIG_HOME must still be deleted"
        );
        assert!(
            keep.exists(),
            "an earlier opencode/skills/<skill> inside XDG_CONFIG_HOME is not the skill directory"
        );
    }

    #[test]
    fn remove_deletes_claude_user_install_when_home_is_a_symlink() {
        let _lock = env_lock();
        let real_home = tempfile::tempdir().unwrap();
        let link_parent = tempfile::tempdir().unwrap();
        let home_link = link_parent.path().join("home");
        std::os::unix::fs::symlink(real_home.path(), &home_link).unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", &home_link);
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");

        let recorded = home_link.join(".claude/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_user_skill(
            &["claude"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:home-link".to_string(),
            }],
        );

        remove_skill_files(&skill, &["claude".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a symlinked $HOME must not block removal of a non-OpenCode user install"
        );
    }

    #[test]
    fn remove_deletes_opencode_user_install_when_xdg_config_home_is_a_symlink() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let real_xdg = tempfile::tempdir().unwrap();
        let link_parent = tempfile::tempdir().unwrap();
        let xdg_link = link_parent.path().join("xdg");
        std::os::unix::fs::symlink(real_xdg.path(), &xdg_link).unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", &xdg_link);

        let recorded = xdg_link.join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:xdg-link".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a symlinked XDG_CONFIG_HOME must not block removal of an OpenCode user install"
        );
    }

    #[test]
    fn remove_deletes_opencode_when_explicit_xdg_is_symlinked_default_config() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let config = home.path().join(".config");
        std::os::unix::fs::symlink(outside.path(), &config).unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", &config);

        let recorded = config.join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:explicit-config".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "XDG_CONFIG_HOME set to a symlinked $HOME/.config is an explicit anchor and must be removed"
        );
    }

    #[test]
    fn remove_deletes_recorded_retired_xdg_outside_home() {
        let _lock = env_lock();
        let home = tempfile::tempdir().unwrap();
        let current = tempfile::tempdir().unwrap();
        let retired = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", current.path());

        let recorded = retired.path().join("opencode/skills/plain-skill/SKILL.md");
        let neighbor = retired.path().join("opencode/skills/keep.txt");
        let unrecorded = current.path().join("opencode/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        write_skill_file(&neighbor);
        write_skill_file(&unrecorded);
        let skill = plain_user_skill(
            &["opencode"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:retired".to_string(),
            }],
        );

        remove_skill_files(&skill, &["opencode".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a recorded OpenCode install under a retired XDG root outside $HOME must be deleted"
        );
        assert!(
            neighbor.exists(),
            "removal stops at the skill directory, not the retired XDG root"
        );
        assert!(
            unrecorded.exists(),
            "an unrecorded file under the current XDG_CONFIG_HOME must survive"
        );
    }

    #[test]
    fn remove_deletes_project_install_when_project_root_is_a_symlink() {
        let real = tempfile::tempdir().unwrap();
        let link_parent = tempfile::tempdir().unwrap();
        let root = link_parent.path().join("project");
        std::os::unix::fs::symlink(real.path(), &root).unwrap();

        let recorded = root.join(".claude/skills/plain-skill/SKILL.md");
        write_skill_file(&recorded);
        let skill = plain_project_skill(
            &root,
            &["claude"],
            vec![InstalledFile {
                path: recorded.to_string_lossy().to_string(),
                hash: "sha256:root-link".to_string(),
            }],
        );

        remove_skill_files(&skill, &["claude".to_string()]).unwrap();
        assert!(
            !recorded.exists(),
            "a symlinked project root must not block removal inside that root"
        );
    }

    #[test]
    fn skipped_containment_keeps_file_and_harness_when_every_harness_is_selected() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("project");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join(".agents")).unwrap();

        let skipped = root.join(".agents/skills/plain-skill/SKILL.md");
        let deleted = root.join(".claude/skills/plain-skill/SKILL.md");
        write_skill_file(&skipped);
        write_skill_file(&deleted);
        let skipped_path = skipped.to_string_lossy().to_string();

        let skill = plain_project_skill(
            &root,
            &["claude", "opencode"],
            vec![
                InstalledFile {
                    path: skipped_path.clone(),
                    hash: "sha256:skipped".to_string(),
                },
                InstalledFile {
                    path: deleted.to_string_lossy().to_string(),
                    hash: "sha256:deleted".to_string(),
                },
            ],
        );
        let harnesses = vec!["claude".to_string(), "opencode".to_string()];
        remove_skill_files(&skill, &harnesses).unwrap();
        assert!(
            skipped.exists(),
            "the symlinked .agents file is outside the project and must stay on disk"
        );
        assert!(!deleted.exists(), "the contained claude file is deleted");

        let state_dir = tmp.path().join("state");
        let mut store = StateStore::open_at(&state_dir).unwrap();
        store.upsert(skill);
        let action = (store.skills()[0].clone(), harnesses);
        apply_removals_to_state(&mut store, vec![action]).unwrap();

        assert_eq!(
            store.skills().len(),
            1,
            "store.remove must not drop a skill that still has a skipped file"
        );
        let updated = &store.skills()[0];
        assert_eq!(updated.harnesses, vec!["opencode".to_string()]);
        assert!(updated.files.iter().any(|file| file.path == skipped_path));
        assert!(
            updated
                .files
                .iter()
                .all(|file| !file.path.contains(".claude"))
        );
    }
}

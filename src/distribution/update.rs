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

//! `skillprism update` command implementation.

use std::collections::HashMap;
use std::path::Path;
use std::{fs, io};

use crate::loader::{discover_asset_dirs, find_template_path};
use crate::registry::HarnessRegistry;
use crate::resolver::HarnessResolver;
use crate::router::{resolve_overwrite, resolve_skill_path};
use crate::state::{
    InstallScope, InstalledFile, InstalledSkill, SkillFormat, SourceType, StateStore, now_rfc3339,
};
use crate::types::ProjectError;

use super::add::InstallScopeArg;
use super::install::{
    InstallError, build_registry_for_harnesses, copy_dir, detect_format, discover_skill_dirs,
    install_scope_to_target, load_skill_into_temp_project, record_asset_hashes, sha256_bytes,
    skill_dir_name,
};
use super::network::{self, NetworkError};
use super::source::{ParsedSource, SourceParseError, parse_source};

/// Errors that can occur during update.
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum UpdateError {
    /// Source string could not be re-parsed.
    #[error("failed to re-parse source for `{skill}`: {source}")]
    SourceParse {
        skill: String,
        #[source]
        source: SourceParseError,
    },

    /// Network error during fetch.
    #[error(transparent)]
    Network(#[from] NetworkError),

    /// Install error.
    #[error(transparent)]
    Install(#[from] InstallError),

    /// Project error.
    #[error(transparent)]
    Project(#[from] ProjectError),

    /// I/O error.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// State store error.
    #[error(transparent)]
    State(#[from] crate::state::StateError),

    /// Rendering error.
    #[error("{0}")]
    Render(miette::Report),

    /// No matching skill found in fetched source.
    #[error("skill `{skill}` not found in the fetched source")]
    SkillNotFound { skill: String },
}

/// Runs the `update` command.
pub fn run_update(
    skills: &[String],
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
    diff: bool,
    force: bool,
) -> Result<(), miette::Report> {
    let mut store = StateStore::open().map_err(|e| miette::Report::new(UpdateError::from(e)))?;

    let all_skills = store.skills().to_vec();

    let candidates: Vec<InstalledSkill> = if skills.is_empty() {
        all_skills
    } else {
        skills
            .iter()
            .filter_map(|name| {
                let found = all_skills.iter().find(|s| s.name == *name);
                if found.is_none() {
                    eprintln!("Skill `{name}` is not installed, skipping");
                }
                found.cloned()
            })
            .collect()
    };

    let candidates = filter_candidates(candidates, target, harnesses);

    if candidates.is_empty() {
        println!("No installed skills to update.");
        return Ok(());
    }

    let harness_filter = harnesses.and_then(|h| {
        let parsed: Vec<_> = h
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    for skill in &candidates {
        update_skill(skill, &mut store, harness_filter.as_deref(), diff, force)?;
        if !diff {
            store
                .save()
                .map_err(|e| miette::Report::new(UpdateError::from(e)))?;
        }
    }

    Ok(())
}

fn filter_candidates(
    skills: Vec<InstalledSkill>,
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
) -> Vec<InstalledSkill> {
    skills
        .into_iter()
        .filter(|s| target.is_none_or(|t| InstallScope::from(t) == s.scope))
        .filter(|s| {
            harnesses.is_none_or(|h| {
                let wanted: Vec<_> = h.split(',').map(|x| x.trim().to_string()).collect();
                s.harnesses
                    .iter()
                    .any(|installed| wanted.contains(installed))
            })
        })
        .collect()
}

#[allow(clippy::too_many_lines)]
fn update_skill(
    old: &InstalledSkill,
    store: &mut StateStore,
    harness_filter: Option<&[String]>,
    diff: bool,
    force: bool,
) -> Result<(), miette::Report> {
    if old.source_type == SourceType::Local {
        println!("{} is a local skill, no remote to update", old.name);
        return Ok(());
    }

    let Some(r#ref) = &old.r#ref else {
        println!("{} has no git ref, cannot check for updates", old.name);
        return Ok(());
    };

    if network::is_sha_ref(r#ref) {
        println!("{} is pinned to commit {ref}, skipping update", old.name);
        return Ok(());
    }

    if let Some(resolved) = &old.resolved_ref {
        match network::git_remote_head(&old.source_url, r#ref) {
            Ok(Some(upstream_sha)) if upstream_sha == *resolved => {
                println!("{} is up to date", old.name);
                return Ok(());
            }
            Ok(Some(_)) => {} // different SHA → proceed with update
            Ok(None) => {
                eprintln!(
                    "Warning: could not resolve ref `{ref}` for {}, proceeding with fetch",
                    old.name
                );
            }
            Err(e) => {
                eprintln!(
                    "Warning: ls-remote failed for {} ({}), proceeding with fetch",
                    old.name, e
                );
            }
        }
    }

    let parsed = parse_source(&old.source).map_err(|e| {
        miette::Report::new(UpdateError::SourceParse {
            skill: old.name.clone(),
            source: e,
        })
    })?;

    let source_url = &old.source_url;
    let source_type = old.source_type;
    let skill_path = old.skill_path.as_deref().map(Path::new);

    let (source_path, cleanup_dir, resolved_ref) = match &parsed {
        ParsedSource::Local { path } => (path.clone(), None, None),
        ParsedSource::GitHub {
            url,
            r#ref: _,
            subpath,
            ..
        } => {
            let dir = network::fetch_git_repo(url, Some(r#ref)).map_err(UpdateError::from)?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.clone(), |s| dir.join(s));
            let head = network::git_dir_head(&dir).ok();
            (base, Some(dir), head)
        }
        ParsedSource::GitLab {
            url,
            r#ref: _,
            subpath,
            ..
        } => {
            let dir = network::fetch_git_repo(url, Some(r#ref)).map_err(UpdateError::from)?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.clone(), |s| dir.join(s));
            let head = network::git_dir_head(&dir).ok();
            (base, Some(dir), head)
        }
        ParsedSource::Git { url, r#ref: _ } => {
            let dir = network::fetch_git_repo(url, Some(r#ref)).map_err(UpdateError::from)?;
            let head = network::git_dir_head(&dir).ok();
            (dir.clone(), Some(dir), head)
        }
        ParsedSource::WellKnown { .. } => {
            return Err(miette::Report::msg(format!(
                "Cannot update well-known source `{}`",
                old.source
            )));
        }
    };

    let skill_dirs = discover_skill_dirs(&source_path).map_err(UpdateError::from)?;

    let matched_dir = skill_dirs
        .into_iter()
        .find(|d| skill_dir_name(d) == old.name)
        .ok_or_else(|| {
            miette::Report::new(UpdateError::SkillNotFound {
                skill: old.name.clone(),
            })
        })?;

    let format = detect_format(&matched_dir).map_err(UpdateError::from)?;

    let harnesses: Vec<String> = harness_filter.map_or_else(
        || old.harnesses.clone(),
        |filter| {
            old.harnesses
                .iter()
                .filter(|h| filter.contains(h))
                .cloned()
                .collect()
        },
    );
    if harnesses.is_empty() {
        println!("No matching harnesses to update for {}", old.name);
        return Ok(());
    }
    let project_root = match old.scope {
        InstallScope::Project => super::find_project_root().ok(),
        InstallScope::User => None,
    };

    let new_record = match format {
        SkillFormat::Skillprism => update_skillprism_skill(
            old,
            &matched_dir,
            source_url,
            source_type,
            r#ref,
            resolved_ref,
            skill_path,
            &harnesses,
            project_root.as_deref(),
            diff,
            force,
        )?,
        SkillFormat::Plain => update_plain_skill(
            old,
            &matched_dir,
            source_url,
            source_type,
            r#ref,
            resolved_ref,
            skill_path,
            &harnesses,
            project_root.as_deref(),
            diff,
            force,
        )?,
    };

    if !diff {
        store.upsert(new_record);
    }

    if let Some(dir) = cleanup_dir {
        let _ = network::cleanup_temp_dir(&dir);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_skillprism_skill(
    old: &InstalledSkill,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: &str,
    resolved_ref: Option<String>,
    skill_path: Option<&Path>,
    harnesses: &[String],
    project_root: Option<&Path>,
    diff: bool,
    force: bool,
) -> Result<InstalledSkill, miette::Report> {
    let (skill, _temp_project) =
        load_skill_into_temp_project(skill_dir, harnesses).map_err(UpdateError::from)?;
    let registry = build_registry_for_harnesses(harnesses);
    let old_files: HashMap<&str, &str> = old
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let mut new_files = Vec::new();
    let mut changed = false;
    let mut skip_all = false;

    let target = install_scope_to_target(old.scope);
    let project_root = project_root.unwrap_or_else(|| Path::new("."));

    for harness_id in harnesses {
        let pair = HarnessResolver::resolve_skill_harness(&skill, harness_id, &registry)
            .map_err(|e| miette::Report::msg(format!("resolve error for {}: {e}", old.name)))?;
        let output = crate::engine::Engine::render(&pair)
            .map_err(|e| miette::Report::new(UpdateError::Render(miette::Report::new(e))))?;

        let skill_path_buf = resolve_skill_path(project_root, &pair.harness, &old.name, target)
            .map_err(|e| miette::Report::msg(format!("path resolution error: {e}")))?;

        update_file_record(
            &skill_path_buf,
            &output.skill_content,
            &old_files,
            &mut new_files,
            &mut changed,
            diff,
            force,
            &mut skip_all,
        )?;

        for sidecar in &output.sidecars {
            let sidecar_path = skill_path_buf.parent().unwrap().join(&sidecar.filename);
            update_file_record(
                &sidecar_path,
                &sidecar.content,
                &old_files,
                &mut new_files,
                &mut changed,
                diff,
                force,
                &mut skip_all,
            )?;
        }

        for asset_dir in &pair.skill.asset_dirs {
            if asset_dir.exists() && !diff {
                copy_dir(
                    asset_dir,
                    &skill_path_buf
                        .parent()
                        .unwrap()
                        .join(asset_dir.file_name().unwrap()),
                    asset_dir,
                )
                .map_err(UpdateError::from)?;
                record_asset_hashes(asset_dir, skill_path_buf.parent().unwrap(), &mut new_files)
                    .map_err(UpdateError::from)?;
            }
        }
    }

    if !diff {
        if changed {
            println!("Updated {}", old.name);
        } else {
            println!("{} is up to date", old.name);
        }
    }

    Ok(InstalledSkill {
        name: old.name.clone(),
        source: old.source.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: Some(r#ref.to_string()),
        resolved_ref,
        skill_path: skill_path.map(|p| p.to_string_lossy().to_string()),
        scope: old.scope,
        harnesses: old.harnesses.clone(),
        format: SkillFormat::Skillprism,
        installed_at: old.installed_at.clone(),
        updated_at: if changed {
            now_rfc3339()
        } else {
            old.updated_at.clone()
        },
        files: new_files,
    })
}

#[allow(clippy::too_many_arguments)]
fn update_plain_skill(
    old: &InstalledSkill,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: &str,
    resolved_ref: Option<String>,
    skill_path: Option<&Path>,
    harnesses: &[String],
    project_root: Option<&Path>,
    diff: bool,
    force: bool,
) -> Result<InstalledSkill, miette::Report> {
    let template = find_template_path(skill_dir)
        .map_err(UpdateError::from)?
        .ok_or_else(|| {
            miette::Report::msg(format!("No SKILL.md or SKILL.md.j2 found in {}", old.name))
        })?;

    let old_files: HashMap<&str, &str> = old
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let mut new_files = Vec::new();
    let mut changed = false;
    let mut skip_all = false;

    let target = install_scope_to_target(old.scope);
    let project_root = project_root.unwrap_or_else(|| Path::new("."));

    for harness_id in harnesses {
        let harness = HarnessRegistry::with_builtins()
            .resolve(harness_id)
            .map_err(miette::Report::new)?;

        let skill_path_buf = resolve_skill_path(project_root, &harness, &old.name, target)
            .map_err(|e| miette::Report::msg(format!("path resolution error: {e}")))?;

        let content = fs::read(&template).map_err(UpdateError::from)?;
        let hash = format!("sha256:{}", sha256_bytes(&content));
        let path_str = skill_path_buf.to_string_lossy().to_string();

        let old_hash = old_files.get(path_str.as_str()).copied();
        let is_changed = old_hash.is_none_or(|h| h != hash.as_str());
        let mut written = false;

        if is_changed {
            changed = true;
            if diff {
                let existing = fs::read_to_string(&skill_path_buf).ok();
                let diff_output = crate::router::diff::compute_diff(
                    existing.as_deref(),
                    &String::from_utf8_lossy(&content),
                    &path_str,
                );
                print_diff_output(&diff_output);
            } else if resolve_overwrite(&skill_path_buf, force, &mut skip_all, &mut Vec::new()) {
                crate::router::atomic_write_bytes(&skill_path_buf, &content)
                    .map_err(|e| miette::Report::msg(format!("write error: {e}")))?;
                written = true;
            }
        }

        if written || !is_changed {
            new_files.push(InstalledFile {
                path: path_str,
                hash,
            });
        } else if let Some(old_hash) = old_hash {
            new_files.push(InstalledFile {
                path: path_str,
                hash: old_hash.to_string(),
            });
        }

        let skill_dir_out = skill_path_buf.parent().unwrap().to_path_buf();
        let asset_dirs = discover_asset_dirs(skill_dir).map_err(UpdateError::from)?;
        for asset_dir in &asset_dirs {
            if asset_dir.exists() {
                let dir_name = asset_dir.file_name().unwrap().to_string_lossy().to_string();
                if !diff {
                    copy_dir(asset_dir, &skill_dir_out.join(&dir_name), asset_dir)
                        .map_err(UpdateError::from)?;
                    record_asset_hashes(asset_dir, &skill_dir_out, &mut new_files)
                        .map_err(UpdateError::from)?;
                }
            }
        }
    }

    if !diff {
        if changed {
            println!("Updated {}", old.name);
        } else {
            println!("{} is up to date", old.name);
        }
    }

    Ok(InstalledSkill {
        name: old.name.clone(),
        source: old.source.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: Some(r#ref.to_string()),
        resolved_ref,
        skill_path: skill_path.map(|p| p.to_string_lossy().to_string()),
        scope: old.scope,
        harnesses: old.harnesses.clone(),
        format: SkillFormat::Plain,
        installed_at: old.installed_at.clone(),
        updated_at: if changed {
            now_rfc3339()
        } else {
            old.updated_at.clone()
        },
        files: new_files,
    })
}

fn write_file_with_overwrite(
    path: &Path,
    content: &str,
    force: bool,
    skip_all: &mut bool,
) -> Result<bool, miette::Report> {
    let mut skipped = Vec::new();
    if resolve_overwrite(path, force, skip_all, &mut skipped) {
        crate::router::atomic_write(path, content)
            .map_err(|e| miette::Report::msg(format!("write error: {e}")))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[allow(clippy::too_many_arguments)]
fn update_file_record(
    path: &Path,
    content: &str,
    old_files: &HashMap<&str, &str>,
    new_files: &mut Vec<InstalledFile>,
    changed: &mut bool,
    diff: bool,
    force: bool,
    skip_all: &mut bool,
) -> Result<(), miette::Report> {
    let hash = format!("sha256:{}", sha256_bytes(content.as_bytes()));
    let path_str = path.to_string_lossy().to_string();
    let old_hash = old_files.get(path_str.as_str()).copied();
    let is_changed = old_hash.is_none_or(|h| h != hash.as_str());
    let mut written = false;

    if is_changed {
        *changed = true;
        if diff {
            print_file_diff(path, content, &path_str);
        } else {
            written = write_file_with_overwrite(path, content, force, skip_all)?;
        }
    }

    if written || !is_changed {
        new_files.push(InstalledFile {
            path: path_str,
            hash,
        });
    } else if let Some(old_hash) = old_hash {
        // Declined to overwrite: keep the old hash so the next update still
        // sees this file as changed.
        new_files.push(InstalledFile {
            path: path_str,
            hash: old_hash.to_string(),
        });
    }

    Ok(())
}

fn print_file_diff(path: &Path, new_content: &str, path_display: &str) {
    let existing = fs::read_to_string(path).ok();
    let diff_output =
        crate::router::diff::compute_diff(existing.as_deref(), new_content, path_display);
    print_diff_output(&diff_output);
}

fn print_diff_output(output: &crate::router::diff::DiffOutput) {
    if output.hunks.is_empty() && output.stats.additions == 0 && output.stats.deletions == 0 {
        return;
    }
    println!("{}", output.header);
    print!("{}", output.hunks);
    let stats = &output.stats;
    if stats.is_new_file {
        println!("  (new file: +{} lines)", stats.additions);
    } else if stats.additions > 0 || stats.deletions > 0 {
        println!(
            "  (+{additions}, -{deletions} lines)",
            additions = stats.additions,
            deletions = stats.deletions
        );
    }
}

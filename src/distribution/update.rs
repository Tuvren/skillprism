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
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::loader::{discover_asset_dirs, find_template_path};
use crate::registry::HarnessRegistry;
use crate::resolver::HarnessResolver;
use crate::router::{resolve_overwrite, resolve_sidecar_path, resolve_skill_path};
use crate::state::{
    InstallScope, InstalledFile, InstalledSkill, SkillFormat, SourceType, StateStore, now_rfc3339,
};
use crate::types::ProjectError;

use super::add::InstallScopeArg;
use super::install::{
    InstallError, build_registry_for_harnesses, copy_dir, detect_format, discover_skill_dirs,
    install_scope_to_target, load_skill_into_temp_project, sha256_bytes, sha256_file,
    skill_dir_name, validate_pairs, walk_files,
};
use super::network::{self, NetworkError};
use super::source::{ParsedSource, SourceParseError, parse_source};

/// Caches fetched git repositories for the lifetime of an `update` command so
/// multiple installed skills that share the same source are cloned only once.
struct CloneCache {
    dirs: std::collections::HashMap<String, PathBuf>,
}

impl CloneCache {
    fn new() -> Self {
        Self {
            dirs: std::collections::HashMap::new(),
        }
    }

    fn fetch(&mut self, url: &str, r#ref: &str) -> Result<&Path, NetworkError> {
        let key = format!("{url}#{ref}");
        if !self.dirs.contains_key(&key) {
            let dir = network::fetch_git_repo(url, Some(r#ref))?;
            self.dirs.insert(key.clone(), dir);
        }
        Ok(self.dirs.get(&key).unwrap())
    }
}

impl Drop for CloneCache {
    fn drop(&mut self) {
        for dir in self.dirs.values() {
            let _ = network::cleanup_temp_dir(dir);
        }
    }
}

/// Errors that can occur during update.
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum UpdateError {
    /// Source string could not be re-parsed.
    #[error("failed to re-parse source for `{skill}`: {source}")]
    #[diagnostic(help(
        "The stored source string is invalid; re-add the skill with a valid source."
    ))]
    SourceParse {
        skill: String,
        #[source]
        source: SourceParseError,
    },

    /// Network error during fetch.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Network(#[from] NetworkError),

    /// Install error.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Install(#[from] InstallError),

    /// Project error.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Project(#[from] ProjectError),

    /// I/O error.
    #[error("I/O error: {0}")]
    #[diagnostic(help(
        "Check filesystem permissions and available disk space for the target path."
    ))]
    Io(#[from] io::Error),

    /// State store error.
    #[error(transparent)]
    #[diagnostic(transparent)]
    State(#[from] crate::state::StateError),

    /// Rendering error.
    #[error("{0}")]
    #[diagnostic(help(
        "Inspect the skill template for invalid MiniJinja syntax or unknown variables."
    ))]
    Render(miette::Report),

    /// No matching skill found in fetched source.
    #[error("skill `{skill}` not found in the fetched source")]
    #[diagnostic(help(
        "The skill may have been renamed or removed upstream; run `skillprism remove` if it no longer exists."
    ))]
    SkillNotFound { skill: String },

    /// Failed to resolve an output path for a skill file.
    #[error("failed to resolve output path: {detail}")]
    #[diagnostic(help(
        "Check the target scope and project layout; ensure the destination is writable."
    ))]
    PathResolution { detail: String },

    /// Failed to write a rendered skill file.
    #[error("failed to write skill file: {detail}")]
    #[diagnostic(help("Ensure the destination directory exists and is writable."))]
    Write { detail: String },

    /// Failed to resolve a harness for the skill.
    #[error("failed to resolve harness for `{skill}`: {detail}")]
    #[diagnostic(help(
        "Run `skillprism completions`/docs to see valid harness ids, or check `--harnesses`."
    ))]
    HarnessResolution { skill: String, detail: String },

    /// The fetched skill directory has no SKILL.md or SKILL.md.j2.
    #[error("no SKILL.md or SKILL.md.j2 found in `{skill}`")]
    #[diagnostic(help(
        "The upstream skill is missing its entry file; it may be malformed or moved."
    ))]
    NoSkillFile { skill: String },

    /// The stored source is a well-known index, which cannot be updated.
    #[error("cannot update well-known source `{source_input}`")]
    #[diagnostic(help(
        "Well-known index installs are not yet supported; remove and re-add from a git source."
    ))]
    WellKnownUnsupported { source_input: String },
}

/// Runs the `update` command.
pub fn run_update(
    skills: &[String],
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
    diff: bool,
    force: bool,
    verbose: bool,
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

    if verbose {
        eprintln!(
            "[update] {count} skill(s) selected for update",
            count = candidates.len()
        );
    }

    if candidates.is_empty() {
        eprintln!("No installed skills to update.");
        return Ok(());
    }

    let harness_filter = harnesses.and_then(|h| {
        let parsed = super::parse_harness_list(h);
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    });

    let mut clone_cache = CloneCache::new();
    for skill in &candidates {
        update_skill(
            skill,
            &mut store,
            harness_filter.as_deref(),
            diff,
            force,
            &mut clone_cache,
        )?;
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
        .filter(|s| super::scope_harness_matches(s, target, harnesses))
        .collect()
}

/// Reports the outcome of an update to stderr (suppressed in `--diff` mode,
/// where the patch is the output). Shared by the skillprism and plain paths.
fn report_update_result(diff: bool, changed: bool, name: &str) {
    if diff {
        return;
    }
    if changed {
        eprintln!("Updated {name}");
    } else {
        eprintln!("{name} is up to date");
    }
}

// reason: linear per-skill update pipeline (ref check → fetch → per-harness
// render/compare) kept as one readable unit.
#[allow(clippy::too_many_lines)]
fn update_skill(
    old: &InstalledSkill,
    store: &mut StateStore,
    harness_filter: Option<&[String]>,
    diff: bool,
    force: bool,
    clone_cache: &mut CloneCache,
) -> Result<(), miette::Report> {
    if old.source_type == SourceType::Local {
        eprintln!("{} is a local skill, no remote to update", old.name);
        return Ok(());
    }

    let Some(r#ref) = &old.r#ref else {
        eprintln!("{} has no git ref, cannot check for updates", old.name);
        return Ok(());
    };

    if network::is_sha_ref(r#ref) {
        eprintln!("{} is pinned to commit {ref}, skipping update", old.name);
        return Ok(());
    }

    if let Some(resolved) = &old.resolved_ref {
        match network::git_remote_head(&old.source_url, r#ref) {
            Ok(Some(upstream_sha)) if upstream_sha == *resolved => {
                eprintln!("{} is up to date", old.name);
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

    let (source_path, resolved_ref) = match &parsed {
        ParsedSource::Local { path } => (path.clone(), None),
        ParsedSource::GitHub {
            url,
            r#ref: _,
            subpath,
            ..
        } => {
            let dir = clone_cache.fetch(url, r#ref).map_err(UpdateError::from)?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.to_path_buf(), |s| dir.join(s));
            let head = network::git_dir_head(dir).ok();
            (base, head)
        }
        ParsedSource::GitLab {
            url,
            r#ref: _,
            subpath,
            ..
        } => {
            let dir = clone_cache.fetch(url, r#ref).map_err(UpdateError::from)?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.to_path_buf(), |s| dir.join(s));
            let head = network::git_dir_head(dir).ok();
            (base, head)
        }
        ParsedSource::Git { url, r#ref: _ } => {
            let dir = clone_cache.fetch(url, r#ref).map_err(UpdateError::from)?;
            let head = network::git_dir_head(dir).ok();
            (dir.to_path_buf(), head)
        }
        ParsedSource::WellKnown { .. } => {
            return Err(miette::Report::new(UpdateError::WellKnownUnsupported {
                source_input: old.source.clone(),
            }));
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
        eprintln!("No matching harnesses to update for {}", old.name);
        return Ok(());
    }
    let project_root: Option<PathBuf> = match old.scope {
        InstallScope::Project => old
            .project_root
            .clone()
            .map(PathBuf::from)
            .or_else(|| super::find_project_root().ok()),
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

    Ok(())
}

// reason: mirrors the install path — threads source-provenance fields through
// the per-harness render/update loop; `SourceMeta` bundling is a tracked follow-up.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
    let mut changed = false;
    let mut skip_all = false;

    let target = install_scope_to_target(old.scope);
    // For user-scope skills there is no project root; `resolve_skill_path`
    // ignores this argument for `TargetScope::User`, so `"."` is an unused
    // placeholder rather than a meaningful path.
    let project_root = project_root.unwrap_or_else(|| Path::new("."));

    // Retain per-file records for harnesses that are not being updated; they
    // will be replaced with fresh records for the filtered harnesses below.
    let updated_prefixes =
        collect_updated_prefixes(harnesses, &registry, project_root, &old.name, target)?;
    let mut new_files: Vec<InstalledFile> = old
        .files
        .iter()
        // Component-aware prefix match (consistent with remove.rs) so a skill
        // dir like `.../skills/foo` doesn't spuriously match `.../skills/foo-bar`.
        .filter(|f| {
            !updated_prefixes
                .iter()
                .any(|p| Path::new(&f.path).starts_with(p))
        })
        .cloned()
        .collect();

    // Resolve every harness pair and validate before writing anything, so an
    // undefined variable or reserved-name collision fails the update with no
    // output written (DIST-I002) — matching install and the `build` command.
    let mut pairs = Vec::new();
    for harness_id in harnesses {
        pairs.push(
            HarnessResolver::resolve_skill_harness(&skill, harness_id, &registry).map_err(|e| {
                miette::Report::new(UpdateError::HarnessResolution {
                    skill: old.name.clone(),
                    detail: e.to_string(),
                })
            })?,
        );
    }
    let pairs = validate_pairs(&old.name, pairs).map_err(miette::Report::new)?;

    for pair in &pairs {
        let harness_id = &pair.harness.id;
        let output = crate::engine::Engine::render(pair)
            .map_err(|e| miette::Report::new(UpdateError::Render(miette::Report::new(e))))?;

        let skill_path_buf = resolve_skill_path(project_root, &pair.harness, &old.name, target)
            .map_err(|e| {
                miette::Report::new(UpdateError::PathResolution {
                    detail: e.to_string(),
                })
            })?;

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
            let sidecar_path = resolve_sidecar_path(
                skill_path_buf.parent().unwrap(),
                sidecar.output_dir.as_deref(),
                &sidecar.filename,
                &old.name,
                harness_id,
            )
            .map_err(|e| {
                miette::Report::new(UpdateError::PathResolution {
                    detail: format!("sidecar for {harness_id}: {e}"),
                })
            })?;
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
            if asset_dir.exists() {
                update_asset_dir(
                    asset_dir,
                    skill_path_buf.parent().unwrap(),
                    &old_files,
                    &mut new_files,
                    &mut changed,
                    diff,
                    force,
                    &mut skip_all,
                )?;
            }
        }
    }

    report_update_result(diff, changed, &old.name);

    Ok(InstalledSkill {
        name: old.name.clone(),
        source: old.source.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: Some(r#ref.to_string()),
        resolved_ref,
        skill_path: skill_path.map(|p| p.to_string_lossy().to_string()),
        project_root: old.project_root.clone(),
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

// reason: mirrors `update_skillprism_skill` — threads source-provenance fields
// through the per-harness update loop; `SourceMeta` bundling is a tracked follow-up.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
            miette::Report::new(UpdateError::NoSkillFile {
                skill: old.name.clone(),
            })
        })?;

    let old_files: HashMap<&str, &str> = old
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let mut changed = false;
    let mut skip_all = false;

    let target = install_scope_to_target(old.scope);
    // For user-scope skills there is no project root; `resolve_skill_path`
    // ignores this argument for `TargetScope::User`, so `"."` is an unused
    // placeholder rather than a meaningful path.
    let project_root = project_root.unwrap_or_else(|| Path::new("."));

    // Retain per-file records for harnesses that are not being updated.
    let registry = HarnessRegistry::with_builtins();
    let updated_prefixes =
        collect_updated_prefixes(harnesses, &registry, project_root, &old.name, target)?;
    let mut new_files: Vec<InstalledFile> = old
        .files
        .iter()
        // Component-aware prefix match (consistent with remove.rs) so a skill
        // dir like `.../skills/foo` doesn't spuriously match `.../skills/foo-bar`.
        .filter(|f| {
            !updated_prefixes
                .iter()
                .any(|p| Path::new(&f.path).starts_with(p))
        })
        .cloned()
        .collect();

    for harness_id in harnesses {
        let harness = registry.resolve(harness_id).map_err(miette::Report::new)?;

        let skill_path_buf = resolve_skill_path(project_root, &harness, &old.name, target)
            .map_err(|e| {
                miette::Report::new(UpdateError::PathResolution {
                    detail: e.to_string(),
                })
            })?;

        let content = fs::read(&template).map_err(UpdateError::from)?;
        let hash = format!("sha256:{}", sha256_bytes(&content));
        let path_str = skill_path_buf.to_string_lossy().to_string();

        let old_hash = old_files.get(path_str.as_str()).copied();
        let is_changed = old_hash.is_none_or(|h| h != hash.as_str());
        let mut written = false;

        if is_changed {
            if diff {
                let existing = fs::read_to_string(&skill_path_buf).ok();
                let diff_output = crate::router::diff::compute_diff(
                    existing.as_deref(),
                    &String::from_utf8_lossy(&content),
                    &path_str,
                );
                print_diff_output(&diff_output);
            } else if resolve_overwrite(&skill_path_buf, force, &mut skip_all, &mut Vec::new()) {
                crate::router::atomic_write_bytes(&skill_path_buf, &content).map_err(|e| {
                    miette::Report::new(UpdateError::Write {
                        detail: e.to_string(),
                    })
                })?;
                written = true;
            }
            if written {
                changed = true;
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

        let skill_dir_out = skill_path_buf.parent().unwrap();
        let asset_dirs = discover_asset_dirs(skill_dir).map_err(UpdateError::from)?;
        for asset_dir in &asset_dirs {
            if asset_dir.exists() {
                update_asset_dir(
                    asset_dir,
                    skill_dir_out,
                    &old_files,
                    &mut new_files,
                    &mut changed,
                    diff,
                    force,
                    &mut skip_all,
                )?;
            }
        }
    }

    report_update_result(diff, changed, &old.name);

    Ok(InstalledSkill {
        name: old.name.clone(),
        source: old.source.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: Some(r#ref.to_string()),
        resolved_ref,
        skill_path: skill_path.map(|p| p.to_string_lossy().to_string()),
        project_root: old.project_root.clone(),
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

// reason: asset-dir diff/copy needs source+dest dirs, the old-file hash map, the
// accumulators, and the diff/force/skip flags together in a single pass.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn update_asset_dir(
    src_dir: &Path,
    dst_base: &Path,
    old_files: &HashMap<&str, &str>,
    new_files: &mut Vec<InstalledFile>,
    changed: &mut bool,
    diff: bool,
    force: bool,
    skip_all: &mut bool,
) -> Result<(), UpdateError> {
    let dir_name = src_dir.file_name().ok_or_else(|| {
        UpdateError::Install(InstallError::Project(ProjectError::ConfigRead {
            path: src_dir.to_string_lossy().to_string(),
            source: io::Error::other("asset directory has no name"),
        }))
    })?;
    let dst_dir = dst_base.join(dir_name);

    let mut expected = Vec::new();
    for src_file in walk_files(src_dir)? {
        let rel = src_file
            .strip_prefix(src_dir)
            .map_err(|e| UpdateError::Io(io::Error::other(e.to_string())))?;
        let dst_file = dst_dir.join(rel);
        let hash = format!("sha256:{}", sha256_file(&src_file)?);
        expected.push((src_file, dst_file, hash));
    }

    let expected_paths: std::collections::HashSet<String> = expected
        .iter()
        .map(|(_, p, _)| p.to_string_lossy().to_string())
        .collect();

    let mut removed: Vec<String> = Vec::new();
    for path_str in old_files.keys() {
        let p = Path::new(path_str);
        if p.starts_with(&dst_dir) && !expected_paths.contains(*path_str) {
            removed.push((*path_str).to_string());
        }
    }

    let mut any_changed = !removed.is_empty();
    for (_, dst_file, hash) in &expected {
        let path_str = dst_file.to_string_lossy().to_string();
        let old_hash = old_files.get(path_str.as_str()).copied();
        if old_hash.is_none_or(|h| h != hash.as_str()) {
            any_changed = true;
            break;
        }
    }

    if !any_changed {
        for (_, dst_file, _) in expected {
            let path_str = dst_file.to_string_lossy().to_string();
            if let Some(old_hash) = old_files.get(path_str.as_str()).copied() {
                new_files.push(InstalledFile {
                    path: path_str,
                    hash: old_hash.to_string(),
                });
            }
        }
        return Ok(());
    }

    let mut copied = false;
    if diff {
        for (_, dst_file, hash) in &expected {
            let path_str = dst_file.to_string_lossy().to_string();
            let old_hash = old_files.get(path_str.as_str()).copied();
            if old_hash.is_none_or(|h| h != hash.as_str()) {
                let marker = if old_hash.is_some() {
                    "changed"
                } else {
                    "added"
                };
                println!("  ({marker} asset) {path_str}");
            }
        }
        for path_str in &removed {
            println!("  (removed asset) {path_str}");
        }
    } else {
        let mut skipped = Vec::new();
        copied = resolve_overwrite(&dst_dir, force, skip_all, &mut skipped);
        if copied {
            copy_dir(src_dir, &dst_dir, src_dir)?;
            for path_str in &removed {
                let _ = fs::remove_file(path_str);
            }
        }
    }

    if copied {
        *changed = true;
        for (_, dst_file, hash) in expected {
            new_files.push(InstalledFile {
                path: dst_file.to_string_lossy().to_string(),
                hash,
            });
        }
    } else {
        // Declined to overwrite (or diff mode): keep the old hashes so the next
        // update still detects any asset drift.
        for (_, dst_file, _) in expected {
            let path_str = dst_file.to_string_lossy().to_string();
            if let Some(old_hash) = old_files.get(path_str.as_str()).copied() {
                new_files.push(InstalledFile {
                    path: path_str,
                    hash: old_hash.to_string(),
                });
            }
        }
        // Also preserve records for assets that no longer exist in the source
        // so the state continues to match the on-disk files.
        for path_str in &removed {
            if let Some(old_hash) = old_files.get(path_str.as_str()).copied() {
                new_files.push(InstalledFile {
                    path: path_str.clone(),
                    hash: old_hash.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn write_file_with_overwrite(
    path: &Path,
    content: &str,
    force: bool,
    skip_all: &mut bool,
) -> Result<bool, miette::Report> {
    let mut skipped = Vec::new();
    if resolve_overwrite(path, force, skip_all, &mut skipped) {
        crate::router::atomic_write(path, content).map_err(|e| {
            miette::Report::new(UpdateError::Write {
                detail: e.to_string(),
            })
        })?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn skill_dir_prefix(
    project_root: &Path,
    harness: &crate::registry::HarnessDefinition,
    skill_name: &str,
    target: crate::cli::TargetScope,
) -> Result<String, miette::Report> {
    let skill_path =
        resolve_skill_path(project_root, harness, skill_name, target).map_err(|e| {
            miette::Report::new(UpdateError::PathResolution {
                detail: e.to_string(),
            })
        })?;
    Ok(skill_path
        .parent()
        .expect("skill path has parent")
        .to_string_lossy()
        .to_string())
}

fn collect_updated_prefixes(
    harnesses: &[String],
    registry: &HarnessRegistry,
    project_root: &Path,
    skill_name: &str,
    target: crate::cli::TargetScope,
) -> Result<Vec<String>, miette::Report> {
    harnesses
        .iter()
        .map(|h| {
            let harness = registry.resolve(h).map_err(|e| {
                miette::Report::new(UpdateError::HarnessResolution {
                    skill: skill_name.to_string(),
                    detail: e.to_string(),
                })
            })?;
            skill_dir_prefix(project_root, &harness, skill_name, target)
        })
        .collect::<Result<Vec<_>, _>>()
}

// reason: single-file change detection needs the path, content, old-hash map,
// output accumulators, and the diff/force/skip flags together.
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
        if diff {
            print_file_diff(path, content, &path_str);
        } else {
            written = write_file_with_overwrite(path, content, force, skip_all)?;
        }
        if written {
            *changed = true;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use tempfile::TempDir;

    fn leak_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    #[test]
    fn update_file_record_writes_and_hashes_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("SKILL.md");
        fs::write(&path, "Version: A").unwrap();

        let mut new_files = Vec::new();
        let mut changed = false;
        let mut skip_all = false;

        update_file_record(
            &path,
            "Version: B",
            &HashMap::new(),
            &mut new_files,
            &mut changed,
            false,
            true,
            &mut skip_all,
        )
        .unwrap();

        assert!(changed, "changed should be true when content differs");
        assert_eq!(fs::read_to_string(&path).unwrap(), "Version: B");
        assert_eq!(new_files.len(), 1);
        assert_eq!(new_files[0].path, path.to_string_lossy());
        assert!(new_files[0].hash.starts_with("sha256:"));
    }

    #[test]
    fn update_file_record_keeps_old_hash_when_declined() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("SKILL.md");
        fs::write(&path, "Version: A").unwrap();

        let path_str = leak_str(path.to_string_lossy().to_string());
        let old_hash = leak_str(format!("sha256:{}", sha256_bytes(b"Version: A")));
        let old_files: HashMap<&str, &str> = std::iter::once((path_str, old_hash)).collect();

        let mut new_files = Vec::new();
        let mut changed = false;
        let mut skip_all = true; // simulate decline without prompting

        update_file_record(
            &path,
            "Version: B",
            &old_files,
            &mut new_files,
            &mut changed,
            false,
            false, // force false so resolve_overwrite checks skip_all
            &mut skip_all,
        )
        .unwrap();

        assert!(
            !changed,
            "changed should stay false when overwrite is declined"
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "Version: A");
        assert_eq!(new_files[0].hash, old_hash);
    }

    #[test]
    fn update_asset_dir_copies_new_assets_and_records_hashes() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("assets");
        let dst_base = tmp.path().join("out");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("icon.svg"), "<svg>A</svg>").unwrap();

        let mut new_files = Vec::new();
        let mut changed = false;
        let mut skip_all = false;

        update_asset_dir(
            &src_dir,
            &dst_base,
            &HashMap::new(),
            &mut new_files,
            &mut changed,
            false,
            true,
            &mut skip_all,
        )
        .unwrap();

        assert!(changed);
        let dst = dst_base.join("assets/icon.svg");
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "<svg>A</svg>");
        assert_eq!(new_files.len(), 1);
        assert!(new_files[0].hash.starts_with("sha256:"));
    }

    #[test]
    fn update_asset_dir_prunes_removed_assets() {
        let tmp = TempDir::new().unwrap();
        let src_dir = tmp.path().join("assets");
        let dst_base = tmp.path().join("out");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(dst_base.join("assets")).unwrap();

        // Source no longer has old-icon.svg.
        fs::write(src_dir.join("icon.svg"), "<svg>A</svg>").unwrap();

        // But the destination still has it from a previous install.
        let old_path = dst_base.join("assets/old-icon.svg");
        fs::write(&old_path, "old").unwrap();

        let old_path_str = leak_str(old_path.to_string_lossy().to_string());
        let icon_path = dst_base.join("assets/icon.svg");
        let icon_path_str = leak_str(icon_path.to_string_lossy().to_string());
        let old_hash = leak_str(format!("sha256:{}", sha256_bytes(b"old")));
        let old_files: HashMap<&str, &str> = [(old_path_str, old_hash), (icon_path_str, old_hash)]
            .into_iter()
            .collect();

        let mut new_files = Vec::new();
        let mut changed = false;
        let mut skip_all = false;

        update_asset_dir(
            &src_dir,
            &dst_base,
            &old_files,
            &mut new_files,
            &mut changed,
            false,
            true,
            &mut skip_all,
        )
        .unwrap();

        assert!(changed);
        assert!(
            !old_path.exists(),
            "removed asset should be deleted from disk"
        );
        assert!(icon_path.exists());
        assert!(
            new_files.iter().all(|f| f.path != old_path_str),
            "removed asset record should not appear in new state"
        );
    }
}

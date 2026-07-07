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

//! Shared install logic used by `add` and `update`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use sha2::Digest;
use thiserror::Error;

use crate::cli::TargetScope;
use crate::engine::Engine;
use crate::loader::ProjectLoader;
use crate::registry::HarnessRegistry;
use crate::resolver::{HarnessResolver, ResolveError};
use crate::router::Router;
use crate::state::{
    InstallScope, InstalledFile, InstalledSkill, SkillFormat, SourceType, now_rfc3339,
};
use crate::types::{ProjectError, SkillModel};

use super::network::{self, NetworkError};
use super::source::{ParsedSource, SourceParseError, mask_credentials};

/// Errors that can occur during installation.
#[derive(Debug, Diagnostic, Error)]
pub enum InstallError {
    /// The source string could not be parsed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    SourceParse(#[from] SourceParseError),

    /// The network fetch failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Network(#[from] NetworkError),

    /// A project error occurred while loading skill configuration.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Project(#[from] ProjectError),

    /// A runtime I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Rendering a skill template failed.
    #[error("{0}")]
    Render(miette::Report),

    /// State persistence failed.
    #[error(transparent)]
    #[diagnostic(transparent)]
    State(#[from] crate::state::StateError),

    /// No skills were found in the source.
    #[error("no skillprism-format or plain-format skills found in {source_input}")]
    #[diagnostic(help(
        "Ensure the source contains skill directories with SKILL.md or SKILL.md.j2 files."
    ))]
    NoSkillsFound { source_input: String },

    /// A skill manifest is malformed.
    #[error("{path}: {detail}")]
    #[diagnostic(help(
        "Either add `skillprism: '1'` to declare skillprism-format, or remove skill.yaml to declare plain-format."
    ))]
    MalformedManifest { path: String, detail: String },

    /// A requested skill filter did not match any discovered skill.
    #[error("skill `{skill}` not found in source `{source_input}`")]
    SkillNotFound { source_input: String, skill: String },

    /// This source type is not yet supported for installation.
    #[error("{source_input}: {detail}")]
    #[diagnostic(help("{help}"))]
    UnsupportedSource {
        source_input: String,
        detail: String,
        help: String,
    },
}

/// Context for a single install/update operation.
#[derive(Debug, Clone)]
pub struct InstallContext {
    /// The exact source string the user supplied.
    pub source_input: String,
    /// Parsed source.
    pub parsed: ParsedSource,
    /// Target install scope.
    pub target_scope: InstallScope,
    /// Selected harness IDs.
    pub harnesses: Vec<String>,
    /// Project root, if known.
    pub project_root: Option<PathBuf>,
    /// Whether to overwrite existing files without prompting.
    pub force: bool,
    /// Optional skill name filter for multi-skill sources.
    pub skill_filter: Option<String>,
}

/// Result of installing a single skill.
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Installed skill metadata, ready for the state file.
    pub record: InstalledSkill,
}

/// Installs all skills from a source into the target scope.
///
/// `on_installed` is called after each individual skill is installed so callers
/// can persist incremental state. If a later skill fails, earlier skills that
/// were successfully installed remain recorded.
pub fn install_source(
    ctx: &InstallContext,
    mut on_installed: impl FnMut(&InstalledSkill) -> Result<(), InstallError>,
) -> Result<Vec<InstallResult>, InstallError> {
    let (source_path, cleanup_dir, source_url, source_type, r#ref, skill_path) = match &ctx.parsed {
        ParsedSource::Local { path } => (
            path.clone(),
            None,
            path.to_string_lossy().to_string(),
            SourceType::Local,
            None,
            None,
        ),
        ParsedSource::GitHub {
            url,
            r#ref,
            subpath,
            ..
        } => {
            let dir = network::fetch_git_repo(url, r#ref.as_deref())?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.clone(), |s| dir.join(s));
            (
                base,
                Some(dir),
                url.clone(),
                SourceType::GitHub,
                r#ref.clone(),
                subpath.clone(),
            )
        }
        ParsedSource::GitLab {
            url,
            r#ref,
            subpath,
            ..
        } => {
            let dir = network::fetch_git_repo(url, r#ref.as_deref())?;
            let base = subpath
                .as_ref()
                .map_or_else(|| dir.clone(), |s| dir.join(s));
            (
                base,
                Some(dir),
                url.clone(),
                SourceType::GitLab,
                r#ref.clone(),
                subpath.clone(),
            )
        }
        ParsedSource::Git { url, r#ref } => {
            let dir = network::fetch_git_repo(url, r#ref.as_deref())?;
            (
                dir.clone(),
                Some(dir),
                url.clone(),
                SourceType::Git,
                r#ref.clone(),
                None,
            )
        }
        ParsedSource::WellKnown { url, index_path } => {
            return install_from_well_known(ctx, url, index_path);
        }
    };

    let resolved_ref = cleanup_dir
        .as_deref()
        .and_then(|dir| network::git_dir_head(dir).ok());

    // When the user did not specify a ref for a remote git source, resolve the
    // remote's default branch so that later `skillprism update` has a named ref
    // to check. If the remote does not advertise a default branch, fall back to
    // the cloned HEAD SHA so the record is still updateable.
    let effective_ref = r#ref.or_else(|| {
        // Only remote git sources (which produced a clone dir) get a resolved ref.
        cleanup_dir.as_ref()?;
        // Fall back to the cloned HEAD SHA only when it is a real, non-empty
        // ref; never record an empty string, which a later `update` would then
        // feed to `ls-remote`/clone as if it were a valid ref.
        let head_fallback = || resolved_ref.clone().filter(|r| !r.is_empty());
        match network::git_default_branch(&source_url) {
            Ok(Some(branch)) => Some(branch),
            Ok(None) => {
                eprintln!(
                    "Warning: no default branch advertised by {source_url}; using resolved HEAD as ref"
                );
                head_fallback()
            }
            Err(e) => {
                eprintln!("Warning: could not resolve default branch for {source_url}: {e}; using resolved HEAD as ref");
                head_fallback()
            }
        }
    });

    let result = install_discovered_skills(
        ctx,
        &source_path,
        &source_url,
        source_type,
        effective_ref.as_ref(),
        resolved_ref,
        skill_path.as_ref(),
        &mut on_installed,
    );

    if let Some(dir) = cleanup_dir {
        let _ = network::cleanup_temp_dir(&dir);
    }

    result
}

#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
#[allow(clippy::too_many_arguments)]
fn install_discovered_skills(
    ctx: &InstallContext,
    source_path: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
    resolved_ref: Option<String>,
    skill_path: Option<&String>,
    on_installed: &mut impl FnMut(&InstalledSkill) -> Result<(), InstallError>,
) -> Result<Vec<InstallResult>, InstallError> {
    let skill_dirs = discover_skill_dirs(source_path)?;
    if skill_dirs.is_empty() {
        return Err(InstallError::NoSkillsFound {
            source_input: ctx.source_input.clone(),
        });
    }

    let filter = ctx
        .skill_filter
        .clone()
        .or_else(|| skill_filter_from_parsed(&ctx.parsed));
    let filtered = if let Some(filter) = filter {
        let matched: Vec<_> = skill_dirs
            .into_iter()
            .filter(|d| skill_dir_name(d) == filter)
            .collect();
        if matched.is_empty() {
            return Err(InstallError::SkillNotFound {
                source_input: ctx.source_input.clone(),
                skill: filter,
            });
        }
        matched
    } else {
        skill_dirs
    };

    let count = filtered.len();
    eprintln!(
        "Found {count} skill{} in {}",
        if count == 1 { "" } else { "s" },
        mask_credentials(&ctx.source_input)
    );

    let mut results = Vec::new();
    let mut skip_all = false;

    for skill_dir in filtered {
        let name = skill_dir_name(&skill_dir);
        eprintln!("Installing {name}...");
        let format = detect_format(&skill_dir)?;
        let record = match format {
            SkillFormat::Skillprism => install_skillprism_skill(
                ctx,
                &skill_dir,
                source_url,
                source_type,
                r#ref,
                resolved_ref.clone(),
                skill_path,
                &mut skip_all,
            )?,
            SkillFormat::Plain => install_plain_skill(
                ctx,
                &skill_dir,
                source_url,
                source_type,
                r#ref,
                resolved_ref.clone(),
                skill_path,
                &mut skip_all,
            )?,
        };
        on_installed(&record)?;
        results.push(InstallResult { record });
    }

    Ok(results)
}

fn install_from_well_known(
    ctx: &InstallContext,
    _url: &str,
    _index_path: &str,
) -> Result<Vec<InstallResult>, InstallError> {
    // v1 WellKnown support is intentionally minimal: the parser recognizes the
    // form and surfaces a clear error. Full index-driven installs are deferred
    // until a registry backend is available.
    Err(InstallError::UnsupportedSource {
        source_input: ctx.source_input.clone(),
        detail: "well-known skill indexes are not supported yet".to_string(),
        help: "Install directly from a git repository or GitHub/GitLab shorthand instead."
            .to_string(),
    })
}

fn skill_filter_from_parsed(parsed: &ParsedSource) -> Option<String> {
    match parsed {
        ParsedSource::GitHub { skill_filter, .. } | ParsedSource::GitLab { skill_filter, .. } => {
            skill_filter.clone()
        }
        _ => None,
    }
}

pub fn skill_dir_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Discovers directories inside `root` that directly contain a skill template.
pub fn discover_skill_dirs(root: &Path) -> Result<Vec<PathBuf>, InstallError> {
    let mut dirs = Vec::new();
    walk_for_skills(root, &mut dirs)?;
    dirs.sort();
    Ok(dirs)
}

fn walk_for_skills(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), InstallError> {
    if !dir.exists() {
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        // Avoid following symlinks: a malicious symlink could escape the source
        // tree or create infinite recursion during discovery.
        if metadata.is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if path.join("SKILL.md").exists() || path.join("SKILL.md.j2").exists() {
                out.push(path);
            } else {
                walk_for_skills(&path, out)?;
            }
        }
    }
    Ok(())
}

/// Detects the format of a skill directory from its manifest.
pub fn detect_format(skill_dir: &Path) -> Result<SkillFormat, InstallError> {
    let manifest = skill_dir.join("skill.yaml");
    if !manifest.exists() {
        return Ok(SkillFormat::Plain);
    }

    let content = fs::read_to_string(&manifest)?;
    let raw: BTreeMap<String, yaml_serde::Value> =
        yaml_serde::from_str(&content).map_err(|e| InstallError::MalformedManifest {
            path: manifest.to_string_lossy().to_string(),
            detail: format!("invalid YAML: {e}"),
        })?;

    raw.get("skillprism").map_or_else(
        // A `skill.yaml` that is present but omits the `skillprism:` field is a
        // malformed manifest (EPIC-I DIST-I002 format rule 3): the author must
        // either declare the format or drop the file. Silently treating it as
        // plain would copy a skillprism skill verbatim, leaving unrendered
        // MiniJinja markers in place.
        || {
            Err(InstallError::MalformedManifest {
                path: manifest.to_string_lossy().to_string(),
                detail: "skill.yaml is present but missing the `skillprism:` field; either add \
                         `skillprism: '1'` to declare skillprism-format, or remove skill.yaml to \
                         declare plain-format."
                    .to_string(),
            })
        },
        |value| match value.as_str() {
            None => Err(InstallError::MalformedManifest {
                path: manifest.to_string_lossy().to_string(),
                detail: "the `skillprism:` field must be a quoted string".to_string(),
            }),
            Some("") => Err(InstallError::MalformedManifest {
                path: manifest.to_string_lossy().to_string(),
                detail: "the `skillprism:` field must not be empty".to_string(),
            }),
            Some("1") => Ok(SkillFormat::Skillprism),
            Some(other) => Err(InstallError::MalformedManifest {
                path: manifest.to_string_lossy().to_string(),
                detail: format!(
                    "unsupported `skillprism:` value `{other}`; only `skillprism: '1'` is supported"
                ),
            }),
        },
    )
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn install_skillprism_skill(
    ctx: &InstallContext,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
    resolved_ref: Option<String>,
    skill_path: Option<&String>,
    skip_all: &mut bool,
) -> Result<InstalledSkill, InstallError> {
    let (skill, _temp_project) = load_skill_into_temp_project(skill_dir, &ctx.harnesses)?;
    let registry = build_registry_for_harnesses(&ctx.harnesses);
    let mut files = Vec::new();

    for harness_id in &ctx.harnesses {
        let pair = HarnessResolver::resolve_skill_harness(&skill, harness_id, &registry)
            .map_err(resolve_to_install_error(&skill.name))?;
        let output =
            Engine::render(&pair).map_err(|e| InstallError::Render(miette::Report::new(e)))?;

        let target = install_scope_to_target(ctx.target_scope);
        let project_root = ctx
            .project_root
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        let result = Router::write(&pair, &output, project_root, target, ctx.force, skip_all)
            .map_err(|e| InstallError::Render(miette::Report::new(e)))?;

        files.push(InstalledFile {
            path: result.written.skill_path.to_string_lossy().to_string(),
            hash: format!("sha256:{}", sha256_file(&result.written.skill_path)?),
        });

        for sidecar in &result.written.sidecar_paths {
            files.push(InstalledFile {
                path: sidecar.to_string_lossy().to_string(),
                hash: format!("sha256:{}", sha256_file(sidecar)?),
            });
        }

        for asset in &result.written.asset_paths {
            files.push(InstalledFile {
                path: asset.to_string_lossy().to_string(),
                hash: format!("sha256:{}", sha256_file(asset)?),
            });
        }
    }

    Ok(build_record(
        &skill.name,
        ctx,
        source_url,
        source_type,
        r#ref,
        resolved_ref,
        skill_path,
        SkillFormat::Skillprism,
        files,
    ))
}

#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn install_plain_skill(
    ctx: &InstallContext,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
    resolved_ref: Option<String>,
    skill_path: Option<&String>,
    skip_all: &mut bool,
) -> Result<InstalledSkill, InstallError> {
    let skill_name = skill_dir_name(skill_dir);
    let template = crate::loader::find_template_path(skill_dir)
        .map_err(InstallError::Project)?
        .ok_or_else(|| InstallError::NoSkillsFound {
            source_input: ctx.source_input.clone(),
        })?;

    let registry = build_registry_for_harnesses(&ctx.harnesses);
    let mut files = Vec::new();

    for harness_id in &ctx.harnesses {
        let harness = registry
            .resolve(harness_id)
            .map_err(InstallError::Project)?;
        let target = install_scope_to_target(ctx.target_scope);
        let project_root = ctx
            .project_root
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        let skill_path_buf =
            crate::router::resolve_skill_path(project_root, &harness, &skill_name, target)
                .map_err(|e| {
                    InstallError::Project(ProjectError::ConfigRead {
                        path: skill_name.clone(),
                        source: std::io::Error::other(e.to_string()),
                    })
                })?;

        let skill_dir_out = skill_path_buf.parent().unwrap().to_path_buf();
        let asset_dirs =
            crate::loader::discover_asset_dirs(skill_dir).map_err(InstallError::Project)?;
        let mut skipped = Vec::new();

        if crate::router::resolve_overwrite(&skill_path_buf, ctx.force, skip_all, &mut skipped) {
            let template_bytes = fs::read(&template)?;
            crate::router::atomic_write_bytes(&skill_path_buf, &template_bytes)?;
            files.push(InstalledFile {
                path: skill_path_buf.to_string_lossy().to_string(),
                hash: format!("sha256:{}", sha256_bytes(&template_bytes)),
            });

            for asset_dir in &asset_dirs {
                let dir_name = asset_dir
                    .file_name()
                    .ok_or_else(|| ProjectError::ConfigRead {
                        path: skill_name.clone(),
                        source: std::io::Error::other("asset directory has no name"),
                    })?
                    .to_string_lossy()
                    .to_string();
                copy_dir(asset_dir, &skill_dir_out.join(&dir_name), asset_dir)?;
                record_asset_hashes(asset_dir, &skill_dir_out, &mut files)?;
            }
        } else if skill_path_buf.exists() {
            // The user declined to overwrite but the file already exists. Record
            // its hash so update has a baseline for the next run.
            files.push(InstalledFile {
                path: skill_path_buf.to_string_lossy().to_string(),
                hash: format!("sha256:{}", sha256_file(&skill_path_buf)?),
            });

            // Also record existing asset hashes so update has a stable baseline.
            for asset_dir in &asset_dirs {
                record_asset_hashes(asset_dir, &skill_dir_out, &mut files)?;
            }
        }
    }

    Ok(build_record(
        &skill_name,
        ctx,
        source_url,
        source_type,
        r#ref,
        resolved_ref,
        skill_path,
        SkillFormat::Plain,
        files,
    ))
}

/// RAII guard that removes a temporary render project directory on drop.
pub struct TempProject(PathBuf);

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn load_skill_into_temp_project(
    skill_dir: &Path,
    harnesses: &[String],
) -> Result<(SkillModel, TempProject), InstallError> {
    let temp_dir = std::env::temp_dir().join(format!(
        "skillprism-render-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir)?;

    let harness_list = harnesses
        .iter()
        .map(|h| format!("  - {h}"))
        .collect::<Vec<_>>()
        .join("\n");
    let config_yaml = format!("harnesses:\n{harness_list}\nskills_dir: skills\n");
    fs::write(temp_dir.join("skillprism.yaml"), config_yaml)?;

    let skill_name = skill_dir_name(skill_dir);
    let dest = temp_dir.join("skills").join(&skill_name);
    copy_dir(skill_dir, &dest, skill_dir)?;

    let model = ProjectLoader::load(&temp_dir)?;
    let skill = model
        .skills
        .into_iter()
        .next()
        .ok_or_else(|| InstallError::NoSkillsFound {
            source_input: skill_dir.to_string_lossy().to_string(),
        })?;

    Ok((skill, TempProject(temp_dir)))
}

pub fn build_registry_for_harnesses(_harnesses: &[String]) -> HarnessRegistry {
    // Harness ids are validated per-skill during rendering via
    // `HarnessResolver::resolve_skill_harness`, which surfaces a clear error for
    // unknown ids. This constructor only assembles the built-in registry; the
    // parameter is retained for call-site symmetry and future upfront
    // validation.
    HarnessRegistry::with_builtins()
}

pub const fn install_scope_to_target(scope: InstallScope) -> TargetScope {
    match scope {
        InstallScope::Project => TargetScope::Project,
        InstallScope::User => TargetScope::User,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_record(
    skill_name: &str,
    ctx: &InstallContext,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
    resolved_ref: Option<String>,
    skill_path: Option<&String>,
    format: SkillFormat,
    files: Vec<InstalledFile>,
) -> InstalledSkill {
    let now = now_rfc3339();
    let project_root = if ctx.target_scope == InstallScope::Project {
        ctx.project_root
            .as_deref()
            .and_then(|p| std::path::absolute(p).ok())
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };
    InstalledSkill {
        name: skill_name.to_string(),
        source: ctx.source_input.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: r#ref.cloned(),
        resolved_ref,
        skill_path: skill_path.cloned(),
        project_root,
        scope: ctx.target_scope,
        harnesses: ctx.harnesses.clone(),
        format,
        installed_at: now.clone(),
        updated_at: now,
        files,
    }
}

pub fn record_asset_hashes(
    src_dir: &Path,
    dst_base: &Path,
    files: &mut Vec<InstalledFile>,
) -> Result<(), InstallError> {
    let dir_name = src_dir.file_name().unwrap();
    let dst_dir = dst_base.join(dir_name);
    for entry in walk_files(&dst_dir)? {
        files.push(InstalledFile {
            path: entry.to_string_lossy().to_string(),
            hash: format!("sha256:{}", sha256_file(&entry)?),
        });
    }
    Ok(())
}

pub fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, InstallError> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn resolve_to_install_error(skill_name: &str) -> impl FnOnce(ResolveError) -> InstallError + '_ {
    move |e| match e {
        ResolveError::UnknownHarness {
            harness_name,
            available,
            ..
        } => InstallError::Project(ProjectError::UnknownHarness {
            name: harness_name,
            message: available,
        }),
        ResolveError::MissingCapability { .. } => InstallError::Project(ProjectError::ConfigRead {
            path: skill_name.to_string(),
            source: std::io::Error::other(e.to_string()),
        }),
    }
}

/// Copies a directory tree, materializing symlinks so that the destination is a
/// self-contained snapshot with no references back to the source tree.
///
/// `src_root` is the top-level directory being copied; any symlink whose target
/// resolves outside that root is rejected. Directory cycles via symlinks are
/// also rejected.
pub fn copy_dir(src: &Path, dst: &Path, src_root: &Path) -> Result<(), InstallError> {
    let mut visited = std::collections::HashSet::new();
    copy_dir_inner(src, dst, src_root, &mut visited)
}

fn copy_dir_inner(
    src: &Path,
    dst: &Path,
    src_root: &Path,
    visited: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), InstallError> {
    let canonical_src = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    if !visited.insert(canonical_src.clone()) {
        return Err(InstallError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "directory cycle detected while copying {}; symlink points to an ancestor",
                src.display()
            ),
        )));
    }

    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let metadata = fs::symlink_metadata(&src_path)?;
        if metadata.is_symlink() {
            copy_symlink_target(&src_path, &dst_path, src_root, visited)?;
        } else if metadata.is_dir() {
            copy_dir_inner(&src_path, &dst_path, src_root, visited)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    // Remove the directory from the visited set when leaving it so that distinct
    // subtrees that happen to share a canonical path are still allowed.
    visited.remove(&canonical_src);
    Ok(())
}

fn copy_symlink_target(
    link: &Path,
    dst: &Path,
    src_root: &Path,
    visited: &mut std::collections::HashSet<PathBuf>,
) -> Result<(), InstallError> {
    let target = fs::read_link(link)?;
    // Resolve relative symlinks against the link's parent so the target is read
    // from inside the source tree, not from the destination path.
    let resolved = if target.is_relative() {
        link.parent()
            .map_or_else(|| target.clone(), |parent| parent.join(&target))
    } else {
        target
    };

    let canonical_root = std::fs::canonicalize(src_root).unwrap_or_else(|_| src_root.to_path_buf());
    let canonical_target = std::fs::canonicalize(&resolved).map_err(|e| {
        InstallError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "symlink {} points to missing or unreadable target {}: {e}",
                link.display(),
                resolved.display()
            ),
        ))
    })?;
    if canonical_target == canonical_root || !canonical_target.starts_with(&canonical_root) {
        return Err(InstallError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "symlink {} escapes the source directory (points to {})",
                link.display(),
                canonical_target.display()
            ),
        )));
    }

    if visited.contains(&canonical_target) {
        return Err(InstallError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "symlink {} creates a directory cycle (points to {})",
                link.display(),
                canonical_target.display()
            ),
        )));
    }

    let metadata = fs::symlink_metadata(&canonical_target)?;
    if metadata.is_dir() {
        copy_dir_inner(&canonical_target, dst, src_root, visited)?;
    } else {
        fs::copy(&canonical_target, dst)?;
    }
    Ok(())
}

/// Computes the SHA-256 hash of a file's contents, returning a lowercase hex string.
pub fn sha256_file(path: &Path) -> Result<String, InstallError> {
    let content = fs::read(path)?;
    Ok(sha256_bytes(&content))
}

/// Computes the SHA-256 hash of bytes, returning a lowercase hex string.
pub fn sha256_bytes(content: &[u8]) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(content);
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn copy_dir_materializes_in_tree_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("real.txt"), b"hello").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(src.join("real.txt"), src.join("link.txt")).unwrap();
        #[cfg(not(unix))]
        fs::copy(src.join("real.txt"), src.join("link.txt")).unwrap();

        copy_dir(&src, &dst, &src).unwrap();

        assert!(dst.join("real.txt").exists());
        assert!(dst.join("link.txt").exists());
        assert!(
            !fs::symlink_metadata(dst.join("link.txt"))
                .unwrap()
                .is_symlink()
        );
    }

    #[test]
    fn detect_format_no_manifest_is_plain() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("SKILL.md"), b"# plain").unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), SkillFormat::Plain);
    }

    #[test]
    fn detect_format_declared_skillprism() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("skill.yaml"), "skillprism: '1'\n").unwrap();
        assert_eq!(detect_format(tmp.path()).unwrap(), SkillFormat::Skillprism);
    }

    #[test]
    fn detect_format_manifest_missing_skillprism_field_errors() {
        // EPIC-I DIST-I002 format rule 3: skill.yaml present without the
        // `skillprism:` field is a malformed manifest, not a plain skill.
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("skill.yaml"), "name: my-skill\n").unwrap();
        let err = detect_format(tmp.path()).unwrap_err();
        match err {
            InstallError::MalformedManifest { detail, .. } => {
                assert!(
                    detail.contains("missing the `skillprism:` field"),
                    "unexpected detail: {detail}"
                );
            }
            other => panic!("expected MalformedManifest, got {other:?}"),
        }
    }

    #[test]
    fn copy_dir_rejects_escaping_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        let secret = tmp.path().join("secret.txt");
        fs::create_dir(&src).unwrap();
        let mut f = fs::File::create(&secret).unwrap();
        f.write_all(b"secret").unwrap();
        drop(f);

        #[cfg(unix)]
        {
            let target = PathBuf::from("..").join("..").join("secret.txt");
            std::os::unix::fs::symlink(&target, src.join("escape.txt")).unwrap();
        }
        #[cfg(not(unix))]
        {
            // Windows symlinks may require privileges; skip this assertion.
            return;
        }

        let result = copy_dir(&src, &dst, &src);
        assert!(result.is_err(), "expected symlink escape to be rejected");
    }
}

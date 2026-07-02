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
use super::source::{ParsedSource, SourceParseError};

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

    /// The template path is ambiguous.
    #[error("{detail}")]
    AmbiguousTemplate { detail: String },
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
}

/// Result of installing a single skill.
#[derive(Debug, Clone)]
pub struct InstallResult {
    /// Installed skill metadata, ready for the state file.
    pub record: InstalledSkill,
}

/// Installs all skills from a source into the target scope.
#[allow(clippy::too_many_lines)]
pub fn install_source(ctx: &InstallContext) -> Result<Vec<InstallResult>, InstallError> {
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

    let skill_dirs = discover_skill_dirs(&source_path)?;
    if skill_dirs.is_empty() {
        return Err(InstallError::NoSkillsFound {
            source_input: ctx.source_input.clone(),
        });
    }

    let filtered = if let Some(filter) = skill_filter_from_parsed(&ctx.parsed) {
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

    let mut results = Vec::new();
    let mut skip_all = false;

    for skill_dir in filtered {
        let format = detect_format(&skill_dir)?;
        let record = match format {
            SkillFormat::Skillprism => install_skillprism_skill(
                ctx,
                &skill_dir,
                &source_url,
                source_type,
                r#ref.as_ref(),
                skill_path.as_ref(),
                &mut skip_all,
            )?,
            SkillFormat::Plain => install_plain_skill(
                ctx,
                &skill_dir,
                &source_url,
                source_type,
                r#ref.as_ref(),
                skill_path.as_ref(),
                &mut skip_all,
            )?,
        };
        results.push(InstallResult { record });
    }

    if let Some(dir) = cleanup_dir {
        let _ = network::cleanup_temp_dir(&dir);
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
    Err(InstallError::NoSkillsFound {
        source_input: ctx.source_input.clone(),
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

fn skill_dir_name(dir: &Path) -> String {
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
        if path.is_dir() {
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
        || {
            Err(InstallError::MalformedManifest {
                path: manifest.to_string_lossy().to_string(),
                detail: "missing the `skillprism:` field".to_string(),
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
            Some(_) => Ok(SkillFormat::Skillprism),
        },
    )
}

fn install_skillprism_skill(
    ctx: &InstallContext,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
    skill_path: Option<&String>,
    skip_all: &mut bool,
) -> Result<InstalledSkill, InstallError> {
    let (skill, temp_project) = load_skill_into_temp_project(skill_dir, &ctx.harnesses)?;
    let registry = build_registry_for_harnesses(&ctx.harnesses);
    let mut files = Vec::new();

    for harness_id in &ctx.harnesses {
        let pair = HarnessResolver::resolve_skill_harness(&skill, harness_id, &registry)
            .map_err(resolve_to_install_error(&skill.name))?;
        let output = Engine::render(&pair).map_err(|e| {
            InstallError::Project(ProjectError::ConfigRead {
                path: skill.name.clone(),
                source: std::io::Error::other(e.to_string()),
            })
        })?;

        let target = install_scope_to_target(ctx.target_scope);
        let project_root = ctx
            .project_root
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        let result = Router::write(&pair, &output, project_root, target, ctx.force, skip_all)
            .map_err(|e| {
                InstallError::Project(ProjectError::ConfigRead {
                    path: skill.name.clone(),
                    source: std::io::Error::other(e.to_string()),
                })
            })?;

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
    }

    if let Some(dir) = temp_project {
        let _ = fs::remove_dir_all(dir);
    }

    Ok(build_record(
        &skill.name,
        ctx,
        source_url,
        source_type,
        r#ref,
        skill_path,
        SkillFormat::Skillprism,
        files,
    ))
}

fn install_plain_skill(
    ctx: &InstallContext,
    skill_dir: &Path,
    source_url: &str,
    source_type: SourceType,
    r#ref: Option<&String>,
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
        let mut skipped = Vec::new();

        if crate::router::resolve_overwrite(&skill_path_buf, ctx.force, skip_all, &mut skipped) {
            fs::copy(&template, &skill_path_buf)?;
            files.push(InstalledFile {
                path: skill_path_buf.to_string_lossy().to_string(),
                hash: format!("sha256:{}", sha256_file(&skill_path_buf)?),
            });
        }

        let asset_dirs =
            crate::loader::discover_asset_dirs(skill_dir).map_err(InstallError::Project)?;
        if !asset_dirs.is_empty() {
            crate::router::copy_assets(&asset_dirs, &skill_dir_out)?;
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
        skill_path,
        SkillFormat::Plain,
        files,
    ))
}

fn load_skill_into_temp_project(
    skill_dir: &Path,
    harnesses: &[String],
) -> Result<(SkillModel, Option<PathBuf>), InstallError> {
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
    copy_dir(skill_dir, &dest)?;

    let model = ProjectLoader::load(&temp_dir)?;
    let skill = model
        .skills
        .into_iter()
        .next()
        .ok_or_else(|| InstallError::NoSkillsFound {
            source_input: skill_dir.to_string_lossy().to_string(),
        })?;

    Ok((skill, Some(temp_dir)))
}

fn build_registry_for_harnesses(harnesses: &[String]) -> HarnessRegistry {
    let registry = HarnessRegistry::with_builtins();
    for id in harnesses {
        let _ = registry.resolve(id); // ensure it exists
    }
    registry
}

const fn install_scope_to_target(scope: InstallScope) -> TargetScope {
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
    skill_path: Option<&String>,
    format: SkillFormat,
    files: Vec<InstalledFile>,
) -> InstalledSkill {
    let now = now_rfc3339();
    InstalledSkill {
        name: skill_name.to_string(),
        source: ctx.source_input.clone(),
        source_url: source_url.to_string(),
        source_type,
        r#ref: r#ref.cloned(),
        skill_path: skill_path.cloned(),
        scope: ctx.target_scope,
        harnesses: ctx.harnesses.clone(),
        format,
        installed_at: now.clone(),
        updated_at: now,
        files,
    }
}

fn record_asset_hashes(
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

fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, InstallError> {
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

fn copy_dir(src: &Path, dst: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let metadata = fs::symlink_metadata(&src_path)?;
        if metadata.is_symlink() {
            let target = fs::read_link(&src_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dst_path)?;
            #[cfg(not(unix))]
            fs::copy(&src_path, &dst_path)?;
        } else if metadata.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
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

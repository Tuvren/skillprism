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

pub mod diff;
mod manifest;
mod overwrite;
pub mod paths;
mod write;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use miette::Diagnostic;
use thiserror::Error;

use crate::cli::TargetScope;
use crate::engine::HarnessOutput;
use crate::resolver::ResolvedPair;

pub use overwrite::resolve_overwrite;
pub use paths::*;
pub use write::*;

/// Errors that occur during file writing or diffing in the router.
#[derive(Debug, Diagnostic, Error)]
pub enum RouterError {
    /// Failed to write a rendered file to disk.
    #[error("[{skill}] {harness}: Failed to write `{path}`")]
    #[diagnostic(help("{detail}"))]
    WriteError {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },

    /// Failed to copy asset directories alongside the skill.
    #[error("[{skill}] {harness}: Failed to copy assets to `{path}`")]
    #[diagnostic(help("{detail}"))]
    AssetCopyError {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },

    /// A resolved output path escapes its allowed scope (path traversal).
    #[error(
        "[{skill}] {harness}: Path traversal detected — `{resolved}` escapes scope `{allowed_base}`"
    )]
    #[diagnostic(help(
        "Harness installation paths must not contain `..` traversal that escapes the intended scope"
    ))]
    PathTraversal {
        skill: String,
        harness: String,
        resolved: String,
        allowed_base: String,
    },

    /// Two or more skill-harness pairs resolve to the same output path.
    #[error("Path collision at `{path}`")]
    #[diagnostic(help("Skills: {colliding_skills}"))]
    PathCollision {
        path: String,
        colliding_skills: String,
    },

    /// `$HOME` is not set; cannot resolve user-scope paths.
    #[error("$HOME is not set; cannot resolve user-scope path")]
    #[diagnostic(help(
        "Set the HOME environment variable or use a different target scope (project, dist)"
    ))]
    MissingHome,

    /// An absolute path was provided where a relative path was required.
    #[error("[{skill}] {harness}: Absolute path not allowed — `{component}`")]
    #[diagnostic(help(
        "Sidecar output_dir and filename must use relative paths within the skill output directory"
    ))]
    AbsolutePathDisallowed {
        skill: String,
        harness: String,
        component: String,
    },
}

/// A single manifest entry produced by rendering a skill through a harness.
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    /// The resolved path where the manifest file should be written.
    pub path: PathBuf,
    /// The rendered content of this entry (e.g., a JSON object).
    pub content: String,
}

/// Routes rendered skill output to the correct file system locations.
pub struct Router;

/// Result of writing a skill's output files.
pub struct WriteResult {
    /// Files that were successfully written.
    pub written: WrittenFiles,
    /// Paths of files that were skipped (user-scope exists without --force).
    pub skipped: Vec<String>,
}

impl Router {
    /// Detects path collisions among resolved pairs before rendering or writing.
    ///
    /// Checks both skill file paths and sidecar file paths for collisions,
    /// including skill-vs-sidecar collisions across different pairs.
    ///
    /// Returns `Ok(())` if no collisions exist, or `Err(Vec<RouterError>)` with one
    /// error per colliding path. Each error lists all colliding skills.
    pub fn detect_collisions(
        pairs: &[ResolvedPair],
        project_root: &Path,
        target: TargetScope,
    ) -> Result<(), Vec<RouterError>> {
        let mut path_map: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

        for pair in pairs {
            let skill_path =
                match resolve_skill_path(project_root, &pair.harness, &pair.skill.name, target) {
                    Ok(path) => path,
                    Err(e) => return Err(vec![e]),
                };
            let label = format!("{} \u{2192} {}", pair.skill.name, &pair.harness.id);
            path_map.entry(skill_path.clone()).or_default().push(label);

            let skill_dir = match skill_path.parent() {
                Some(p) => p.to_path_buf(),
                None => continue,
            };

            for sidecar_def in &pair.harness.sidecars {
                let sidecar_path = match resolve_sidecar_path(
                    &skill_dir,
                    sidecar_def.output_dir.as_deref(),
                    &sidecar_def.filename,
                    &pair.skill.name,
                    &pair.harness.id,
                ) {
                    Ok(p) => p,
                    Err(e) => return Err(vec![e]),
                };
                let sidecar_label = format!(
                    "{} \u{2192} {} (sidecar: {})",
                    pair.skill.name, &pair.harness.id, sidecar_def.filename,
                );
                path_map
                    .entry(sidecar_path)
                    .or_default()
                    .push(sidecar_label);
            }
        }

        let errors: Vec<RouterError> = path_map
            .into_iter()
            .filter(|(_, labels)| labels.len() > 1)
            .map(|(path, labels)| RouterError::PathCollision {
                path: path.to_string_lossy().to_string(),
                colliding_skills: labels.join(", "),
            })
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Writes rendered skill output (skill file + sidecars) to disk at the resolved path.
    ///
    /// Manifest files are NOT written here — they are batch-processed via
    /// [`write_aggregated_manifests`](Self::write_aggregated_manifests) after all skills are rendered.
    pub fn write(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        project_root: &Path,
        target: TargetScope,
        force: bool,
        skip_all: &mut bool,
    ) -> Result<WriteResult, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;

        let skill_path = resolve_skill_path(project_root, &pair.harness, skill_name, target)?;
        let skill_dir = skill_path
            .parent()
            .expect("skill path has parent")
            .to_path_buf();

        let mut skipped = Vec::new();

        if overwrite::resolve_overwrite(&skill_path, force, skip_all, &mut skipped) {
            atomic_write(&skill_path, &output.skill_content).map_err(|e| {
                RouterError::WriteError {
                    skill: skill_name.clone(),
                    harness: harness_id.clone(),
                    path: skill_path.to_string_lossy().to_string(),
                    detail: e.to_string(),
                }
            })?;
        }

        let skill_was_skipped = skipped.contains(&skill_path.to_string_lossy().to_string());

        let sidecar_paths = if skill_was_skipped {
            Vec::new()
        } else {
            Self::write_sidecars(pair, output, &skill_dir, force, skip_all, &mut skipped)?
        };

        for dir in &pair.skill.asset_dirs {
            if !dir.exists() {
                eprintln!(
                    "Warning: [{}] {}: asset directory `{}` does not exist",
                    skill_name,
                    harness_id,
                    dir.display(),
                );
            }
        }
        let asset_paths = if !pair.skill.asset_dirs.is_empty() && !skill_was_skipped {
            write::copy_assets(&pair.skill.asset_dirs, &skill_dir).map_err(|e| {
                RouterError::AssetCopyError {
                    skill: skill_name.clone(),
                    harness: harness_id.clone(),
                    path: skill_dir.to_string_lossy().to_string(),
                    detail: e.to_string(),
                }
            })?
        } else {
            Vec::new()
        };

        Ok(WriteResult {
            written: WrittenFiles {
                skill_path,
                sidecar_paths,
                asset_paths,
            },
            skipped,
        })
    }

    /// Writes aggregated manifest files from collected per-skill entries.
    ///
    /// For JSON-format manifests, entries are aggregated into a JSON array.
    /// Manifests are grouped by unique (resolved path) — each group produces one file.
    pub fn write_aggregated_manifests(
        entries: &[ManifestEntry],
        force: bool,
        skip_all: &mut bool,
        skipped: &mut Vec<String>,
    ) -> Result<Vec<PathBuf>, RouterError> {
        let mut written = Vec::new();

        let grouped = manifest::group_manifest_entries(entries);

        for (path, group) in &grouped {
            if !overwrite::resolve_overwrite(path, force, skip_all, skipped) {
                continue;
            }
            let aggregated = manifest::aggregate_json_entries(group);
            atomic_write(path, &aggregated).map_err(|e| RouterError::WriteError {
                skill: "manifest".to_string(),
                harness: "aggregated".to_string(),
                path: path.to_string_lossy().to_string(),
                detail: e.to_string(),
            })?;
            written.push(path.clone());
        }

        Ok(written)
    }

    /// Computes diff entries for all files that would be written (skill + sidecars, no manifest).
    ///
    /// Returns `RouterError` if any path cannot be resolved (e.g., path traversal,
    /// missing `$HOME`, or absolute sidecar path).
    ///
    /// Manifest diffs are computed separately via [`diff_manifests`](Self::diff_manifests).
    pub fn diff(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        project_root: &Path,
        target: TargetScope,
    ) -> Result<Vec<DiffEntry>, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;

        let skill_path = resolve_skill_path(project_root, &pair.harness, skill_name, target)?;
        let skill_dir = skill_path
            .parent()
            .expect("skill path has parent")
            .to_path_buf();

        let mut entries = Vec::new();

        let existing = diff::read_existing(&skill_path);
        let diff_output = diff::compute_diff(
            existing.as_deref(),
            &output.skill_content,
            &skill_path.to_string_lossy(),
        );
        entries.push(DiffEntry {
            path: skill_path,
            diff: diff_output,
        });

        for sidecar in &output.sidecars {
            let sidecar_path = paths::resolve_sidecar_path(
                &skill_dir,
                sidecar.output_dir.as_deref(),
                &sidecar.filename,
                skill_name,
                harness_id,
            )?;
            let existing = diff::read_existing(&sidecar_path);
            let diff_output = diff::compute_diff(
                existing.as_deref(),
                &sidecar.content,
                &sidecar_path.to_string_lossy(),
            );
            entries.push(DiffEntry {
                path: sidecar_path,
                diff: diff_output,
            });
        }

        Ok(entries)
    }

    /// Computes diff entries for aggregated manifest files.
    pub fn diff_manifests(entries: &[ManifestEntry]) -> Vec<DiffEntry> {
        let grouped = manifest::group_manifest_entries(entries);
        let mut result = Vec::new();

        for (path, group) in &grouped {
            let aggregated = manifest::aggregate_json_entries(group);
            let existing = diff::read_existing(path);
            let diff_output =
                diff::compute_diff(existing.as_deref(), &aggregated, &path.to_string_lossy());
            result.push(DiffEntry {
                path: path.clone(),
                diff: diff_output,
            });
        }

        result
    }

    fn write_sidecars(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        skill_dir: &Path,
        force: bool,
        skip_all: &mut bool,
        skipped: &mut Vec<String>,
    ) -> Result<Vec<PathBuf>, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;
        let mut sidecar_paths = Vec::new();

        for sidecar in &output.sidecars {
            let sidecar_path = paths::resolve_sidecar_path(
                skill_dir,
                sidecar.output_dir.as_deref(),
                &sidecar.filename,
                skill_name,
                harness_id,
            )?;

            if !overwrite::resolve_overwrite(&sidecar_path, force, skip_all, skipped) {
                continue;
            }

            atomic_write(&sidecar_path, &sidecar.content).map_err(|e| RouterError::WriteError {
                skill: skill_name.clone(),
                harness: harness_id.clone(),
                path: sidecar_path.to_string_lossy().to_string(),
                detail: e.to_string(),
            })?;
            sidecar_paths.push(sidecar_path);
        }

        Ok(sidecar_paths)
    }
}

/// Paths of files written during a build operation.
pub struct WrittenFiles {
    /// Path to the rendered skill file.
    pub skill_path: std::path::PathBuf,
    /// Paths to any sidecar files written.
    pub sidecar_paths: Vec<std::path::PathBuf>,
    /// Paths to any asset files copied alongside the skill.
    pub asset_paths: Vec<std::path::PathBuf>,
}

/// A single file's diff output for the --diff mode.
pub struct DiffEntry {
    /// Path the diff applies to.
    pub path: std::path::PathBuf,
    /// Computed diff output.
    pub diff: diff::DiffOutput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SidecarOutput;
    use crate::registry::HarnessRegistry;
    use crate::resolver::HarnessResolver;
    use crate::resolver::ResolvedPair;
    use crate::resolver::tests::test_skill;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    fn written_content(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn writes_skill_to_project_scope() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "my-agent-rendered".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();
        assert_eq!(
            result.written.skill_path,
            dir.join(".claude/skills/my-agent/SKILL.md")
        );
        assert!(result.written.skill_path.exists());
        assert_eq!(
            written_content(&result.written.skill_path),
            "my-agent-rendered"
        );
    }

    #[test]
    fn creates_directory_if_not_exists() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("new-skill", vec![]);
        skill.template_path = Path::new("/nonexistent/template.j2").to_path_buf();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "content".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();
        assert!(
            result.written.skill_path.exists(),
            "directory should be created"
        );
        assert_eq!(written_content(&result.written.skill_path), "content");
    }

    #[test]
    fn writes_sidecar_files() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/agent")).unwrap();
        fs::write(dir.join("skills/agent/SKILL.md.j2"), "main").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("agent", vec![]);
        skill.template_path = dir.join("skills/agent/SKILL.md.j2");

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "main".to_string(),
            sidecars: vec![SidecarOutput {
                filename: "config.yaml".to_string(),
                content: "key: value".to_string(),
                output_dir: None,
            }],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();
        assert!(result.written.sidecar_paths[0].exists());
        assert_eq!(
            written_content(&result.written.sidecar_paths[0]),
            "key: value"
        );
    }

    #[test]
    fn aggregates_manifest_entries_into_json_array() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let manifest_path = dir.join("plugin.json");
        let entries = vec![
            ManifestEntry {
                path: manifest_path.clone(),
                content: r#"{"name":"skill-a"}"#.to_string(),
            },
            ManifestEntry {
                path: manifest_path.clone(),
                content: r#"{"name":"skill-b"}"#.to_string(),
            },
        ];

        let written =
            Router::write_aggregated_manifests(&entries, false, &mut false, &mut Vec::new())
                .unwrap();
        assert_eq!(written.len(), 1);
        assert!(manifest_path.exists());

        let content = fs::read_to_string(&manifest_path).unwrap();
        assert!(content.contains("skill-a"));
        assert!(content.contains("skill-b"));
        assert!(content.starts_with('['));
        assert!(content.ends_with(']'));
    }

    #[test]
    fn copies_asset_directories() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let skill_dir = dir.join("skills/test-skill");
        fs::create_dir_all(skill_dir.join("references")).unwrap();
        fs::write(skill_dir.join("references/guide.md"), "# Guide").unwrap();
        fs::write(skill_dir.join("SKILL.md.j2"), "content").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("test-skill", vec![]);
        skill.template_path = skill_dir.join("SKILL.md.j2");
        skill.asset_dirs = vec![skill_dir.join("references")];

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "content".to_string(),
            sidecars: vec![],
        };

        Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();

        let dest_refs = dir.join(".claude/skills/test-skill/references/guide.md");
        assert!(dest_refs.exists());
        assert_eq!(written_content(&dest_refs), "# Guide");
    }

    #[test]
    fn diff_new_file_returns_single_entry() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "rendered content".to_string(),
            sidecars: vec![],
        };

        let entries = Router::diff(&pair, &output, dir, TargetScope::Project).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].diff.stats.is_new_file);
        assert_eq!(entries[0].diff.stats.additions, 1);
    }

    #[test]
    fn diff_changed_file_shows_diff() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join(".claude/skills/my-agent")).unwrap();
        fs::write(dir.join(".claude/skills/my-agent/SKILL.md"), "old content").unwrap();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "new content".to_string(),
            sidecars: vec![],
        };

        let entries = Router::diff(&pair, &output, dir, TargetScope::Project).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].diff.stats.is_new_file);
        assert_eq!(entries[0].diff.stats.additions, 1);
        assert_eq!(entries[0].diff.stats.deletions, 1);
    }

    #[test]
    fn diff_unchanged_file_returns_empty_hunks() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join(".claude/skills/my-agent")).unwrap();
        fs::write(dir.join(".claude/skills/my-agent/SKILL.md"), "same content").unwrap();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "same content".to_string(),
            sidecars: vec![],
        };

        let entries = Router::diff(&pair, &output, dir, TargetScope::Project).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].diff.hunks.is_empty());
        assert_eq!(entries[0].diff.stats.additions, 0);
    }

    #[test]
    fn diff_includes_sidecar_entries() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/agent")).unwrap();
        fs::write(dir.join("skills/agent/SKILL.md.j2"), "main").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("agent", vec![]);
        skill.template_path = dir.join("skills/agent/SKILL.md.j2");

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "main".to_string(),
            sidecars: vec![SidecarOutput {
                filename: "config.yaml".to_string(),
                content: "key: value".to_string(),
                output_dir: None,
            }],
        };

        let entries = Router::diff(&pair, &output, dir, TargetScope::Project).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].diff.stats.is_new_file);
        assert!(entries[1].diff.stats.is_new_file);
    }

    #[test]
    fn write_result_includes_skipped_vec() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "rendered".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();
        assert!(result.skipped.is_empty());
        assert!(result.written.skill_path.exists());
    }

    #[test]
    fn force_true_does_not_skip() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "rendered".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, true, &mut false).unwrap();
        assert!(result.skipped.is_empty());
        assert!(result.written.skill_path.exists());
        assert_eq!(written_content(&result.written.skill_path), "rendered");
    }

    #[test]
    fn write_aggregated_manifests_empty_entries() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let _dir = tmp_dir.path();

        let written =
            Router::write_aggregated_manifests(&[], false, &mut false, &mut Vec::new()).unwrap();
        assert!(written.is_empty());
    }

    #[test]
    fn diff_manifests_empty_entries() {
        let diffs = Router::diff_manifests(&[]);
        assert!(diffs.is_empty());
    }

    #[test]
    fn diff_manifests_shows_aggregated_diff() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let manifest_path = dir.join("plugin.json");
        fs::write(&manifest_path, r#"[{"name":"old-skill"}]"#).unwrap();

        let entries = vec![ManifestEntry {
            path: manifest_path,
            content: r#"{"name":"new-skill"}"#.to_string(),
        }];

        let diffs = Router::diff_manifests(&entries);
        assert_eq!(diffs.len(), 1);
        assert!(!diffs[0].diff.stats.is_new_file);
        assert!(diffs[0].diff.stats.additions > 0 || diffs[0].diff.stats.deletions > 0);
    }

    #[test]
    fn existing_file_skipped_in_non_interactive_mode() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join(".claude/skills/my-agent")).unwrap();
        fs::write(dir.join(".claude/skills/my-agent/SKILL.md"), "old").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "new".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, false, &mut false).unwrap();
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(written_content(&result.written.skill_path), "old");
    }

    #[test]
    fn skip_all_prevents_prompt_for_existing_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join(".claude/skills/my-agent")).unwrap();
        fs::write(dir.join(".claude/skills/my-agent/SKILL.md"), "old").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "new".to_string(),
            sidecars: vec![],
        };

        let mut skip_all = true;
        let result = Router::write(
            &pair,
            &output,
            dir,
            TargetScope::Project,
            false,
            &mut skip_all,
        )
        .unwrap();
        assert!(skip_all);
        assert_eq!(result.skipped.len(), 1);
        assert_eq!(written_content(&result.written.skill_path), "old");
    }

    #[test]
    fn force_overwrites_existing_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join(".claude/skills/my-agent")).unwrap();
        fs::write(dir.join(".claude/skills/my-agent/SKILL.md"), "old").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");
        skill.variables = BTreeMap::new();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "overwritten".to_string(),
            sidecars: vec![],
        };

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, true, &mut false).unwrap();
        assert!(result.skipped.is_empty());
        assert_eq!(written_content(&result.written.skill_path), "overwritten");
    }

    #[test]
    fn detect_collisions_detects_duplicate_skill_name() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/a")).unwrap();
        fs::write(dir.join("skills/a/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join("skills/b")).unwrap();
        fs::write(dir.join("skills/b/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill_a = test_skill("same-name", vec![]);
        skill_a.template_path = dir.join("skills/a/SKILL.md.j2");
        skill_a.variables = BTreeMap::new();
        let mut skill_b = test_skill("same-name", vec![]);
        skill_b.template_path = dir.join("skills/b/SKILL.md.j2");
        skill_b.variables = BTreeMap::new();

        let pair_a = HarnessResolver::resolve_skill_harness(&skill_a, "claude", &registry).unwrap();
        let pair_b = HarnessResolver::resolve_skill_harness(&skill_b, "claude", &registry).unwrap();
        let pairs = vec![pair_a, pair_b];

        let result = Router::detect_collisions(&pairs, dir, TargetScope::Project);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            RouterError::PathCollision {
                path,
                colliding_skills,
            } => {
                assert!(
                    path.contains("same-name"),
                    "path should mention the colliding skill name, got {path}"
                );
                assert!(
                    colliding_skills.contains("same-name"),
                    "colliding_skills should mention same-name, got {colliding_skills}"
                );
            }
            e => panic!("expected PathCollision, got {e:?}"),
        }
    }

    #[test]
    fn detect_collisions_no_collision_for_unique_paths() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/a")).unwrap();
        fs::write(dir.join("skills/a/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join("skills/b")).unwrap();
        fs::write(dir.join("skills/b/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill_a = test_skill("unique-a", vec![]);
        skill_a.template_path = dir.join("skills/a/SKILL.md.j2");
        skill_a.variables = BTreeMap::new();
        let mut skill_b = test_skill("unique-b", vec![]);
        skill_b.template_path = dir.join("skills/b/SKILL.md.j2");
        skill_b.variables = BTreeMap::new();

        let pair_a = HarnessResolver::resolve_skill_harness(&skill_a, "claude", &registry).unwrap();
        let pair_b = HarnessResolver::resolve_skill_harness(&skill_b, "claude", &registry).unwrap();
        let pairs = vec![pair_a, pair_b];

        let result = Router::detect_collisions(&pairs, dir, TargetScope::Project);
        assert!(
            result.is_ok(),
            "expected no collisions for unique skill names, got {result:?}"
        );
    }

    #[test]
    fn sidecars_respect_skip_all_when_skill_skipped() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let skill_dir = dir.join("skills/test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md.j2"), "content").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("test-skill", vec![]);
        skill.template_path = skill_dir.join("SKILL.md.j2");

        let pair = HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "rendered".to_string(),
            sidecars: vec![SidecarOutput {
                filename: "sidecar.yaml".to_string(),
                content: "key: val".to_string(),
                output_dir: None,
            }],
        };

        // Pre-create skill file so it would be skipped with skip_all=true
        let skill_output = dir.join(".opencode/skills/test-skill/SKILL.md");
        fs::create_dir_all(skill_output.parent().unwrap()).unwrap();
        fs::write(&skill_output, "old").unwrap();

        // Pre-create sidecar file too
        let sidecar_output = dir.join(".opencode/skills/test-skill/sidecar.yaml");
        fs::write(&sidecar_output, "old").unwrap();

        let mut skip_all = true;
        let result = Router::write(
            &pair,
            &output,
            dir,
            TargetScope::Project,
            false,
            &mut skip_all,
        )
        .unwrap();
        assert!(
            result
                .skipped
                .contains(&skill_output.to_string_lossy().to_string()),
            "skill should be in skipped list"
        );
        assert_eq!(
            written_content(&skill_output),
            "old",
            "skill file should remain unchanged"
        );
        assert_eq!(
            written_content(&sidecar_output),
            "old",
            "sidecar file should remain unchanged when skill is skipped"
        );
    }

    #[test]
    fn sidecars_respect_force_true() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();

        let skill_dir = dir.join("skills/test-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md.j2"), "content").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("test-skill", vec![]);
        skill.template_path = skill_dir.join("SKILL.md.j2");

        let pair = HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "new-content".to_string(),
            sidecars: vec![SidecarOutput {
                filename: "sidecar.yaml".to_string(),
                content: "new-val".to_string(),
                output_dir: None,
            }],
        };

        // Pre-create both files so they'd normally be skipped
        let skill_output = dir.join(".opencode/skills/test-skill/SKILL.md");
        fs::create_dir_all(skill_output.parent().unwrap()).unwrap();
        fs::write(&skill_output, "old").unwrap();
        let sidecar_output = dir.join(".opencode/skills/test-skill/sidecar.yaml");
        fs::write(&sidecar_output, "old").unwrap();

        let result =
            Router::write(&pair, &output, dir, TargetScope::Project, true, &mut false).unwrap();
        assert!(
            result.skipped.is_empty(),
            "no files should be skipped with --force"
        );
        assert_eq!(
            written_content(&skill_output),
            "new-content",
            "skill should be overwritten"
        );
        assert_eq!(
            written_content(&sidecar_output),
            "new-val",
            "sidecar should be overwritten"
        );
    }

    #[test]
    fn detect_sidecar_vs_sidecar_collision() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/a")).unwrap();
        fs::write(dir.join("skills/a/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join("skills/b")).unwrap();
        fs::write(dir.join("skills/b/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();

        // Build a patched harness with a sidecar to exercise the sidecar collision
        // code path in detect_collisions.
        let mut harness = registry.resolve("claude").unwrap();
        harness.sidecars = vec![crate::registry::SidecarDef {
            filename: "config.yaml".to_string(),
            template: "key: val".to_string(),
            output_dir: None,
        }];

        // Use the same skill name so both pairs share the same skill output directory,
        // causing their sidecar paths to collide.
        let skill_a = test_skill("same-skill", vec![]);
        let skill_b = test_skill("same-skill", vec![]);

        let pair_a = ResolvedPair {
            skill: skill_a,
            harness: harness.clone(),
        };
        let pair_b = ResolvedPair {
            skill: skill_b,
            harness,
        };
        let pairs = vec![pair_a, pair_b];

        let result = Router::detect_collisions(&pairs, dir, TargetScope::Project);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty(), "expected at least one collision error");
        for err in &errors {
            assert!(matches!(err, RouterError::PathCollision { .. }));
        }
    }

    #[test]
    fn detect_collisions_no_sidecar_collision_for_unique_sidecar_names() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let dir = tmp_dir.path();
        fs::create_dir_all(dir.join("skills/a")).unwrap();
        fs::write(dir.join("skills/a/SKILL.md.j2"), "{{ skill_name }}").unwrap();
        fs::create_dir_all(dir.join("skills/b")).unwrap();
        fs::write(dir.join("skills/b/SKILL.md.j2"), "{{ skill_name }}").unwrap();

        let registry = HarnessRegistry::with_builtins();

        // Different skill names → different skill directories → different sidecar paths → no collision.
        let mut harness = registry.resolve("claude").unwrap();
        harness.sidecars = vec![crate::registry::SidecarDef {
            filename: "config.yaml".to_string(),
            template: "key: val".to_string(),
            output_dir: None,
        }];

        let skill_a = test_skill("unique-a", vec![]);
        let skill_b = test_skill("unique-b", vec![]);

        let pair_a = ResolvedPair {
            skill: skill_a,
            harness: harness.clone(),
        };
        let pair_b = ResolvedPair {
            skill: skill_b,
            harness,
        };
        let pairs = vec![pair_a, pair_b];

        let result = Router::detect_collisions(&pairs, dir, TargetScope::Project);
        assert!(
            result.is_ok(),
            "expected no collisions for unique skill names, got {result:?}"
        );
    }
}

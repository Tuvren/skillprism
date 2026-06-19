pub mod diff;
mod paths;
mod write;

use std::path::Path;

use std::collections::BTreeMap;
use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

use crate::cli::TargetScope;
use crate::engine::HarnessOutput;
use crate::resolver::ResolvedPair;

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
    ) -> Result<WriteResult, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;

        let skill_path = resolve_skill_path(project_root, &pair.harness, skill_name, target);
        let skill_dir = skill_path
            .parent()
            .expect("skill path has parent")
            .to_path_buf();

        let mut skipped = Vec::new();

        if !force && target == TargetScope::User && skill_path.exists() {
            eprintln!(
                "Warning: skipping `{}` (user-scope file exists, use --force to overwrite)",
                skill_path.display()
            );
            skipped.push(skill_path.to_string_lossy().to_string());
        } else {
            atomic_write(&skill_path, &output.skill_content).map_err(|e| {
                RouterError::WriteError {
                    skill: skill_name.clone(),
                    harness: harness_id.clone(),
                    path: skill_path.to_string_lossy().to_string(),
                    detail: e.to_string(),
                }
            })?;
        }

        let sidecar_paths =
            Self::write_sidecars(pair, output, &skill_dir, target, force, &mut skipped)?;

        let skill_skipped = skipped.contains(&skill_path.to_string_lossy().to_string());
        if !pair.skill.asset_dirs.is_empty() && !skill_skipped {
            write::copy_assets(&pair.skill.asset_dirs, &skill_dir).map_err(|e| {
                RouterError::AssetCopyError {
                    skill: skill_name.clone(),
                    harness: harness_id.clone(),
                    path: skill_dir.to_string_lossy().to_string(),
                    detail: e.to_string(),
                }
            })?;
        }

        Ok(WriteResult {
            written: WrittenFiles {
                skill_path,
                sidecar_paths,
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
        target: TargetScope,
        force: bool,
    ) -> Result<Vec<PathBuf>, RouterError> {
        let mut written = Vec::new();

        let grouped = group_manifest_entries(entries);
        for (path, group) in &grouped {
            if !force && target == TargetScope::User && path.exists() {
                eprintln!(
                    "Warning: skipping manifest `{}` (user-scope file exists, use --force to overwrite)",
                    path.display()
                );
                continue;
            }

            let aggregated = aggregate_json_entries(group);
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
    /// Manifest diffs are computed separately via [`diff_manifests`](Self::diff_manifests).
    pub fn diff(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        project_root: &Path,
        target: TargetScope,
    ) -> Vec<DiffEntry> {
        let skill_name = &pair.skill.name;

        let skill_path = resolve_skill_path(project_root, &pair.harness, skill_name, target);
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
            );
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

        entries
    }

    /// Computes diff entries for aggregated manifest files.
    pub fn diff_manifests(
        entries: &[ManifestEntry],
    ) -> Vec<DiffEntry> {
        let grouped = group_manifest_entries(entries);
        let mut result = Vec::new();

        for (path, group) in &grouped {
            let aggregated = aggregate_json_entries(group);
            let existing = diff::read_existing(path);
            let diff_output = diff::compute_diff(
                existing.as_deref(),
                &aggregated,
                &path.to_string_lossy(),
            );
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
        target: TargetScope,
        force: bool,
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
            );

            if !force && target == TargetScope::User && sidecar_path.exists() {
                eprintln!(
                    "Warning: skipping `{}` (user-scope file exists, use --force to overwrite)",
                    sidecar_path.display()
                );
                skipped.push(sidecar_path.to_string_lossy().to_string());
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

/// Groups manifest entries by their resolved file path.
fn group_manifest_entries(entries: &[ManifestEntry]) -> BTreeMap<PathBuf, Vec<String>> {
    let mut grouped: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.path.clone())
            .or_default()
            .push(entry.content.clone());
    }
    grouped
}

/// Aggregates manifest entries into a JSON array.
///
/// Each entry is expected to be a JSON object string.
/// The result is a JSON array containing all entries.
fn aggregate_json_entries(entries: &[String]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push_str(",\n");
        }
        for line in entry.lines() {
            result.push_str("  ");
            result.push_str(line);
            result.push('\n');
        }
    }
    result.push(']');
    result
}

/// Paths of files written during a build operation.
pub struct WrittenFiles {
    /// Path to the rendered skill file.
    pub skill_path: std::path::PathBuf,
    /// Paths to any sidecar files written.
    pub sidecar_paths: Vec<std::path::PathBuf>,
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
    use crate::resolver::tests::test_skill;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    fn written_content(path: &Path) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn writes_skill_to_project_scope() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_project");
        let _ = fs::remove_dir_all(&dir);
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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();
        assert_eq!(
            result.written.skill_path,
            dir.join(".claude/skills/my-agent/SKILL.md")
        );
        assert!(result.written.skill_path.exists());
        assert_eq!(
            written_content(&result.written.skill_path),
            "my-agent-rendered"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_directory_if_not_exists() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_mkdir");
        let _ = fs::remove_dir_all(&dir);

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("new-skill", vec![]);
        skill.template_path = Path::new("/nonexistent/template.j2").to_path_buf();

        let pair = HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "content".to_string(),
            sidecars: vec![],
        };

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();
        assert!(
            result.written.skill_path.exists(),
            "directory should be created"
        );
        assert_eq!(written_content(&result.written.skill_path), "content");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_sidecar_files() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_sidecar");
        let _ = fs::remove_dir_all(&dir);
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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();
        assert!(result.written.sidecar_paths[0].exists());
        assert_eq!(
            written_content(&result.written.sidecar_paths[0]),
            "key: value"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn aggregates_manifest_entries_into_json_array() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_manifest_agg");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

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

        let written = Router::write_aggregated_manifests(&entries, TargetScope::Project, false)
            .unwrap();
        assert_eq!(written.len(), 1);
        assert!(manifest_path.exists());

        let content = fs::read_to_string(&manifest_path).unwrap();
        assert!(content.contains("skill-a"));
        assert!(content.contains("skill-b"));
        assert!(content.starts_with('['));
        assert!(content.ends_with(']'));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_asset_directories() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_assets");
        let _ = fs::remove_dir_all(&dir);

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

        Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();

        let dest_refs = dir.join(".claude/skills/test-skill/references/guide.md");
        assert!(dest_refs.exists());
        assert_eq!(written_content(&dest_refs), "# Guide");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_new_file_returns_single_entry() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_diff_new");
        let _ = fs::remove_dir_all(&dir);
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

        let entries = Router::diff(&pair, &output, &dir, TargetScope::Project);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].diff.stats.is_new_file);
        assert_eq!(entries[0].diff.stats.additions, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_changed_file_shows_diff() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_diff_changed");
        let _ = fs::remove_dir_all(&dir);
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

        let entries = Router::diff(&pair, &output, &dir, TargetScope::Project);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].diff.stats.is_new_file);
        assert_eq!(entries[0].diff.stats.additions, 1);
        assert_eq!(entries[0].diff.stats.deletions, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_unchanged_file_returns_empty_hunks() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_diff_unchanged");
        let _ = fs::remove_dir_all(&dir);
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

        let entries = Router::diff(&pair, &output, &dir, TargetScope::Project);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].diff.hunks.is_empty());
        assert_eq!(entries[0].diff.stats.additions, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_includes_sidecar_entries() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_diff_sidecar");
        let _ = fs::remove_dir_all(&dir);
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

        let entries = Router::diff(&pair, &output, &dir, TargetScope::Project);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].diff.stats.is_new_file);
        assert!(entries[1].diff.stats.is_new_file);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_result_includes_skipped_vec() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_write_result");
        let _ = fs::remove_dir_all(&dir);
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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();
        assert!(result.skipped.is_empty());
        assert!(result.written.skill_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn force_true_does_not_skip() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_force_true");
        let _ = fs::remove_dir_all(&dir);
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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, true).unwrap();
        assert!(result.skipped.is_empty());
        assert!(result.written.skill_path.exists());
        assert_eq!(written_content(&result.written.skill_path), "rendered");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_aggregated_manifests_empty_entries() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_manifest_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let written =
            Router::write_aggregated_manifests(&[], TargetScope::Project, false).unwrap();
        assert!(written.is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_manifests_empty_entries() {
        let diffs = Router::diff_manifests(&[]);
        assert!(diffs.is_empty());
    }

    #[test]
    fn diff_manifests_shows_aggregated_diff() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_diff_manifest");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let manifest_path = dir.join("plugin.json");
        fs::write(&manifest_path, r#"[{"name":"old-skill"}]"#).unwrap();

        let entries = vec![ManifestEntry {
            path: manifest_path.clone(),
            content: r#"{"name":"new-skill"}"#.to_string(),
        }];

        let diffs = Router::diff_manifests(&entries);
        assert_eq!(diffs.len(), 1);
        assert!(!diffs[0].diff.stats.is_new_file);
        assert!(diffs[0].diff.stats.additions > 0 || diffs[0].diff.stats.deletions > 0);

        let _ = fs::remove_dir_all(&dir);
    }
}

pub mod diff;
mod paths;
mod write;

use std::path::Path;

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
    /// Writes rendered skill output to disk at the resolved path.
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
            return Ok(WriteResult {
                written: WrittenFiles {
                    skill_path,
                    sidecar_paths: Vec::new(),
                    manifest_path: None,
                },
                skipped,
            });
        }

        atomic_write(&skill_path, &output.skill_content).map_err(|e| RouterError::WriteError {
            skill: skill_name.clone(),
            harness: harness_id.clone(),
            path: skill_path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;

        let sidecar_paths =
            Self::write_sidecars(pair, output, &skill_dir, target, force, &mut skipped)?;

        let manifest_path =
            Self::write_manifest(pair, output, project_root, target, force, &mut skipped)?;

        if !pair.skill.asset_dirs.is_empty() {
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
                manifest_path,
            },
            skipped,
        })
    }

    fn write_sidecars(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        skill_dir: &Path,
        target: TargetScope,
        force: bool,
        skipped: &mut Vec<String>,
    ) -> Result<Vec<std::path::PathBuf>, RouterError> {
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

    fn write_manifest(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        project_root: &Path,
        target: TargetScope,
        force: bool,
        skipped: &mut Vec<String>,
    ) -> Result<Option<std::path::PathBuf>, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;
        let Some(ref manifest_content) = output.manifest_entry else {
            return Ok(None);
        };
        let Some(path) = paths::resolve_manifest_path(project_root, &pair.harness, target) else {
            return Ok(None);
        };

        if !force && target == TargetScope::User && path.exists() {
            eprintln!(
                "Warning: skipping `{}` (user-scope file exists, use --force to overwrite)",
                path.display()
            );
            skipped.push(path.to_string_lossy().to_string());
            return Ok(None);
        }

        atomic_write(&path, manifest_content).map_err(|e| RouterError::WriteError {
            skill: skill_name.clone(),
            harness: harness_id.clone(),
            path: path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;

        Ok(Some(path))
    }

    /// Computes diffs for all files that would be written.
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

        if let Some(ref manifest_content) = output.manifest_entry {
            if let Some(path) = paths::resolve_manifest_path(project_root, &pair.harness, target) {
                let existing = diff::read_existing(&path);
                let diff_output = diff::compute_diff(
                    existing.as_deref(),
                    manifest_content,
                    &path.to_string_lossy(),
                );
                entries.push(DiffEntry {
                    path,
                    diff: diff_output,
                });
            }
        }

        entries
    }
}

/// Paths of files written during a build operation.
#[allow(dead_code)]
pub struct WrittenFiles {
    /// Path to the rendered skill file.
    pub skill_path: std::path::PathBuf,
    /// Paths to any sidecar files written.
    pub sidecar_paths: Vec<std::path::PathBuf>,
    /// Path to the manifest file, if one was produced.
    pub manifest_path: Option<std::path::PathBuf>,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
    fn writes_manifest_entry() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("router_manifest");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("skills/my-agent")).unwrap();
        fs::write(dir.join("skills/my-agent/SKILL.md.j2"), "content").unwrap();

        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("my-agent", vec![]);
        skill.template_path = dir.join("skills/my-agent/SKILL.md.j2");

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = HarnessOutput {
            skill_content: "rendered".to_string(),
            sidecars: vec![],
            manifest_entry: Some(r#"{"name":"my-agent"}"#.to_string()),
        };

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, false).unwrap();
        assert!(result.written.manifest_path.is_some());
        let manifest = result.written.manifest_path.unwrap();
        assert_eq!(manifest, dir.join(".claude/plugin.json"));
        assert!(manifest.exists());
        assert_eq!(written_content(&manifest), r#"{"name":"my-agent"}"#);

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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
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
            manifest_entry: None,
        };

        let result = Router::write(&pair, &output, &dir, TargetScope::Project, true).unwrap();
        assert!(result.skipped.is_empty());
        assert!(result.written.skill_path.exists());
        assert_eq!(written_content(&result.written.skill_path), "rendered");

        let _ = fs::remove_dir_all(&dir);
    }
}

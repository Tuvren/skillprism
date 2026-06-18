#![allow(dead_code)]

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

#[derive(Debug, Diagnostic, Error)]
pub enum RouterError {
    #[error("[{skill}] {harness}: Failed to write `{path}`")]
    #[diagnostic(help("{detail}"))]
    WriteError {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },

    #[error("[{skill}] {harness}: Failed to copy assets to `{path}`")]
    #[diagnostic(help("{detail}"))]
    AssetCopyError {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },
}

pub struct Router;

impl Router {
    pub fn write(
        pair: &ResolvedPair,
        output: &HarnessOutput,
        project_root: &Path,
        target: TargetScope,
    ) -> Result<WrittenFiles, RouterError> {
        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;

        let skill_path = resolve_skill_path(project_root, &pair.harness, skill_name, target);
        let skill_dir = skill_path
            .parent()
            .expect("skill path has parent")
            .to_path_buf();

        atomic_write(&skill_path, &output.skill_content).map_err(|e| RouterError::WriteError {
            skill: skill_name.clone(),
            harness: harness_id.clone(),
            path: skill_path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;

        for sidecar in &output.sidecars {
            let sidecar_path = paths::resolve_sidecar_path(
                &skill_dir,
                sidecar.output_dir.as_deref(),
                &sidecar.filename,
            );
            atomic_write(&sidecar_path, &sidecar.content).map_err(|e| RouterError::WriteError {
                skill: skill_name.clone(),
                harness: harness_id.clone(),
                path: sidecar_path.to_string_lossy().to_string(),
                detail: e.to_string(),
            })?;
        }

        let manifest_path = if let Some(ref manifest_content) = output.manifest_entry {
            if let Some(path) = paths::resolve_manifest_path(project_root, &pair.harness, target) {
                atomic_write(&path, manifest_content).map_err(|e| RouterError::WriteError {
                    skill: skill_name.clone(),
                    harness: harness_id.clone(),
                    path: path.to_string_lossy().to_string(),
                    detail: e.to_string(),
                })?;
                Some(path)
            } else {
                None
            }
        } else {
            None
        };

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

        Ok(WrittenFiles {
            skill_path,
            sidecar_paths: output
                .sidecars
                .iter()
                .map(|s| {
                    paths::resolve_sidecar_path(&skill_dir, s.output_dir.as_deref(), &s.filename)
                })
                .collect(),
            manifest_path,
        })
    }

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

pub struct WrittenFiles {
    pub skill_path: std::path::PathBuf,
    pub sidecar_paths: Vec<std::path::PathBuf>,
    pub manifest_path: Option<std::path::PathBuf>,
}

pub struct DiffEntry {
    pub path: std::path::PathBuf,
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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project).unwrap();
        assert_eq!(
            result.skill_path,
            dir.join(".claude/skills/my-agent/SKILL.md")
        );
        assert!(result.skill_path.exists());
        assert_eq!(written_content(&result.skill_path), "my-agent-rendered");

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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project).unwrap();
        assert!(result.skill_path.exists(), "directory should be created");
        assert_eq!(written_content(&result.skill_path), "content");

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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project).unwrap();
        assert!(result.sidecar_paths[0].exists());
        assert_eq!(written_content(&result.sidecar_paths[0]), "key: value");

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

        let result = Router::write(&pair, &output, &dir, TargetScope::Project).unwrap();
        assert!(result.manifest_path.is_some());
        let manifest = result.manifest_path.unwrap();
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

        Router::write(&pair, &output, &dir, TargetScope::Project).unwrap();

        let dest_refs = dir.join(".claude/skills/test-skill/references/guide.md");
        assert!(dest_refs.exists());
        assert_eq!(written_content(&dest_refs), "# Guide");

        let _ = fs::remove_dir_all(&dir);
    }
}

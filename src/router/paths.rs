use std::path::{Path, PathBuf};

use crate::cli::TargetScope;
use crate::registry::HarnessDefinition;

/// Resolves the full path to the rendered skill file for a given scope.
pub fn resolve_skill_path(
    project_root: &Path,
    harness: &HarnessDefinition,
    skill_name: &str,
    target: TargetScope,
) -> PathBuf {
    let scope_path = match target {
        TargetScope::Project => PathBuf::from(&harness.paths.project_scope_path),
        TargetScope::User => {
            let home = home_dir();
            home.join(&harness.paths.user_scope_path)
        }
        TargetScope::Dist => PathBuf::from("dist").join(&harness.id),
    };

    let base = match target {
        TargetScope::Project | TargetScope::Dist => project_root.join(scope_path),
        TargetScope::User => scope_path,
    };

    base.join(skill_name).join(&harness.paths.skill_filename)
}

/// Resolves the full path to the manifest file, if one is defined.
pub fn resolve_manifest_path(
    project_root: &Path,
    harness: &HarnessDefinition,
    target: TargetScope,
) -> Option<PathBuf> {
    let scope_path = harness.paths.manifest_scope_path.as_ref()?;
    let filename = harness.paths.manifest_filename.as_ref()?;

    let base_dir = match target {
        TargetScope::Project => project_root.join(scope_path),
        TargetScope::User => home_dir().join(scope_path),
        TargetScope::Dist => project_root.join("dist").join(&harness.id).join(scope_path),
    };

    Some(base_dir.join(filename))
}

/// Resolves the full path to a sidecar file within the skill output directory.
pub fn resolve_sidecar_path(
    skill_output_dir: &Path,
    sidecar_output_dir: Option<&str>,
    filename: &str,
) -> PathBuf {
    sidecar_output_dir.map_or_else(
        || skill_output_dir.join(filename),
        |dir| skill_output_dir.join(dir).join(filename),
    )
}

/// Returns the output directory for a skill (parent of the skill file path).
#[allow(dead_code)]
pub fn skill_output_dir(
    project_root: &Path,
    harness: &HarnessDefinition,
    skill_name: &str,
    target: TargetScope,
) -> PathBuf {
    resolve_skill_path(project_root, harness, skill_name, target)
        .parent()
        .expect("skill path should have a parent directory")
        .to_path_buf()
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;

    fn claude_harness() -> HarnessDefinition {
        HarnessRegistry::with_builtins().resolve("claude").unwrap()
    }

    #[test]
    fn project_scope_path() {
        let root = Path::new("/projects/my-skills");
        let path = resolve_skill_path(root, &claude_harness(), "test-agent", TargetScope::Project);
        assert_eq!(path, root.join(".claude/skills/test-agent/SKILL.md"));
    }

    #[test]
    fn user_scope_path() {
        let root = Path::new("/tmp/project");
        let path = resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::User);
        assert!(path.ends_with(".claude/skills/my-agent/SKILL.md"));
    }

    #[test]
    fn dist_scope_path() {
        let root = Path::new("/tmp/project");
        let path = resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::Dist);
        assert_eq!(path, root.join("dist/claude/my-agent/SKILL.md"));
    }

    #[test]
    fn manifest_path_for_harness_with_manifest() {
        let root = Path::new("/tmp/project");
        let path = resolve_manifest_path(root, &claude_harness(), TargetScope::Project);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), root.join(".claude/plugin.json"));
    }

    #[test]
    fn sidecar_path_with_output_dir() {
        let base = Path::new("/out/skill-dir");
        let path = resolve_sidecar_path(base, Some("meta"), "config.yaml");
        assert_eq!(path, base.join("meta/config.yaml"));
    }

    #[test]
    fn sidecar_path_without_output_dir() {
        let base = Path::new("/out/skill-dir");
        let path = resolve_sidecar_path(base, None, "config.yaml");
        assert_eq!(path, base.join("config.yaml"));
    }
}

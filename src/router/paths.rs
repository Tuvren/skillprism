use std::path::{Component, Path, PathBuf};

use crate::cli::TargetScope;
use crate::registry::HarnessDefinition;
use crate::router::RouterError;

/// Resolves the full path to the rendered skill file for a given scope.
pub fn resolve_skill_path(
    project_root: &Path,
    harness: &HarnessDefinition,
    skill_name: &str,
    target: TargetScope,
) -> Result<PathBuf, RouterError> {
    let (scope_path, allowed_base) = match target {
        TargetScope::Project => {
            check_no_traversal(&harness.paths.project_scope_path, skill_name, &harness.id)?;
            let base = project_root.join(&harness.paths.project_scope_path);
            (base, project_root.to_path_buf())
        }
        TargetScope::User => {
            check_no_traversal(&harness.paths.user_scope_path, skill_name, &harness.id)?;
            let home = home_dir();
            let base = home.join(&harness.paths.user_scope_path);
            (base, home)
        }
        TargetScope::Dist => {
            let base = project_root.join("dist").join(&harness.id);
            (base, project_root.to_path_buf())
        }
    };

    let resolved = scope_path.join(skill_name).join(&harness.paths.skill_filename);
    validate_scope_relative(&resolved, &allowed_base, skill_name, &harness.id)?;
    Ok(resolved)
}

/// Resolves the full path to the manifest file, if one is defined.
pub fn resolve_manifest_path(
    project_root: &Path,
    harness: &HarnessDefinition,
    target: TargetScope,
) -> Option<Result<PathBuf, RouterError>> {
    let scope_path = harness.paths.manifest_scope_path.as_ref()?;
    let filename = harness.paths.manifest_filename.as_ref()?;
    if let Err(e) = check_no_traversal(scope_path, "manifest", &harness.id) {
        return Some(Err(e));
    }

    let (base_dir, allowed_base) = match target {
        TargetScope::Project => {
            let base = project_root.join(scope_path);
            (base.clone(), base)
        }
        TargetScope::User => {
            let home = home_dir();
            (home.join(scope_path), home)
        }
        TargetScope::Dist => {
            let base = project_root.join("dist").join(&harness.id).join(scope_path);
            (base.clone(), base)
        }
    };

    let resolved = base_dir.join(filename);
    match validate_scope_relative(&resolved, &allowed_base, "manifest", &harness.id) {
        Ok(()) => Some(Ok(resolved)),
        Err(e) => Some(Err(e)),
    }
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

/// Checks that a scope path string does not contain `..` traversal components.
fn check_no_traversal(
    scope_path: &str,
    skill_name: &str,
    harness_id: &str,
) -> Result<(), RouterError> {
    for component in Path::new(scope_path).components() {
        if component == Component::ParentDir {
            return Err(RouterError::PathTraversal {
                skill: skill_name.to_string(),
                harness: harness_id.to_string(),
                resolved: scope_path.to_string(),
                allowed_base: "(scope path)".to_string(),
            });
        }
    }
    Ok(())
}

/// Validates that a resolved path stays within the allowed base directory.
///
/// Attempts `std::fs::canonicalize` for symlink-aware validation; if the path
/// does not yet exist (e.g., first build), falls back to component-level checking.
fn validate_scope_relative(
    resolved: &Path,
    allowed_base: &Path,
    skill_name: &str,
    harness_id: &str,
) -> Result<(), RouterError> {
    let (Ok(resolved_check), Ok(base_check)) = (
        resolved.canonicalize(),
        allowed_base.canonicalize(),
    ) else {
        if !resolved.starts_with(allowed_base) {
            return Err(RouterError::PathTraversal {
                skill: skill_name.to_string(),
                harness: harness_id.to_string(),
                resolved: resolved.to_string_lossy().to_string(),
                allowed_base: allowed_base.to_string_lossy().to_string(),
            });
        }
        if let Ok(relative) = resolved.strip_prefix(allowed_base) {
            for component in relative.components() {
                if component == Component::ParentDir {
                    return Err(RouterError::PathTraversal {
                        skill: skill_name.to_string(),
                        harness: harness_id.to_string(),
                        resolved: resolved.to_string_lossy().to_string(),
                        allowed_base: allowed_base.to_string_lossy().to_string(),
                    });
                }
            }
        }
        return Ok(());
    };

    if !resolved_check.starts_with(&base_check) {
        return Err(RouterError::PathTraversal {
            skill: skill_name.to_string(),
            harness: harness_id.to_string(),
            resolved: resolved_check.to_string_lossy().to_string(),
            allowed_base: base_check.to_string_lossy().to_string(),
        });
    }

    Ok(())
}

/// Returns the output directory for a skill (parent of the skill file path).
#[allow(dead_code)]
pub fn skill_output_dir(
    project_root: &Path,
    harness: &HarnessDefinition,
    skill_name: &str,
    target: TargetScope,
) -> Result<PathBuf, RouterError> {
    resolve_skill_path(project_root, harness, skill_name, target).map(|p| {
        p.parent()
            .expect("skill path should have a parent directory")
            .to_path_buf()
    })
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
        let path = resolve_skill_path(root, &claude_harness(), "test-agent", TargetScope::Project).unwrap();
        assert_eq!(path, root.join(".claude/skills/test-agent/SKILL.md"));
    }

    #[test]
    fn user_scope_path() {
        let root = Path::new("/tmp/project");
        let path = resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::User).unwrap();
        assert!(path.ends_with(".claude/skills/my-agent/SKILL.md"));
    }

    #[test]
    fn dist_scope_path() {
        let root = Path::new("/tmp/project");
        let path = resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::Dist).unwrap();
        assert_eq!(path, root.join("dist/claude/my-agent/SKILL.md"));
    }

    #[test]
    fn manifest_path_for_harness_with_manifest() {
        let root = Path::new("/tmp/project");
        let path = resolve_manifest_path(root, &claude_harness(), TargetScope::Project);
        assert!(path.is_some());
        assert_eq!(path.unwrap().unwrap(), root.join(".claude/plugin.json"));
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

    #[test]
    fn rejects_traversal_in_project_scope_path() {
        let root = Path::new("/tmp/project");
        let mut harness = claude_harness();
        harness.paths.project_scope_path = "../outside".to_string();

        let result = resolve_skill_path(root, &harness, "evil", TargetScope::Project);
        match result {
            Err(RouterError::PathTraversal { .. }) => {}
            other => panic!("expected PathTraversal, got {other:?}"),
        }
    }

    #[test]
    fn rejects_traversal_in_user_scope_path() {
        let root = Path::new("/tmp/project");
        let mut harness = claude_harness();
        harness.paths.user_scope_path = "../../escape".to_string();

        let result = resolve_skill_path(root, &harness, "evil", TargetScope::User);
        match result {
            Err(RouterError::PathTraversal { .. }) => {}
            other => panic!("expected PathTraversal, got {other:?}"),
        }
    }

    #[test]
    fn rejects_manifest_path_traversal() {
        let root = Path::new("/tmp/project");
        let mut harness = claude_harness();
        harness.paths.manifest_scope_path = Some("../outside".to_string());

        let result = resolve_manifest_path(root, &harness, TargetScope::Project);
        assert!(result.is_some());
        match result.unwrap() {
            Err(RouterError::PathTraversal { .. }) => {}
            other => panic!("expected PathTraversal, got {other:?}"),
        }
    }

    #[test]
    fn valid_path_through_dist_scope_succeeds() {
        let root = Path::new("/tmp/project");
        let path = resolve_skill_path(root, &claude_harness(), "agent", TargetScope::Dist).unwrap();
        assert_eq!(path, root.join("dist/claude/agent/SKILL.md"));
    }

    #[test]
    fn rejects_traversal_in_user_scope_manifest_path() {
        let root = Path::new("/tmp/project");
        let mut harness = claude_harness();
        harness.paths.manifest_scope_path = Some("../hijack".to_string());

        let result = resolve_manifest_path(root, &harness, TargetScope::User);
        assert!(result.is_some());
        match result.unwrap() {
            Err(RouterError::PathTraversal { .. }) => {}
            other => panic!("expected PathTraversal, got {other:?}"),
        }
    }
}

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

use std::path::{Component, Path, PathBuf};

use crate::registry::HarnessDefinition;
use crate::router::RouterError;

/// Target scope for where rendered skill files are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetScope {
    /// Write to the project-local skill directory.
    Project,
    /// Write to the user's home directory.
    User,
    /// Write to a distribution output directory.
    Dist,
}

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
            let anchor = user_scope_anchor(harness)?;
            let base = anchor.join(&harness.paths.user_scope_path);
            (base, anchor)
        }
        TargetScope::Dist => {
            let base = project_root.join("dist").join(&harness.id);
            (base, project_root.to_path_buf())
        }
    };

    let resolved = scope_path
        .join(skill_name)
        .join(&harness.paths.skill_filename);
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
            let anchor = match user_scope_anchor(harness) {
                Ok(anchor) => anchor,
                Err(e) => return Some(Err(e)),
            };
            (anchor.join(scope_path), anchor)
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
///
/// Returns an error if `filename` or `sidecar_output_dir` contains `..` components
/// or is an absolute path, or if the resulting path escapes the skill output directory.
pub fn resolve_sidecar_path(
    skill_output_dir: &Path,
    sidecar_output_dir: Option<&str>,
    filename: &str,
    skill_name: &str,
    harness_id: &str,
) -> Result<PathBuf, RouterError> {
    if Path::new(filename).is_absolute() {
        return Err(RouterError::AbsolutePathDisallowed {
            skill: skill_name.to_string(),
            harness: harness_id.to_string(),
            component: filename.to_string(),
        });
    }
    for component in Path::new(filename).components() {
        if component == Component::ParentDir {
            return Err(RouterError::PathTraversal {
                skill: skill_name.to_string(),
                harness: harness_id.to_string(),
                resolved: filename.to_string(),
                allowed_base: "(sidecar filename)".to_string(),
            });
        }
    }

    let combined = match sidecar_output_dir {
        Some(dir) => {
            if Path::new(dir).is_absolute() {
                return Err(RouterError::AbsolutePathDisallowed {
                    skill: skill_name.to_string(),
                    harness: harness_id.to_string(),
                    component: dir.to_string(),
                });
            }
            for component in Path::new(dir).components() {
                if component == Component::ParentDir {
                    return Err(RouterError::PathTraversal {
                        skill: skill_name.to_string(),
                        harness: harness_id.to_string(),
                        resolved: dir.to_string(),
                        allowed_base: "(sidecar output_dir)".to_string(),
                    });
                }
            }
            skill_output_dir.join(dir).join(filename)
        }
        None => skill_output_dir.join(filename),
    };

    if !combined.starts_with(skill_output_dir) {
        return Err(RouterError::PathTraversal {
            skill: skill_name.to_string(),
            harness: harness_id.to_string(),
            resolved: combined.to_string_lossy().to_string(),
            allowed_base: skill_output_dir.to_string_lossy().to_string(),
        });
    }

    Ok(combined)
}

/// Resolves the full path to a rendered non-skill `.j2` template within the
/// skill output directory.
///
/// `relative_path` is the `.j2` source path relative to the skill directory
/// with the suffix already stripped. Returns an error if it is absolute, has
/// `..` components, or escapes the skill output directory.
pub fn resolve_rendered_path(
    skill_output_dir: &Path,
    relative_path: &Path,
    skill_name: &str,
    harness_id: &str,
) -> Result<PathBuf, RouterError> {
    if relative_path.is_absolute() {
        return Err(RouterError::AbsolutePathDisallowed {
            skill: skill_name.to_string(),
            harness: harness_id.to_string(),
            component: relative_path.to_string_lossy().to_string(),
        });
    }
    for component in relative_path.components() {
        if component == Component::ParentDir {
            return Err(RouterError::PathTraversal {
                skill: skill_name.to_string(),
                harness: harness_id.to_string(),
                resolved: relative_path.to_string_lossy().to_string(),
                allowed_base: skill_output_dir.to_string_lossy().to_string(),
            });
        }
    }

    let combined = skill_output_dir.join(relative_path);
    if !combined.starts_with(skill_output_dir) {
        return Err(RouterError::PathTraversal {
            skill: skill_name.to_string(),
            harness: harness_id.to_string(),
            resolved: combined.to_string_lossy().to_string(),
            allowed_base: skill_output_dir.to_string_lossy().to_string(),
        });
    }
    Ok(combined)
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
    let (Ok(resolved_check), Ok(base_check)) =
        (resolved.canonicalize(), allowed_base.canonicalize())
    else {
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

fn home_dir() -> Result<PathBuf, RouterError> {
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() => Ok(PathBuf::from(h)),
        _ => Err(RouterError::MissingHome),
    }
}

/// Directory that a harness's `user_scope_path` is joined to.
///
/// `OpenCode` follows the skills CLI: `opencode/skills` is relative to
/// `$XDG_CONFIG_HOME`, or `$HOME/.config` when that variable is unset or empty.
/// The harness schema has no base field, so this is selected by harness id.
/// The returned directory is the allowed base, including when it is not under
/// `$HOME`. Every other harness stays relative to `$HOME`.
pub fn user_scope_anchor(harness: &HarnessDefinition) -> Result<PathBuf, RouterError> {
    if harness.id == "opencode" {
        xdg_config_home()
    } else {
        home_dir()
    }
}

pub fn xdg_config_home() -> Result<PathBuf, RouterError> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg));
        }
    }
    Ok(home_dir()?.join(".config"))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;

    #[must_use = "saving guard to a variable is required to keep env var overridden until scope ends"]
    pub struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        pub(crate) fn set(key: &'static str, value: &Path) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        pub(crate) fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(ref val) = self.previous {
                unsafe {
                    std::env::set_var(self.key, val);
                }
            } else {
                unsafe {
                    std::env::remove_var(self.key);
                }
            }
        }
    }

    /// Serialises tests that mutate the global `HOME` env var so they don't
    /// race under parallel test execution.
    pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_home_for_test() -> (tempfile::TempDir, EnvGuard) {
        let tmp = tempfile::tempdir().unwrap();
        let guard = EnvGuard::set("HOME", tmp.path());
        (tmp, guard)
    }

    fn claude_harness() -> HarnessDefinition {
        HarnessRegistry::with_builtins().resolve("claude").unwrap()
    }

    #[test]
    fn project_scope_path() {
        let root = Path::new("/projects/my-skills");
        let path = resolve_skill_path(root, &claude_harness(), "test-agent", TargetScope::Project)
            .unwrap();
        assert_eq!(path, root.join(".claude/skills/test-agent/SKILL.md"));
    }

    #[test]
    fn user_scope_path() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (tmp, _guard) = set_home_for_test();
        let home = tmp.path();
        let root = Path::new("/tmp/project");
        let path =
            resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::User).unwrap();
        assert!(path.ends_with(".claude/skills/my-agent/SKILL.md"));
        assert!(path.starts_with(home));
    }

    #[test]
    fn dist_scope_path() {
        let root = Path::new("/tmp/project");
        let path =
            resolve_skill_path(root, &claude_harness(), "my-agent", TargetScope::Dist).unwrap();
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
        let path =
            resolve_sidecar_path(base, Some("meta"), "config.yaml", "skill", "harness").unwrap();
        assert_eq!(path, base.join("meta/config.yaml"));
    }

    #[test]
    fn sidecar_path_without_output_dir() {
        let base = Path::new("/out/skill-dir");
        let path = resolve_sidecar_path(base, None, "config.yaml", "skill", "harness").unwrap();
        assert_eq!(path, base.join("config.yaml"));
    }

    #[test]
    fn sidecar_rejects_absolute_filename() {
        let base = Path::new("/out/skill-dir");
        let err = resolve_sidecar_path(base, None, "/etc/passwd", "s", "h").unwrap_err();
        assert!(matches!(err, RouterError::AbsolutePathDisallowed { .. }));
    }

    #[test]
    fn sidecar_rejects_traversal_in_filename() {
        let base = Path::new("/out/skill-dir");
        let err = resolve_sidecar_path(base, None, "../../escape.yaml", "s", "h").unwrap_err();
        assert!(matches!(err, RouterError::PathTraversal { .. }));
    }

    #[test]
    fn sidecar_rejects_absolute_output_dir() {
        let base = Path::new("/out/skill-dir");
        let err = resolve_sidecar_path(base, Some("/etc"), "cfg.yaml", "s", "h").unwrap_err();
        assert!(matches!(err, RouterError::AbsolutePathDisallowed { .. }));
    }

    #[test]
    fn sidecar_rejects_traversal_in_output_dir() {
        let base = Path::new("/out/skill-dir");
        let err =
            resolve_sidecar_path(base, Some("../../escape"), "cfg.yaml", "s", "h").unwrap_err();
        assert!(matches!(err, RouterError::PathTraversal { .. }));
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
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (_tmp, _home) = set_home_for_test();
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
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (_tmp, _home) = set_home_for_test();
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

    #[test]
    fn opencode_user_scope_follows_xdg_config_home_outside_home() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = tempfile::tempdir().unwrap();
        let xdg = tempfile::tempdir().unwrap();
        let _home_guard = EnvGuard::set("HOME", home.path());
        let _xdg_guard = EnvGuard::set("XDG_CONFIG_HOME", xdg.path());

        let harness = HarnessRegistry::with_builtins()
            .resolve("opencode")
            .unwrap();
        let path = resolve_skill_path(
            Path::new("/tmp/project"),
            &harness,
            "plain-skill",
            TargetScope::User,
        )
        .unwrap();
        assert_eq!(
            path,
            xdg.path().join("opencode/skills/plain-skill/SKILL.md")
        );
    }

    #[test]
    fn opencode_user_scope_defaults_to_home_config_dir() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = tempfile::tempdir().unwrap();
        let _xdg_guard = EnvGuard::remove("XDG_CONFIG_HOME");
        let _home_guard = EnvGuard::set("HOME", home.path());

        let harness = HarnessRegistry::with_builtins()
            .resolve("opencode")
            .unwrap();
        let path = resolve_skill_path(
            Path::new("/tmp/project"),
            &harness,
            "plain-skill",
            TargetScope::User,
        )
        .unwrap();
        assert_eq!(
            path,
            home.path()
                .join(".config/opencode/skills/plain-skill/SKILL.md")
        );
    }

    #[test]
    fn user_scope_reports_missing_home() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _guard = EnvGuard::remove("HOME");

        let root = Path::new("/tmp/project");
        let harness = claude_harness();
        let result = resolve_skill_path(root, &harness, "agent", TargetScope::User);

        match result {
            Err(RouterError::MissingHome) => {}
            other => panic!("expected MissingHome, got {other:?}"),
        }
    }

    #[test]
    fn missing_home_error_formatting() {
        let err = RouterError::MissingHome;
        let msg = format!("{err}");
        assert!(
            msg.contains("HOME"),
            "error should mention HOME, got: {msg}"
        );
    }
}

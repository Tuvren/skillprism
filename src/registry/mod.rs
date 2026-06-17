#![allow(dead_code)]

mod types;

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::ProjectError;
pub use types::*;

pub struct HarnessRegistry {
    builtins: BTreeMap<String, HarnessDefinition>,
    user_overrides: BTreeMap<String, HarnessDefinition>,
}

impl HarnessRegistry {
    pub const fn new() -> Self {
        Self {
            builtins: BTreeMap::new(),
            user_overrides: BTreeMap::new(),
        }
    }

    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.load_builtins();
        registry
    }

    fn load_builtins(&mut self) {
        let harnesses = builtin_sources();
        for (id, yaml) in harnesses {
            if let Ok(def) = yaml_serde::from_str::<HarnessDefinition>(yaml) {
                self.builtins.insert(id.to_string(), def);
            }
        }
    }

    pub fn load_user_overrides(&mut self, project_root: &Path) -> Result<(), ProjectError> {
        let harnesses_dir = project_root.join("harnesses");
        if !harnesses_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&harnesses_dir).map_err(|e| ProjectError::ConfigRead {
            path: harnesses_dir.to_string_lossy().to_string(),
            source: e,
        })? {
            let entry = entry.map_err(|e| ProjectError::ConfigRead {
                path: harnesses_dir.to_string_lossy().to_string(),
                source: e,
            })?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                let content =
                    std::fs::read_to_string(&path).map_err(|e| ProjectError::ConfigRead {
                        path: path.to_string_lossy().to_string(),
                        source: e,
                    })?;
                if let Ok(def) = yaml_serde::from_str::<HarnessDefinition>(&content) {
                    let id = def.id.clone();
                    self.builtins.remove(&id);
                    self.user_overrides.insert(id, def);
                }
            }
        }

        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Result<HarnessDefinition, ProjectError> {
        self.user_overrides
            .get(name)
            .or_else(|| self.builtins.get(name))
            .cloned()
            .ok_or_else(|| ProjectError::MissingField {
                path: name.to_string(),
                message: format!("Unknown harness: '{name}'. Available: {}", self.available()),
            })
    }

    pub fn available(&self) -> String {
        let mut names: Vec<&str> = self
            .builtins
            .keys()
            .chain(self.user_overrides.keys())
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        names.join(", ")
    }
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

fn builtin_sources() -> Vec<(&'static str, &'static str)> {
    vec![
        ("claude", include_str!("../builtin_harnesses/claude.yaml")),
        ("codex", include_str!("../builtin_harnesses/codex.yaml")),
        (
            "opencode",
            include_str!("../builtin_harnesses/opencode.yaml"),
        ),
        ("factory", include_str!("../builtin_harnesses/factory.yaml")),
        ("pi", include_str!("../builtin_harnesses/pi.yaml")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_builtin_by_name() {
        let registry = HarnessRegistry::with_builtins();
        let claude = registry.resolve("claude").unwrap();
        assert_eq!(claude.id, "claude");
        assert_eq!(claude.name, "Claude Code");
        assert!(claude.capabilities.supports_subagent);
        assert_eq!(claude.paths.skill_filename, "SKILL.md");
        assert_eq!(claude.skill_ref_pattern, Some("/{name}".to_string()));
    }

    #[test]
    fn resolve_builtin_has_required_fields() {
        let registry = HarnessRegistry::with_builtins();
        for name in ["claude", "codex", "opencode", "factory", "pi"] {
            let def = registry.resolve(name).unwrap();
            assert!(!def.id.is_empty(), "{name}: id empty");
            assert!(!def.name.is_empty(), "{name}: name empty");
            assert!(
                !def.paths.skill_filename.is_empty(),
                "{name}: skill_filename empty"
            );
            assert!(
                !def.paths.project_scope_path.is_empty(),
                "{name}: project_scope_path empty"
            );
            assert!(
                !def.paths.user_scope_path.is_empty(),
                "{name}: user_scope_path empty"
            );
        }
    }

    #[test]
    fn resolve_user_override_over_builtin() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("override_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("harnesses")).unwrap();

        let override_yaml = r#"
id: opencode
name: OpenCode Custom
version: 0.1.0
capabilities:
  supports_subagent: false
  frontmatter_mode: lenient
paths:
  project_scope_path: custom/skills
  user_scope_path: custom/skills
  skill_filename: SKILL.md
"#;
        std::fs::write(dir.join("harnesses/opencode.yaml"), override_yaml).unwrap();

        let mut registry = HarnessRegistry::with_builtins();
        registry.load_user_overrides(&dir).unwrap();

        let def = registry.resolve("opencode").unwrap();
        assert_eq!(def.name, "OpenCode Custom");
        assert_eq!(def.paths.project_scope_path, "custom/skills");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_unknown_harness_errors() {
        let registry = HarnessRegistry::with_builtins();
        let result = registry.resolve("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::MissingField { path, message } => {
                assert_eq!(path, "nonexistent");
                assert!(message.contains("Unknown harness"));
            }
            e => panic!("expected MissingField error, got {e:?}"),
        }
    }
}

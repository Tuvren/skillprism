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

mod types;

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::ProjectError;
pub use types::*;

/// Registry of known harness definitions, supporting builtins and user overrides.
pub struct HarnessRegistry {
    builtins: BTreeMap<String, HarnessDefinition>,
    user_overrides: BTreeMap<String, HarnessDefinition>,
}

impl HarnessRegistry {
    /// Returns an empty registry with no builtin harnesses loaded.
    ///
    /// Use [`with_builtins`](Self::with_builtins) or `Default::default()`
    /// to include the five standard harnesses (claude, codex, opencode, factory, pi).
    pub const fn new() -> Self {
        Self {
            builtins: BTreeMap::new(),
            user_overrides: BTreeMap::new(),
        }
    }

    /// Creates a registry with the five built-in harnesses loaded.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.load_builtins();
        registry
    }

    fn load_builtins(&mut self) {
        let harnesses = builtin_sources();
        for (id, yaml) in harnesses {
            let def = yaml_serde::from_str::<HarnessDefinition>(yaml)
                .unwrap_or_else(|e| panic!("builtin harness '{id}': malformed YAML: {e}"));
            self.builtins.insert(id.to_string(), def);
        }
    }

    /// Loads user-provided harness YAML files from a directory, overriding builtins.
    pub fn load_user_overrides(&mut self, harnesses_dir: &Path) -> Result<(), ProjectError> {
        if !harnesses_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(harnesses_dir).map_err(|e| ProjectError::ConfigRead {
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
                let def = yaml_serde::from_str::<HarnessDefinition>(&content).map_err(|e| {
                    ProjectError::YamlParse {
                        path: path.to_string_lossy().to_string(),
                        line: e.location().map_or(0, |l| l.line()),
                        message: format!(
                            "Failed to parse harness override: {} — {e}",
                            path.display()
                        ),
                    }
                })?;
                let id = def.id.clone();
                self.builtins.remove(&id);
                self.user_overrides.insert(id, def);
            }
        }

        Ok(())
    }

    /// Resolves a harness by name, preferring user overrides over builtins.
    pub fn resolve(&self, name: &str) -> Result<HarnessDefinition, ProjectError> {
        self.user_overrides
            .get(name)
            .or_else(|| self.builtins.get(name))
            .cloned()
            .ok_or_else(|| ProjectError::UnknownHarness {
                name: name.to_string(),
                message: format!("Available harnesses: {}", self.available()),
            })
    }

    /// Returns a comma-separated list of available harness names.
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

/// IDs of the built-in harnesses shipped with skillprism.
pub const BUILTIN_HARNESS_IDS: &[&str] = &["claude", "codex", "opencode", "factory", "pi"];

fn builtin_sources() -> Vec<(&'static str, &'static str)> {
    BUILTIN_HARNESS_IDS
        .iter()
        .map(|id| (*id, builtin_yaml(id)))
        .collect()
}

fn builtin_yaml(id: &str) -> &'static str {
    match id {
        "claude" => include_str!("../builtin_harnesses/claude.yaml"),
        "codex" => include_str!("../builtin_harnesses/codex.yaml"),
        "opencode" => include_str!("../builtin_harnesses/opencode.yaml"),
        "factory" => include_str!("../builtin_harnesses/factory.yaml"),
        "pi" => include_str!("../builtin_harnesses/pi.yaml"),
        _ => panic!("unknown builtin harness: {id}"),
    }
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

        let override_yaml = r"
id: opencode
name: OpenCode Custom
version: 0.1.0
capabilities:
  supports_subagent: false
paths:
  project_scope_path: custom/skills
  user_scope_path: custom/skills
  skill_filename: SKILL.md
";
        std::fs::write(dir.join("harnesses/opencode.yaml"), override_yaml).unwrap();

        let mut registry = HarnessRegistry::with_builtins();
        registry
            .load_user_overrides(&dir.join("harnesses"))
            .unwrap();

        let def = registry.resolve("opencode").unwrap();
        assert_eq!(def.name, "OpenCode Custom");
        assert_eq!(def.paths.project_scope_path, "custom/skills");

        drop_dir(&dir);
    }

    fn drop_dir(path: &std::path::Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn resolve_unknown_harness_errors() {
        let registry = HarnessRegistry::with_builtins();
        let result = registry.resolve("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::UnknownHarness { name, message } => {
                assert_eq!(name, "nonexistent");
                assert!(message.contains("Available harnesses"));
            }
            e => panic!("expected UnknownHarness error, got {e:?}"),
        }
    }
}

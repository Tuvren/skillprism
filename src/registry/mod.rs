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

use crate::types::{ConfigKind, ProjectError};
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
                let def: HarnessDefinition =
                    crate::loader::deserialize_config(&content, &path, ConfigKind::Harness)?;
                let id = def.id.clone();
                self.builtins.remove(&id);
                self.user_overrides.insert(id, def);
            }
        }

        Ok(())
    }

    /// Resolves a harness by name, preferring user overrides over builtins.
    ///
    /// An exact id wins. Skills CLI agent ids (`claude-code`, `droid`) resolve
    /// only when no harness is registered under that exact name.
    pub fn resolve(&self, name: &str) -> Result<HarnessDefinition, ProjectError> {
        self.lookup(name)
            .or_else(|| alias_of(name).and_then(|id| self.lookup(id)))
            .cloned()
            .ok_or_else(|| ProjectError::UnknownHarness {
                name: name.to_string(),
                message: format!("Available harnesses: {}", self.available()),
            })
    }

    fn lookup(&self, name: &str) -> Option<&HarnessDefinition> {
        self.user_overrides
            .get(name)
            .or_else(|| self.builtins.get(name))
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

    /// Skillprism id for `name`. An exact registered id wins over a built-in alias.
    ///
    /// `droid` and `claude-code` map to Factory and Claude only when no harness
    /// is registered under that exact id.
    pub fn canonical_id<'a>(&'a self, name: &'a str) -> &'a str {
        if self.lookup(name).is_some() {
            name
        } else {
            canonical_harness_id(name)
        }
    }

    /// Canonicalizes harness names, dropping blanks and duplicates.
    ///
    /// Exact user-harness ids are preserved. Built-in aliases apply only when
    /// that exact id is not registered. First-seen order is preserved.
    pub fn selected_ids<I, S>(&self, names: I) -> Vec<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut out = Vec::new();
        for name in names {
            let trimmed = name.as_ref().trim();
            if trimmed.is_empty() {
                continue;
            }
            let id = self.canonical_id(trimmed);
            if !out.iter().any(|existing| existing == id) {
                out.push(id.to_string());
            }
        }
        out
    }

    /// Returns a sorted list of all available harness IDs (built-ins and user overrides).
    pub fn all_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .builtins
            .keys()
            .chain(self.user_overrides.keys())
            .cloned()
            .collect();
        ids.sort_unstable();
        ids
    }
}

impl Default for HarnessRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// IDs of the built-in harnesses shipped with skillprism.
pub const BUILTIN_HARNESS_IDS: &[&str] = &["claude", "codex", "opencode", "factory", "pi"];

/// Maps a skills CLI agent id onto the skillprism harness id.
///
/// `claude-code` is Claude and `droid` is Factory. Unknown names, including
/// skillprism ids, return `None`.
pub fn alias_of(name: &str) -> Option<&'static str> {
    match name {
        "claude-code" => Some("claude"),
        "droid" => Some("factory"),
        _ => None,
    }
}

/// Returns the skillprism id for a skills CLI alias, or `name` when it is not one.
#[must_use]
pub fn canonical_harness_id(name: &str) -> &str {
    alias_of(name).unwrap_or(name)
}

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
    fn config_diagnostics_harness_overrides() {
        use miette::Diagnostic;

        let cases = [
            ("typo: custom\n", "unknown field `typo`", false),
            ("name: Custom\n", "missing field `id`", false),
            (
                "id: custom\nname: Custom\ncapabilities: text\n",
                "capabilities: invalid type",
                false,
            ),
            (
                "id: custom\nname: Custom\ncapabilities: [text",
                "YAML does not parse",
                true,
            ),
        ];
        for (content, reason, syntax) in cases {
            let tmp = tempfile::tempdir().unwrap();
            std::fs::write(tmp.path().join("custom.yaml"), content).unwrap();
            let mut registry = HarnessRegistry::new();
            let error = registry.load_user_overrides(tmp.path()).unwrap_err();
            assert_eq!(matches!(error, ProjectError::YamlSyntax { .. }), syntax);
            let message = error.to_string();
            assert!(message.contains("custom.yaml"), "{message}");
            assert!(message.contains(reason), "{message}");
            let has_location = !reason.starts_with("missing field");
            assert_eq!(message.contains("column"), has_location, "{message}");
            assert!(!message.contains("Invalid YAML"), "{message}");
            assert_eq!(error.labels().unwrap().next().is_some(), has_location);
            assert!(error.source_code().is_some());
            assert!(registry.all_ids().is_empty());
        }
    }

    #[test]
    fn resolve_skills_cli_aliases_to_builtin_ids() {
        let registry = HarnessRegistry::with_builtins();
        assert_eq!(registry.resolve("claude-code").unwrap().id, "claude");
        assert_eq!(registry.resolve("droid").unwrap().id, "factory");
        assert_eq!(registry.canonical_id("droid"), "factory");
        assert_eq!(registry.canonical_id("claude-code"), "claude");
        assert_eq!(canonical_harness_id("droid"), "factory");
        assert_eq!(canonical_harness_id("opencode"), "opencode");
    }

    #[test]
    fn exact_user_harness_id_wins_over_builtin_alias() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("harnesses")).unwrap();
        for id in ["droid", "claude-code"] {
            std::fs::write(
                tmp.path().join(format!("harnesses/{id}.yaml")),
                format!(
                    "id: {id}\nname: {id}\ncapabilities:\n  supports_subagent: false\npaths:\n  project_scope_path: .{id}/skills\n  user_scope_path: .{id}/skills\n  skill_filename: SKILL.md\n"
                ),
            )
            .unwrap();
        }
        let mut registry = HarnessRegistry::with_builtins();
        registry
            .load_user_overrides(tmp.path().join("harnesses").as_path())
            .unwrap();
        assert_eq!(registry.canonical_id("droid"), "droid");
        assert_eq!(registry.canonical_id("claude-code"), "claude-code");
        assert_eq!(registry.resolve("droid").unwrap().id, "droid");
        assert_eq!(registry.resolve("claude-code").unwrap().id, "claude-code");
        assert_eq!(
            registry.selected_ids(["droid", "claude-code", "droid"]),
            vec!["droid".to_string(), "claude-code".to_string()]
        );
        // The built-in targets remain selectable by their own ids.
        assert_eq!(registry.canonical_id("factory"), "factory");
        assert_eq!(registry.canonical_id("claude"), "claude");
    }

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
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("harnesses")).unwrap();

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
        std::fs::write(dir.path().join("harnesses/opencode.yaml"), override_yaml).unwrap();

        let mut registry = HarnessRegistry::with_builtins();
        registry
            .load_user_overrides(&dir.path().join("harnesses"))
            .unwrap();

        let def = registry.resolve("opencode").unwrap();
        assert_eq!(def.name, "OpenCode Custom");
        assert_eq!(def.paths.project_scope_path, "custom/skills");
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

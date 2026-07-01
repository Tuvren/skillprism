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

use miette::Diagnostic;
use thiserror::Error;

use crate::registry::{HarnessDefinition, HarnessRegistry};
use crate::types::{ProjectError, ProjectModel, SkillModel};

/// A resolved skill-harness pair ready for validation or rendering.
#[derive(Debug, Clone)]
pub struct ResolvedPair {
    /// The skill being rendered.
    pub skill: SkillModel,
    /// The harness the skill is rendered for.
    pub harness: HarnessDefinition,
}

/// Errors that occur during skill-harness resolution.
#[derive(Debug, Diagnostic, Error)]
pub enum ResolveError {
    /// The named harness was not found in the registry.
    #[error("Unknown harness `{harness_name}` for skill `{skill_name}`")]
    #[diagnostic(help("Available harnesses: {available}"))]
    UnknownHarness {
        skill_name: String,
        harness_name: String,
        available: String,
    },

    /// The skill requires a capability the harness does not support.
    #[error(
        "Skill `{skill_name}` requires capability `{capability}` but harness `{harness_name}` does not support it"
    )]
    #[diagnostic(help(
        "This skill is skipped for `{harness_name}` — it still builds for any other \
         configured harness that satisfies `{capability}`. Remove the \
         required-capability from the skill, or drop `{harness_name}` from \
         skillprism.yaml, if you don't want it skipped here."
    ))]
    MissingCapability {
        skill_name: String,
        harness_name: String,
        capability: String,
    },
}

/// Outcome of resolving a project's skills against its configured harnesses.
///
/// Unlike an unknown harness name (a project misconfiguration that aborts the whole
/// build), a single skill-harness pair failing a capability check doesn't invalidate
/// the rest of the project — that skill simply isn't shipped for that harness. This
/// mirrors `validator::ValidationOutcome`'s accumulate-don't-abort pattern.
pub struct ResolveOutcome {
    /// Pairs that resolved successfully.
    pub resolved: Vec<ResolvedPair>,
    /// Pairs skipped because the skill required a capability the harness doesn't
    /// support — non-fatal; every other pair in the project still builds.
    pub skipped: Vec<ResolveError>,
    /// Errors that abort the whole resolution (currently: an unknown harness name in
    /// `skillprism.yaml`, which is a project misconfiguration, not a per-skill issue).
    pub fatal: Vec<ResolveError>,
}

/// Resolves skills to their target harnesses, producing renderable pairs.
#[derive(Debug, Default)]
pub struct HarnessResolver;

impl HarnessResolver {
    /// Resolves all skills in a project against the project's configured harnesses.
    ///
    /// Never short-circuits: every skill × harness combination is attempted, and
    /// capability mismatches only remove that one pair (see `ResolveOutcome::skipped`)
    /// rather than the whole project.
    pub fn resolve_project(model: &ProjectModel, registry: &HarnessRegistry) -> ResolveOutcome {
        let mut resolved = Vec::new();
        let mut skipped = Vec::new();
        let mut fatal = Vec::new();

        for skill in &model.skills {
            for harness_name in &model.config.harnesses {
                match Self::resolve_skill_harness(skill, harness_name, registry) {
                    Ok(pair) => resolved.push(pair),
                    Err(e @ ResolveError::MissingCapability { .. }) => skipped.push(e),
                    Err(e @ ResolveError::UnknownHarness { .. }) => fatal.push(e),
                }
            }
        }

        ResolveOutcome {
            resolved,
            skipped,
            fatal,
        }
    }

    /// Resolves a single skill against a named harness.
    pub fn resolve_skill_harness(
        skill: &SkillModel,
        harness_name: &str,
        registry: &HarnessRegistry,
    ) -> Result<ResolvedPair, ResolveError> {
        let harness = registry.resolve(harness_name).map_err(|e| match e {
            ProjectError::UnknownHarness { name, message } => ResolveError::UnknownHarness {
                skill_name: skill.name.clone(),
                harness_name: name,
                available: message
                    .strip_prefix("Available harnesses: ")
                    .unwrap_or(&message)
                    .to_string(),
            },
            _ => ResolveError::UnknownHarness {
                skill_name: skill.name.clone(),
                harness_name: harness_name.to_string(),
                available: String::new(),
            },
        })?;

        for capability in &skill.required_capabilities {
            if !harness_has_capability(&harness, capability) {
                return Err(ResolveError::MissingCapability {
                    skill_name: skill.name.clone(),
                    harness_name: harness_name.to_string(),
                    capability: capability.clone(),
                });
            }
        }

        Ok(ResolvedPair {
            skill: skill.clone(),
            harness,
        })
    }
}

fn harness_has_capability(harness: &HarnessDefinition, capability: &str) -> bool {
    match capability {
        "subagent" => harness.capabilities.supports_subagent,
        "sidecar" => harness.capabilities.requires_sidecar,
        "manifest" => harness.capabilities.requires_manifest,
        "allowed_tools" | "allowed-tools" => harness.capabilities.supports_allowed_tools,
        "disable_model_invocation" | "disable-model-invocation" => {
            harness.capabilities.supports_disable_model_invocation
        }
        "user_invocable" | "user-invocable" => harness.capabilities.supports_user_invocable_flag,
        _ => false,
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;
    use std::collections::BTreeMap;

    fn test_registry() -> HarnessRegistry {
        HarnessRegistry::with_builtins()
    }

    pub fn test_skill(name: &str, required_capabilities: Vec<String>) -> SkillModel {
        SkillModel {
            name: name.to_string(),
            directory_name: name.to_string(),
            description: String::new(),
            version: None,
            license: None,
            compatibility: None,
            metadata: BTreeMap::new(),
            allowed_tools: None,
            when_to_use: None,
            argument_hint: None,
            arguments: None,
            disable_model_invocation: None,
            user_invocable: None,
            disallowed_tools: None,
            model_override: None,
            effort: None,
            context_fork: false,
            agent: None,
            hooks: None,
            activation_paths: None,
            shell: None,
            required_capabilities,
            variables: BTreeMap::new(),
            template_path: std::path::PathBuf::new(),
            asset_dirs: Vec::new(),
            harness_overrides: BTreeMap::new(),
        }
    }

    #[test]
    fn resolve_skill_to_builtin_harness() {
        let registry = test_registry();
        let skill = test_skill("my-agent", vec![]);

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();

        assert_eq!(pair.harness.id, "claude");
        assert_eq!(pair.skill.name, "my-agent");
        assert_eq!(pair.harness.name, "Claude Code");
    }

    #[test]
    fn resolve_unknown_harness_error() {
        let registry = test_registry();
        let skill = test_skill("my-agent", vec![]);

        let result = HarnessResolver::resolve_skill_harness(&skill, "nonexistent", &registry);
        assert!(result.is_err());
        match result.unwrap_err() {
            ResolveError::UnknownHarness { .. } => {}
            e @ ResolveError::MissingCapability { .. } => {
                panic!("expected UnknownHarness, got {e:?}")
            }
        }
    }

    #[test]
    fn capability_match_success() {
        let registry = test_registry();
        let skill = test_skill("sub-agent", vec!["subagent".to_string()]);

        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        assert_eq!(pair.harness.id, "claude");
    }

    #[test]
    fn capability_mismatch_error() {
        let registry = test_registry();
        let skill = test_skill("custom-agent", vec!["allowed_tools".to_string()]);

        let result = HarnessResolver::resolve_skill_harness(&skill, "pi", &registry);
        assert!(result.is_err());
        match result.unwrap_err() {
            ResolveError::MissingCapability { ref capability, .. } => {
                assert_eq!(capability, "allowed_tools");
            }
            e @ ResolveError::UnknownHarness { .. } => {
                panic!("expected MissingCapability, got {e:?}")
            }
        }
    }

    #[test]
    fn resolve_project_returns_all_pairs() {
        let registry = test_registry();
        let skills = vec![test_skill("skill-a", vec![]), test_skill("skill-b", vec![])];
        let model = ProjectModel {
            config: crate::types::ProjectConfig {
                harnesses: vec!["claude".to_string(), "opencode".to_string()],
                ..Default::default()
            },
            skills,
            project_root: std::path::PathBuf::from("/tmp/test"),
        };

        let outcome = HarnessResolver::resolve_project(&model, &registry);
        assert_eq!(outcome.resolved.len(), 4);
        assert!(outcome.skipped.is_empty());
        assert!(outcome.fatal.is_empty());
    }

    #[test]
    fn resolve_project_treats_unknown_harness_as_fatal() {
        let registry = test_registry();

        let cfg = crate::types::ProjectConfig {
            harnesses: vec!["claude".to_string(), "nonexistent".to_string()],
            ..Default::default()
        };

        let model = ProjectModel {
            config: cfg,
            skills: vec![test_skill("my-agent", vec![])],
            project_root: std::path::PathBuf::from("/tmp/test"),
        };

        let outcome = HarnessResolver::resolve_project(&model, &registry);
        assert_eq!(outcome.fatal.len(), 1);
        match &outcome.fatal[0] {
            ResolveError::UnknownHarness { .. } => {}
            e @ ResolveError::MissingCapability { .. } => {
                panic!("expected UnknownHarness, got {e:?}")
            }
        }
        // The valid pair (my-agent x claude) is still resolved despite the other
        // harness being unknown — accumulate-don't-abort applies per pair.
        assert_eq!(outcome.resolved.len(), 1);
    }

    #[test]
    fn resolve_project_skips_capability_mismatch_without_aborting_other_pairs() {
        let registry = test_registry();

        // pi has supports_subagent: false; claude and opencode both support it.
        let cfg = crate::types::ProjectConfig {
            harnesses: vec![
                "claude".to_string(),
                "opencode".to_string(),
                "pi".to_string(),
            ],
            ..Default::default()
        };

        let model = ProjectModel {
            config: cfg,
            skills: vec![test_skill("sub-agent", vec!["subagent".to_string()])],
            project_root: std::path::PathBuf::from("/tmp/test"),
        };

        let outcome = HarnessResolver::resolve_project(&model, &registry);
        assert!(outcome.fatal.is_empty());
        assert_eq!(
            outcome.resolved.len(),
            2,
            "claude and opencode pairs should still resolve"
        );
        assert_eq!(
            outcome.skipped.len(),
            1,
            "only the pi pair should be skipped"
        );
        match &outcome.skipped[0] {
            ResolveError::MissingCapability { harness_name, .. } => {
                assert_eq!(harness_name, "pi");
            }
            e @ ResolveError::UnknownHarness { .. } => {
                panic!("expected MissingCapability, got {e:?}")
            }
        }
    }
}

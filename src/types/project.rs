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

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// Top-level project configuration deserialized from skillprism.yaml.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    /// Harness IDs this project targets (e.g. `claude`, `opencode`).
    #[serde(default)]
    pub harnesses: Vec<String>,

    /// Directory containing skill definitions, relative to project root.
    #[serde(default = "default_skills_dir")]
    pub skills_dir: PathBuf,

    /// Directory containing user harness overrides.
    #[serde(default = "default_harnesses_dir")]
    #[allow(dead_code)]
    pub harnesses_dir: PathBuf,

    /// Optional project name.
    #[allow(dead_code)]
    pub name: Option<String>,
}

fn default_skills_dir() -> PathBuf {
    PathBuf::from("skills")
}

fn default_harnesses_dir() -> PathBuf {
    PathBuf::from("harnesses")
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            harnesses: Vec::new(),
            skills_dir: default_skills_dir(),
            harnesses_dir: default_harnesses_dir(),
            name: None,
        }
    }
}

/// A skill loaded from disk with all its configuration and metadata.
#[derive(Debug, Clone)]
pub struct SkillModel {
    /// Skill name, either from skill.yaml or the directory name.
    pub name: String,
    /// The directory name on disk.
    #[allow(dead_code)]
    pub directory_name: String,
    /// Human-readable description of the skill.
    pub description: String,
    /// Optional semantic version.
    pub version: Option<String>,
    /// Optional license identifier.
    pub license: Option<String>,
    /// Optional compatibility note.
    pub compatibility: Option<String>,
    /// Arbitrary key-value metadata.
    pub metadata: BTreeMap<String, String>,
    /// Comma-separated list of allowed tools.
    pub allowed_tools: Option<String>,
    /// Description of when the skill should be used.
    pub when_to_use: Option<String>,
    /// Hint shown to users for skill arguments.
    pub argument_hint: Option<String>,
    /// List of argument names.
    pub arguments: Option<Vec<String>>,
    /// Whether to suppress default model invocation.
    pub disable_model_invocation: Option<bool>,
    /// Whether the skill is user-invocable.
    pub user_invocable: Option<bool>,
    /// Tools explicitly disallowed for this skill.
    pub disallowed_tools: Option<Vec<String>>,
    /// Override the default model for this skill.
    pub model_override: Option<String>,
    /// Effort level hint for the model.
    pub effort: Option<String>,
    /// Whether the skill uses context forking.
    pub context_fork: bool,
    /// Optional agent identifier for sub-agent invocation.
    pub agent: Option<String>,
    /// Optional lifecycle hooks.
    pub hooks: Option<BTreeMap<String, yaml_serde::Value>>,
    /// Optional file paths that trigger this skill.
    pub activation_paths: Option<Vec<String>>,
    /// Optional shell command to execute.
    pub shell: Option<String>,
    /// Capabilities required from the harness.
    pub required_capabilities: Vec<String>,
    /// Template variables defined in skill.yaml (inherited from parent groups).
    pub variables: BTreeMap<String, yaml_serde::Value>,
    /// Path to the Jinja2 template file.
    pub template_path: PathBuf,
    /// Every direct subdirectory of the skill's own directory (e.g. `references/`,
    /// `scripts/`, or any other name an author uses), copied verbatim alongside it.
    pub asset_dirs: Vec<PathBuf>,
    /// Per-harness overrides from skill.yaml's `harnesses:` block, keyed by harness ID.
    pub harness_overrides: BTreeMap<String, HarnessOverride>,
}

impl SkillModel {
    /// Resolves this skill's variables for rendering against a specific harness: the
    /// top-level `variables:` map with that harness's `harnesses.<id>.variables`
    /// overrides merged in on top (harness wins), per the schema's documented
    /// "merged with top-level variables, harness wins" semantics.
    pub fn variables_for_harness(&self, harness_id: &str) -> BTreeMap<String, yaml_serde::Value> {
        let mut merged = self.variables.clone();
        if let Some(override_) = self.harness_overrides.get(harness_id) {
            for (k, v) in &override_.variables {
                merged.insert(k.clone(), v.clone());
            }
        }
        merged
    }
}

/// Per-harness overrides for a single skill, from skill.yaml's `harnesses:` block.
#[derive(Debug, Clone, Default)]
pub struct HarnessOverride {
    /// Variable overrides merged over top-level `variables`, harness wins.
    pub variables: BTreeMap<String, yaml_serde::Value>,
    /// Macro overrides scoped to this skill only — harness wins over that harness's
    /// own builtin macro of the same name, if any.
    pub macros: BTreeMap<String, String>,
}

/// A group of skills sharing local variables and nested sub-groups.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillGroup {
    /// Variables inherited by all skills in this group.
    pub local_variables: BTreeMap<String, yaml_serde::Value>,
    /// Path to the group's skill.yaml config, if any.
    pub config_path: Option<PathBuf>,
    /// Nested sub-groups within this group.
    pub groups: Vec<Self>,
    /// Skills directly contained in this group.
    pub skills: Vec<SkillModel>,
}

/// Names of `SkillModel` metadata fields exposed as built-in template variables,
/// one-to-one with the fields below `description`. Shared by
/// `engine::context::build_context` (which inserts them into the render context) and
/// `validator::variables::is_builtin` (which exempts them from undefined-variable
/// checks) so the two can't silently drift apart — `engine::context::tests` asserts
/// every name here is actually present in a built context.
pub const SKILL_METADATA_FIELDS: &[&str] = &[
    "version",
    "license",
    "compatibility",
    "metadata",
    "allowed_tools",
    "when_to_use",
    "argument_hint",
    "arguments",
    "disable_model_invocation",
    "user_invocable",
    "disallowed_tools",
    "model_override",
    "effort",
    "context_fork",
    "agent",
    "hooks",
    "activation_paths",
    "shell",
    "required_capabilities",
];

/// The complete loaded model of a skillprism project.
#[derive(Debug, Clone)]
pub struct ProjectModel {
    /// Project-level configuration from skillprism.yaml.
    pub config: ProjectConfig,
    /// All discovered skills in the project.
    pub skills: Vec<SkillModel>,
    /// Absolute path to the project root directory.
    #[allow(dead_code)]
    pub project_root: PathBuf,
}

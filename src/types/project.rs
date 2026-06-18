#![allow(clippy::redundant_pub_crate, dead_code)]

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
    pub harnesses_dir: PathBuf,

    /// Optional project name.
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
    /// Asset directories (references, scripts) to copy alongside the skill.
    pub asset_dirs: Vec<PathBuf>,
}

/// A group of skills sharing local variables and nested sub-groups.
#[derive(Debug, Clone)]
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

/// The complete loaded model of a skillprism project.
#[derive(Debug, Clone)]
pub struct ProjectModel {
    /// Project-level configuration from skillprism.yaml.
    pub config: ProjectConfig,
    /// All discovered skills in the project.
    pub skills: Vec<SkillModel>,
    /// Absolute path to the project root directory.
    pub project_root: PathBuf,
}

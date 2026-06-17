#![allow(clippy::redundant_pub_crate, dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub harnesses: Vec<String>,

    #[serde(default = "default_skills_dir")]
    pub skills_dir: PathBuf,

    #[serde(default = "default_harnesses_dir")]
    pub harnesses_dir: PathBuf,

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

#[derive(Debug, Clone)]
pub struct SkillModel {
    pub name: String,
    pub directory_name: String,
    pub description: String,
    pub version: Option<String>,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub allowed_tools: Option<String>,
    pub when_to_use: Option<String>,
    pub argument_hint: Option<String>,
    pub arguments: Option<Vec<String>>,
    pub disable_model_invocation: Option<bool>,
    pub user_invocable: Option<bool>,
    pub disallowed_tools: Option<Vec<String>>,
    pub model_override: Option<String>,
    pub effort: Option<String>,
    pub context_fork: bool,
    pub agent: Option<String>,
    pub hooks: Option<BTreeMap<String, yaml_serde::Value>>,
    pub activation_paths: Option<Vec<String>>,
    pub shell: Option<String>,
    pub required_capabilities: Vec<String>,
    pub variables: BTreeMap<String, yaml_serde::Value>,
    pub template_path: PathBuf,
    pub asset_dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SkillGroup {
    pub local_variables: BTreeMap<String, yaml_serde::Value>,
    pub config_path: Option<PathBuf>,
    pub groups: Vec<Self>,
    pub skills: Vec<SkillModel>,
}

#[derive(Debug, Clone)]
pub struct ProjectModel {
    pub config: ProjectConfig,
    pub skills: Vec<SkillModel>,
    pub project_root: PathBuf,
}

#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessDefinition {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub capabilities: HarnessCapabilities,
    pub paths: HarnessPaths,
    #[serde(default)]
    pub macros: BTreeMap<String, MacroDef>,
    #[serde(default)]
    pub functions: BTreeMap<String, FunctionDef>,
    #[serde(default)]
    pub sidecars: Vec<SidecarDef>,
    pub manifest: Option<ManifestDef>,
    pub skill_ref_pattern: Option<String>,
    pub discovery: Option<DiscoveryConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessCapabilities {
    pub supports_subagent: bool,
    #[serde(default)]
    pub requires_sidecar: bool,
    #[serde(default)]
    pub requires_manifest: bool,
    pub frontmatter_mode: String,
    #[serde(default = "default_name_max")]
    pub name_max_length: usize,
    #[serde(default = "default_desc_max")]
    pub description_max_length: usize,
    #[serde(default)]
    pub supports_allowed_tools: bool,
    #[serde(default)]
    pub supports_disable_model_invocation: bool,
    #[serde(default)]
    pub supports_user_invocable_flag: bool,
    pub extra_metadata_path: Option<String>,
}

const fn default_name_max() -> usize {
    64
}

const fn default_desc_max() -> usize {
    1024
}

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessPaths {
    pub project_scope_path: String,
    pub user_scope_path: String,
    pub skill_filename: String,
    pub manifest_scope_path: Option<String>,
    pub manifest_filename: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MacroDef {
    Inline(String),
    Function {
        content: String,
        arguments: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct FunctionDef {
    pub description: String,
    pub returns: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidecarDef {
    pub filename: String,
    pub template: String,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestDef {
    pub format: String,
    pub template: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryConfig {
    #[serde(default)]
    pub project: bool,
    #[serde(default)]
    pub user: bool,
    #[serde(default)]
    pub plugin: bool,
}

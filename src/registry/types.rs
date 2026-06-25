// reason: HarnessCapabilities has 8 boolean fields by design; extracting a bools struct
// would add indirection without clarity benefit. Lint kept at module level intentionally.
#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeMap;

use serde::Deserialize;

/// Complete definition of a harness including its capabilities, paths, and templates.
#[derive(Debug, Clone, Deserialize)]
pub struct HarnessDefinition {
    /// Unique identifier for the harness (e.g. "claude", "opencode").
    pub id: String,
    /// Human-readable display name (e.g. "Claude Code").
    pub name: String,
    /// Optional version string.
    pub version: Option<String>,
    /// Capabilities supported by this harness.
    pub capabilities: HarnessCapabilities,
    /// File system paths for skill and manifest output.
    pub paths: HarnessPaths,
    /// Named macros available to templates as `harness.<name>`.
    #[serde(default)]
    pub macros: BTreeMap<String, MacroDef>,
    /// Custom functions exposed to templates.
    #[serde(default)]
    #[allow(dead_code)]
    pub functions: BTreeMap<String, FunctionDef>,
    /// Sidecar file definitions produced alongside skills.
    #[serde(default)]
    pub sidecars: Vec<SidecarDef>,
    /// Optional manifest definition for plugin metadata.
    pub manifest: Option<ManifestDef>,
    /// Pattern for referencing skills in this harness (e.g. "/{name}").
    pub skill_ref_pattern: Option<String>,
    /// Configuration for where the harness discovers skills.
    #[allow(dead_code)]
    pub discovery: Option<DiscoveryConfig>,
}

/// Feature flags and constraints for a harness.
#[derive(Debug, Clone, Deserialize)]
pub struct HarnessCapabilities {
    /// Whether the harness supports sub-agent invocation.
    pub supports_subagent: bool,
    /// Whether the harness requires sidecar files.
    #[serde(default)]
    pub requires_sidecar: bool,
    /// Whether the harness requires a manifest file.
    #[serde(default)]
    pub requires_manifest: bool,
    /// Frontmatter parsing mode ("strict" or "lenient").
    #[allow(dead_code)]
    pub frontmatter_mode: String,
    /// Maximum allowed skill name length.
    #[serde(default = "default_name_max")]
    #[allow(dead_code)]
    pub name_max_length: usize,
    /// Maximum allowed description length.
    #[serde(default = "default_desc_max")]
    #[allow(dead_code)]
    pub description_max_length: usize,
    /// Whether `allowed-tools` is supported in skill config.
    #[serde(default)]
    pub supports_allowed_tools: bool,
    /// Whether `disable-model-invocation` is supported in skill config.
    #[serde(default)]
    pub supports_disable_model_invocation: bool,
    /// Whether `user-invocable` is supported in skill config.
    #[serde(default)]
    pub supports_user_invocable_flag: bool,
    /// Path to an extra metadata file for the harness.
    #[allow(dead_code)]
    pub extra_metadata_path: Option<String>,
}

const fn default_name_max() -> usize {
    64
}

const fn default_desc_max() -> usize {
    1024
}

/// File system paths where skills and manifests are written.
#[derive(Debug, Clone, Deserialize)]
pub struct HarnessPaths {
    /// Directory for project-scoped skill output (e.g. ".claude/skills").
    pub project_scope_path: String,
    /// Directory for user-scoped skill output (e.g. "~/.claude/skills").
    pub user_scope_path: String,
    /// Filename for the rendered skill file (e.g. "SKILL.md").
    pub skill_filename: String,
    /// Directory for manifest output (e.g. ".claude").
    pub manifest_scope_path: Option<String>,
    /// Filename for the manifest file (e.g. "plugin.json").
    pub manifest_filename: Option<String>,
}

/// A harness macro definition: either inline text or a function with arguments.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MacroDef {
    /// An inline string macro.
    Inline(String),
    /// A function-like macro with optional argument list.
    Function {
        /// The macro body.
        content: String,
        /// Optional parameter names.
        #[allow(dead_code)]
        arguments: Option<Vec<String>>,
    },
}

/// Definition of a custom template function exposed by a harness.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionDef {
    /// Human-readable description of the function.
    #[allow(dead_code)]
    pub description: String,
    /// Optional description of the return value.
    #[allow(dead_code)]
    pub returns: Option<String>,
    /// Optional inline template for the function body.
    #[allow(dead_code)]
    pub template: Option<String>,
}

/// Definition of a sidecar file produced alongside the main skill.
#[derive(Debug, Clone, Deserialize)]
pub struct SidecarDef {
    /// Filename for the sidecar output.
    pub filename: String,
    /// Jinja2 template for the sidecar content.
    pub template: String,
    /// Optional subdirectory within the skill output directory.
    pub output_dir: Option<String>,
}

/// Definition of a manifest file produced for the harness.
#[derive(Debug, Clone, Deserialize)]
pub struct ManifestDef {
    /// Format of the manifest (e.g. "json").
    #[allow(dead_code)]
    pub format: String,
    /// Jinja2 template for the manifest content.
    pub template: String,
}

/// Configuration for where a harness discovers skill definitions.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoveryConfig {
    /// Discover skills from the project directory.
    #[serde(default)]
    #[allow(dead_code)]
    pub project: bool,
    /// Discover skills from the user's home directory.
    #[serde(default)]
    #[allow(dead_code)]
    pub user: bool,
    /// Discover skills from plugin directories.
    #[serde(default)]
    #[allow(dead_code)]
    pub plugin: bool,
}

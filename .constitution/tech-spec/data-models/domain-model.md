// In-Memory Domain Data Models
//
// This file documents the core Rust types that flow through the pipeline.
// Types are grouped by the logical container that produces them.
// All types implement Debug + Clone + PartialEq unless noted.

// ─── Project Loader ──────────────────────────────────────────────────────────

/// A directory group with an optional skill.yaml and child skills/groups.
/// Represents the directory hierarchy before variable flattening.
struct SkillGroup {
    /// Variables declared in this group's skill.yaml (if any)
    local_variables: BTreeMap<String, Value>,
    /// Path to this group's skill.yaml (for error reporting)
    config_path: Option<PathBuf>,
    /// Child groups (subdirectories with their own skill.yaml)
    groups: Vec<SkillGroup>,
    /// Leaf skills in this directory
    skills: Vec<SkillModel>,
}

/// Top-level project configuration parsed from skillprism.yaml
struct ProjectConfig {
    /// Project-level skills directory (default: "skills")
    skills_dir: PathBuf,
    /// Selected harness IDs (e.g., ["claude", "codex", "opencode"])
    harnesses: Vec<String>,
    /// Project name (used for manifest generation)
    name: Option<String>,
    /// User harness overrides directory (default: "harnesses")
    harnesses_dir: PathBuf,
}

/// A single skill after configuration loading and variable resolution
struct SkillModel {
    /// Canonical skill name (from skill.yaml or directory name)
    name: String,
    /// Description (from skill.yaml)
    description: Option<String>,
    /// Version (from skill.yaml)
    version: Option<String>,
    /// Resolved variables (group + skill merged, skill wins)
    variables: BTreeMap<String, Value>,
    /// Path to the SKILL.md.j2 template file
    template_path: PathBuf,
    /// Paths to shared asset directories (references/, scripts/)
    asset_dirs: Vec<PathBuf>,
    /// Output files for each harness (populated by engine)
    rendered: BTreeMap<String, HarnessOutput>,
}

/// The resolved project model — output of ProjectLoader, input to Validator
struct ProjectModel {
    config: ProjectConfig,
    skills: Vec<SkillModel>,
    user_harness_defs: Vec<HarnessDefinition>,
    project_root: PathBuf,
}

// ─── Harness Registry ────────────────────────────────────────────────────────

/// A fully resolved harness definition (built-in + user overrides applied)
struct HarnessDefinition {
    /// Canonical identifier
    id: String,
    /// Human-readable name
    name: String,
    /// Capability flags
    capabilities: HarnessCapabilities,
    /// Installation and output paths
    paths: HarnessPaths,
    /// Named content blocks (macros) keyed by name
    macros: BTreeMap<String, Macro>,
    /// Custom template functions
    functions: BTreeMap<String, FunctionDef>,
    /// Optional sidecar definition
    sidecar: Option<SidecarDef>,
    /// Optional manifest template
    manifest: Option<ManifestDef>,
    /// Pattern for skill references (e.g., "/{name}")
    skill_ref_pattern: Option<String>,
}

struct HarnessCapabilities {
    supports_subagent: bool,
    requires_sidecar: bool,
    requires_manifest: bool,
    supports_frontmatter_extensions: bool,
}

struct HarnessPaths {
    skill_dir: String,
    skill_filename: String,
    manifest_dir: Option<String>,
    manifest_filename: Option<String>,
    project_root: Option<String>,
    user_root: Option<String>,
}

enum Macro {
    /// Simple string content block
    Inline(String),
    /// Macro with function-like arguments
    Function { content: String, arguments: Vec<String> },
}

struct FunctionDef {
    description: String,
    returns: Option<String>,
    template: Option<String>,
}

struct SidecarDef {
    filename: String,
    template: String,
    output_dir: Option<String>,
}

struct ManifestDef {
    template: String,
}

// ─── Validator ───────────────────────────────────────────────────────────────

/// Structured error for a single skill
#[derive(miette::Diagnostic, thiserror::Error)]
struct SkillError {
    /// The skill name
    skill: String,
    /// Source file path
    #[source_code]
    src: SourceCode,
    /// Offending line
    #[label]
    span: SourceSpan,
    /// Human-readable message
    #[help]
    message: String,
}

// ─── Template Engine ─────────────────────────────────────────────────────────

/// Rendered output for a single harness
struct HarnessOutput {
    /// The rendered SKILL.md content
    skill_content: String,
    /// Sidecar file content (if harness.requires_sidecar)
    sidecar: Option<SidecarOutput>,
    /// Manifest entry for this skill (if harness.requires_manifest)
    manifest_entry: Option<String>,
}

struct SidecarOutput {
    filename: String,
    content: String,
    output_dir: Option<String>,
}

// ─── Output Router ───────────────────────────────────────────────────────────

/// Resolved target path for a single file write
struct ResolvedTarget {
    /// Absolute output path (e.g., /home/user/project/.claude/skills/my-skill/SKILL.md)
    path: PathBuf,
    /// Content to write
    content: String,
    /// Whether a file already exists at this path
    exists: bool,
}

enum TargetScope {
    Project,
    User,
    Dist,
}

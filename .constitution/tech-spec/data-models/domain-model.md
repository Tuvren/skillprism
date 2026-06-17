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
/// All fields use #[serde(default)] with sensible defaults:
/// - skills_dir = PathBuf::from("skills")
/// - harnesses_dir = PathBuf::from("harnesses")
struct ProjectConfig {
    /// Selected harness IDs (e.g., ["claude", "codex", "opencode"])
    harnesses: Vec<String>,
    /// Project-level skills directory (default: "skills")
    skills_dir: PathBuf,
    /// User harness overrides directory (default: "harnesses")
    harnesses_dir: PathBuf,
    /// Project name (used for manifest generation)
    name: Option<String>,
}

/// A single skill after configuration loading and variable resolution
struct SkillModel {
    /// Canonical skill name (from skill.yaml or directory name)
    /// Constraints: 1-64 chars (or per-harness max), lowercase letters/digits/hyphens only,
    /// no leading/trailing/consecutive hyphens, must match parent directory name
    name: String,
    /// Parent directory name (for name-vs-directory validation per Agent Skills spec)
    directory_name: String,
    /// Description (from skill.yaml) — what the skill does and when to use it
    description: String,
    /// Version (from skill.yaml, used internally; not part of output SKILL.md frontmatter)
    version: Option<String>,
    /// License (from skill.yaml, maps to SKILL.md frontmatter)
    license: Option<String>,
    /// Environment requirements (from skill.yaml, maps to SKILL.md frontmatter)
    compatibility: Option<String>,
    /// Arbitrary key-value metadata (from skill.yaml, maps to SKILL.md frontmatter)
    metadata: BTreeMap<String, String>,
    /// Pre-approved tools (from skill.yaml, maps to SKILL.md frontmatter, experimental)
    allowed_tools: Option<String>,
    /// Additional trigger phrases (Claude Code: when_to_use)
    when_to_use: Option<String>,
    /// Autocomplete hint (Claude Code: argument-hint)
    argument_hint: Option<String>,
    /// Positional arguments for $name substitution (Claude Code: arguments)
    arguments: Option<Vec<String>>,
    /// Prevent automatic loading (Claude Code, Factory, Pi: disable-model-invocation)
    disable_model_invocation: Option<bool>,
    /// Hide from / menu (Claude Code, Factory: user-invocable)
    user_invocable: Option<bool>,
    /// Tools removed from agent's pool while skill active (Claude Code: disallowed-tools)
    disallowed_tools: Option<Vec<String>>,
    /// Model override (Claude Code: model)
    model_override: Option<String>,
    /// Effort level (Claude Code: effort)
    effort: Option<String>,
    /// Fork context (Claude Code: context)
    context_fork: bool,
    /// Subagent type for fork context (Claude Code: agent)
    agent: Option<String>,
    /// Lifecycle hooks (Claude Code: hooks)
    hooks: Option<BTreeMap<String, Value>>,
    /// Glob patterns for auto-activation (Claude Code: paths)
    activation_paths: Option<Vec<String>>,
    /// Shell for command injection (Claude Code: shell)
    shell: Option<String>,
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
    /// Version of this harness definition (SemVer)
    version: Option<String>,
    /// Capability flags
    capabilities: HarnessCapabilities,
    /// Installation and output paths
    paths: HarnessPaths,
    /// Named content blocks (macros) keyed by name
    macros: BTreeMap<String, Macro>,
    /// Custom template functions
    functions: BTreeMap<String, FunctionDef>,
    /// Sidecar file templates generated alongside each skill (may be multiple)
    sidecars: Vec<SidecarDef>,
    /// Optional manifest template
    manifest: Option<ManifestDef>,
    /// Pattern for skill references (e.g., "/{name}")
    skill_ref_pattern: Option<String>,
    /// Discovery path support for this harness
    discovery: Option<DiscoveryConfig>,
}

/// Which discovery scopes a harness supports
struct DiscoveryConfig {
    project: bool,
    user: bool,
    plugin: bool,
}

/// How the harness validates SKILL.md frontmatter fields
enum FrontmatterMode {
    /// Spec fields only, ignores unknown fields
    /// Platforms: OpenCode, Codex, Pi, Factory
    Lenient,
    /// Spec + platform-specific extended fields
    /// Platforms: Claude Code only
    Extended,
}

struct HarnessCapabilities {
    supports_subagent: bool,
    requires_sidecar: bool,
    requires_manifest: bool,
    frontmatter_mode: FrontmatterMode,
    /// Maximum length for skill name field (spec default: 64, Codex: 100)
    name_max_length: usize,
    /// Maximum length for skill description field (spec default: 1024, Codex: 500)
    description_max_length: usize,
    /// Whether the harness supports the experimental `allowed-tools` field
    supports_allowed_tools: bool,
    /// Whether the harness recognizes `disable-model-invocation` (hide from auto-load)
    supports_disable_model_invocation: bool,
    /// Whether the harness recognizes `user-invocable` (hide from / menu)
    supports_user_invocable_flag: bool,
    /// Optional per-skill metadata file path relative to the skill dir
    /// e.g. "agents/openai.yaml" for Codex
    extra_metadata_path: Option<String>,
}

/// Output path configuration for a single harness
struct HarnessPaths {
    /// Project-scoped output directory relative to project root
    /// e.g. ".claude/skills" (Claude), ".agents/skills" (Codex),
    ///      ".opencode/skills" (OpenCode), ".pi/skills" (Pi),
    ///      ".factory/skills" (Factory)
    project_scope_path: String,
    /// User-scoped output directory relative to home
    /// e.g. "~/.claude/skills" (Claude), "~/.codex/skills" (Codex),
    ///      "~/.config/opencode/skills" (OpenCode), "~/.pi/agent/skills" (Pi),
    ///      "~/.factory/skills" (Factory)
    user_scope_path: String,
    /// Skill filename (e.g. "SKILL.md", "agent.md")
    skill_filename: String,
    /// Directory for plugin manifest relative to project/user scope root (nullable)
    /// e.g. ".claude" for Claude plugin manifest
    manifest_scope_path: Option<String>,
    /// Plugin manifest filename (nullable)
    /// e.g. "plugin.json" for Claude, "marketplace.json" for Codex
    manifest_filename: Option<String>,
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

/// Plugin manifest format identifier
enum ManifestFormat {
    /// Claude Code plugin manifest (plugin.json)
    ClaudePlugin,
    /// Codex marketplace manifest (marketplace.json or personal-marketplace.json)
    CodexPlugin,
}

struct ManifestDef {
    /// Which platform format this manifest targets
    format: ManifestFormat,
    /// Template string rendered with skill variables
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
    /// Sidecar files generated alongside each skill (zero or more)
    sidecars: Vec<SidecarOutput>,
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

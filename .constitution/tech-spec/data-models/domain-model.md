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
    name: String,
    /// Parent directory name (for name-vs-directory validation per Agent Skills spec)
    directory_name: String,
    /// Description (from skill.yaml)
    description: String,
    /// Version (from skill.yaml, used internally)
    version: Option<String>,
    /// License (from skill.yaml, maps to SKILL.md frontmatter)
    license: Option<String>,
    /// Environment requirements (from skill.yaml)
    compatibility: Option<String>,
    /// Arbitrary key-value metadata (from skill.yaml)
    metadata: BTreeMap<String, String>,
    /// Pre-approved tools (from skill.yaml, experimental)
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
    /// Path to the template file (SKILL.md or SKILL.md.j2; not both)
    template_path: PathBuf,
    /// Every direct subdirectory of the skill's own directory (e.g. references/,
    /// scripts/, or any other name), copied verbatim alongside it — dot-directories
    /// (.venv/, .git/, ...) are excluded
    asset_dirs: Vec<PathBuf>,
    /// Required harness capabilities this skill depends on
    required_capabilities: Vec<String>,
    /// Per-harness overrides from skill.yaml's `harnesses:` block, keyed by harness ID
    harness_overrides: BTreeMap<String, HarnessOverride>,
}

/// A single harness's overrides from skill.yaml's `harnesses.<id>` block.
struct HarnessOverride {
    /// Variable overrides merged over top-level `variables`, harness wins
    variables: BTreeMap<String, Value>,
    /// Macro overrides scoped to this skill only — wins over that harness's own
    /// builtin macro of the same name, if any
    macros: BTreeMap<String, String>,
}

/// The resolved project model — output of ProjectLoader
struct ProjectModel {
    config: ProjectConfig,
    skills: Vec<SkillModel>,
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

/// Frontmatter validation mode (stored as String in code for simpler deserialization)
/// Expected values: "lenient" (default), "extended"
/// lenient: Only spec fields, ignores unknown fields (OpenCode, Codex, Pi, Factory)
/// extended: Spec + platform-specific extended fields (Claude Code)
type FrontmatterMode = String;

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

/// Plugin manifest format identifier (stored as String for simpler deserialization)
/// Expected: "claude-plugin" (plugin.json), "codex-plugin" (marketplace.json)
type ManifestFormat = String;

struct ManifestDef {
    /// Which platform format this manifest targets
    format: ManifestFormat,
    /// Template string rendered with skill variables
    template: String,
}

// ─── Resolver ────────────────────────────────────────────────────────────────

/// A paired skill + harness definition — the carrier type
/// flowing from Resolver through Validator to Engine and Router.
struct ResolvedPair {
    skill: SkillModel,
    harness: HarnessDefinition,
}

/// Structured error during harness resolution
#[derive(thiserror::Error)]
enum ResolveError {
    /// No harness found matching the requested name
    UnknownHarness {
        skill_name: String,
        harness_name: String,
        available: Vec<String>,
    },
    /// A required capability is missing from the harness
    MissingCapability {
        skill_name: String,
        harness_name: String,
        capability: String,
    },
}

// ─── Validator ───────────────────────────────────────────────────────────────

/// Validated outcome: valid pairs continue, errors collected for reporting
struct ValidationOutcome {
    /// Pairs that passed all validation checks
    valid: Vec<ResolvedPair>,
    /// Errors collected across all skills (collect-all-errors pattern)
    errors: Vec<ValidationError>,
}

/// Structured error during template validation
#[derive(thiserror::Error)]
enum ValidationError {
    /// MiniJinja syntax parse failure
    SyntaxError { skill: String, harness: String, detail: String },
    /// Reference to an undefined variable
    UndefinedVariable { skill: String, harness: String, variable: String },
    /// Reference to an undefined harness macro
    UndefinedMacro { skill: String, harness: String, macro_name: String },
    /// Failed to read template file from disk
    TemplateRead { skill: String, harness: String, path: String, detail: String },
}

// ─── Template Engine ─────────────────────────────────────────────────────────

/// Rendered output for a single harness
struct HarnessOutput {
    /// The rendered SKILL.md content
    skill_content: String,
    /// Sidecar files generated alongside each skill (zero or more)
    sidecars: Vec<SidecarOutput>,
    /// Manifest entry for this skill (if harness has manifest defined)
    manifest_entry: Option<String>,
}

struct SidecarOutput {
    filename: String,
    content: String,
    output_dir: Option<String>,
}

/// Errors during template rendering
#[derive(thiserror::Error)]
enum EngineError {
    /// Failed to read template file
    TemplateRead { skill: String, harness: String, path: String },
    /// MiniJinja render failure
    RenderError { skill: String, harness: String, detail: String },
}

// ─── Output Router ───────────────────────────────────────────────────────────

enum TargetScope {
    Project,
    User,
    Dist,
}

/// Result of a successful write operation
struct WrittenFiles {
    /// Absolute path of the written skill file
    skill_path: PathBuf,
    /// Absolute paths of any sidecar files written
    sidecar_paths: Vec<PathBuf>,
    /// Absolute path of the written manifest file (if the harness defines one and an entry was generated)
    manifest_path: Option<PathBuf>,
}

/// Errors during file routing and writing
#[derive(thiserror::Error)]
enum RouterError {
    /// Failed to write a file to disk
    WriteError { skill: String, harness: String, path: String, detail: String },
    /// Failed to copy asset directories
    AssetCopyError { skill: String, harness: String, path: String, detail: String },
}

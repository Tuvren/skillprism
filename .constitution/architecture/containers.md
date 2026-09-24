# Logical Containers

## Container Diagram

```mermaid
C4Container
  title Container Diagram — skillprism

  Person(solo, "Skill Author (Solo)", "Runs build/validate/init from CLI")
  Person(lead, "Team Lead", "Runs build in CI, manages shared skills")

  System_Boundary(skillprism, "skillprism Binary") {
    Container(cli, "CLI Entrypoint", "CLI arg parser", "Parse args, dispatch to build/validate/init pipeline")
    Container(loader, "Project Loader", "Library", "Discover skill hierarchy, load YAML configs, resolve group-level variables")
    Container(registry, "Harness Registry", "Library", "Manage built-in and user-override harness definitions")
    Container(resolver, "Harness Resolver", "Library", "Pair each skill with its resolved harness definition, check capability compatibility")
    Container(validator, "Validator", "Library", "Batch-check all resolved skill-harness pairs for syntax errors, missing macros, undefined variables")
    Container(engine, "Template Engine", "Library", "Render MiniJinja templates with resolved variables and macros")
    Container(router, "Output Router", "Library", "Resolve target path (project/user/dist), write with atomic safety")
    Container(scaffolder, "Scaffolder", "Library", "Generate project or skill scaffolding")
    Container(distribution, "Distribution CLI", "Library", "Fetch, install, list, remove, and update skills from remote/local sources (add/list/remove/update)")
    Container(state, "State Layer", "Library", "Track installed skills system-wide with atomic, schema-versioned records")
  }

  System_Ext(fs, "Filesystem", "Project files, agent install directories")
  System_Ext(source, "Source host", "Remote skill repositories")

  Rel(solo, cli, "Invokes via shell", "args, flags")
  Rel(lead, cli, "Invokes via shell/CI", "args, flags")

  Rel(cli, loader, "Dispatches build/validate", "in-process")
  Rel(cli, scaffolder, "Dispatches init", "in-process")

  Rel(loader, fs, "Reads skillprism.yaml, skill.yaml, and harness overrides", "file-handoff")

  Rel(registry, fs, "Reads optional user harness overrides", "file-handoff")

  Rel(resolver, loader, "Pairs skills from the project model", "in-process")
  Rel(resolver, registry, "Resolves each harness name", "in-process")

  Rel(validator, resolver, "Validates resolved pairs", "in-process")
  Rel(validator, fs, "Reads templates to check syntax", "file-handoff")

  Rel(engine, validator, "Renders pairs the validator accepted", "in-process")
  Rel(engine, fs, "Reads templates to render", "file-handoff")

  Rel(router, engine, "Routes rendered output", "in-process")
  Rel(router, fs, "Writes skill files, sidecars, and manifests atomically", "file-handoff")

  Rel(scaffolder, fs, "Creates scaffolded project files", "file-handoff")

  Rel(cli, distribution, "Dispatches add, list, remove, and update", "in-process")
  Rel(distribution, loader, "Reuses discovery and template helpers", "in-process")
  Rel(distribution, registry, "Resolves harnesses for an install", "in-process")
  Rel(distribution, resolver, "Pairs a fetched skill with harness definitions", "in-process")
  Rel(distribution, validator, "Validates before writing. Fail closed", "in-process")
  Rel(distribution, engine, "Renders skillprism-format skills", "in-process")
  Rel(distribution, router, "Writes rendered output", "in-process")
  Rel(distribution, state, "Records and queries installs", "in-process")
  Rel(distribution, fs, "Copies local sources and writes installed files", "file-handoff")
  Rel(distribution, source, "Shallow clone and ls-remote. The tech-spec chose the git binary", "other")
  Rel(state, fs, "Reads and writes installed.yaml atomically", "file-handoff")
```

## Container Responsibilities

### CLI Entrypoint

| Field | Value |
| :--- | :--- |
| **Logical type** | CLI boundary |
| **Responsibility** | Parse command-line arguments (subcommand, flags, paths), validate flag combinations, dispatch to the correct pipeline handler (build, validate, init) |
| **Inputs** | Raw CLI args (`skillprism build --target user`, `skillprism validate`, `skillprism init`, etc.) |
| **Outputs** | Structured dispatch to build pipeline, validate pipeline, or scaffolder |
| **Depends on** | Nothing (entry point) |

### Project Loader

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Walk the project directory tree starting from the project root. Discover and parse `skillprism.yaml`, traverse skill directories, load `skill.yaml` files per directory, resolve group-level variable inheritance (parent → child merge, child wins), and discover user harness overrides under `harnesses/` |
| **Inputs** | Project root path |
| **Outputs** | Resolved project model: list of skills (each with its resolved variables, template path, asset paths), list of user harness definitions |
| **Depends on** | Filesystem |

### Harness Registry

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Maintain the set of built-in harness definitions (compiled into the binary). Accept user override harnesses (same name as built-in → fields merged or replaced) and custom harnesses (new name → added to registry) from the project loader. Resolve a harness definition by name to its full definition |
| **Inputs** | Harness name, optional user override definitions |
| **Outputs** | Resolved `HarnessDefinition` (built-in + user overrides applied) |
| **Depends on** | Compiled-in harness data, Project Loader (for user overrides) |

### Harness Resolver

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | For every skill in the project model, match it to the harness definition referenced in the project config. Check that each skill's `required_capabilities` are satisfied by the harness. Produce resolved pairs (skill + harness definition) for downstream stages. Collect all resolution errors across all skills before returning. |
| **Inputs** | Project model with skills and configured harness names, Harness Registry |
| **Outputs** | List of `ResolvedPair` (skill + harness), or list of `ResolveError` |
| **Depends on** | Project Loader, Harness Registry |

### Validator

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | For every resolved skill-harness pair: read the template file and check MiniJinja syntax by attempting to parse it, use MiniJinja's `undeclared_variables()` to find undefined variable references, scan template text for `harness.<macro_name>` refs and verify each resolves against the harness definition. Collect all errors across all pairs. Return valid pairs alongside errors (collect-all-errors pattern). |
| **Inputs** | List of `ResolvedPair` from Resolver |
| **Outputs** | `ValidationOutcome` — list of valid pairs + list of `ValidationError` |
| **Depends on** | Harness Resolver, Filesystem (template reads) |

### Template Engine

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | For a resolved skill-harness pair: read the template, build a MiniJinja context with skill variables (name, description, custom variables) and the `harness` object (id, name, version, macros as strings), register custom helpers (`skill_ref`), and render skill content, sidecars, and manifest entry. |
| **Inputs** | `ResolvedPair` (skill + harness) |
| **Outputs** | `HarnessOutput` (skill_content, sidecars, manifest_entry) or `EngineError` |
| **Depends on** | Harness Resolver, Filesystem (template reads), MiniJinja runtime |

### Output Router

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Resolve the target output path for a resolved skill-harness pair based on target scope (project paths vs user home paths vs `dist/`) using the harness definition's installation path table. Write the rendered `SKILL.md`, sidecar files, and manifest entries. Copy shared asset directories (references/, scripts/). Perform atomic writes (temp `.tmp` file → `rename`). Create parent directories as needed. |
| **Inputs** | `ResolvedPair`, `HarnessOutput`, `TargetScope`, project root path |
| **Outputs** | `WrittenFiles` (skill_path, sidecar_paths) or `RouterError` |
| **Depends on** | Harness Resolver (for paths), Filesystem |

### Scaffolder

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Generate a new skillprism project directory (P1: SC-1) or scaffold a single skill within an existing project (P1: SC-2). Create `skillprism.yaml`, sample skill template, `harnesses/` directory placeholder. |
| **Inputs** | Scaffold type, target path, project name |
| **Outputs** | Created directory tree and files on disk |
| **Depends on** | Filesystem |

### Distribution CLI

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Implement the distribution commands (Epic I, DIST-I001–I010): `add` (parse source, fetch via the git auth chain or local copy, detect skillprism vs plain format, render/copy per harness, record state), `list`, `remove`, and `update` (ls-remote no-op check, per-file SHA-256 change detection, `--diff`). Reuse the build-time containers (loader discovery/template helpers, registry, resolver, validator, engine, router) rather than re-implementing them; validate every resolved pair before writing (fail-closed). Harden untrusted-source handling: credential redaction, symlink-escape rejection, path-traversal guards. |
| **Inputs** | Source string, scope/harness selection, command flags; the built-in harness registry |
| **Outputs** | Rendered/copied skill files per harness, `InstalledSkill` state records, or a typed `CommandError`/`miette::Report` |
| **Depends on** | Project Loader, Harness Registry, Harness Resolver, Validator, Template Engine, Output Router, State Layer, Filesystem, git/`gh` (subprocess) |

### State Layer

| Field | Value |
| :--- | :--- |
| **Logical type** | Library boundary |
| **Responsibility** | Track installed skills system-wide (DIST-I001). Read/write `~/.config/skillprism/installed.yaml` (XDG-resolved, `~/.config` fallback) with `0o700` dir / `0o600` file modes, a schema-versioned document (`version: 1`), `(name, scope)`-keyed records sorted for merge-friendly diffs, per-file SHA-256 hashes, and atomic temp-file-plus-rename writes. Single-writer model (no concurrent-`add` locking in v1, documented). |
| **Inputs** | `InstalledSkill` records (upsert/remove), scope/name queries |
| **Outputs** | Persisted `installed.yaml`, in-memory record set, or `StateError` |
| **Depends on** | Filesystem |

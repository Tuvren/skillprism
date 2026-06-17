# Logical Containers

**Version:** v0.2.0

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
  }

  System_Ext(fs, "Filesystem", "Project files, agent install directories")

  Rel(solo, cli, "Invokes via shell", "args, flags")
  Rel(lead, cli, "Invokes via shell/CI", "args, flags")

  Rel(cli, loader, "Dispatches build/validate", "project root path")
  Rel(cli, scaffolder, "Dispatches init", "scaffold type, path")

  Rel(loader, fs, "Reads", "skillprism.yaml, skill.yaml, harnesses/*.yaml")

  Rel(registry, fs, "Reads (optional)", "user harness overrides")

  Rel(resolver, loader, "Pairs skills with", "project model")
  Rel(resolver, registry, "Resolves against", "harness registry")

  Rel(validator, resolver, "Validates", "resolved skill-harness pairs")
  Rel(validator, fs, "Reads", "template files for validation")

  Rel(engine, validator, "Renders validated", "skill-harness pairs")
  Rel(engine, fs, "Reads", "template files for rendering")

  Rel(router, engine, "Routes rendered output", "harness output per pair")
  Rel(router, fs, "Writes", "skill files, sidecars, manifests atomically")

  Rel(scaffolder, fs, "Creates", "project files")
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

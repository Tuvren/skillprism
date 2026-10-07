# Changelog

## Unreleased

## v0.3.0 — 2026-10-07

### Added

- **Editor validation schemas** — Publish versioned JSON Schemas for project configuration, skill metadata, and harness definitions, add schema modelines to scaffolds and examples, and link the schemas from the documentation. Addresses [#32](https://github.com/tuvren/skillprism/issues/32).

### Fixed

- **JSON manifest rendering** — Escape every interpolated string in the built-in Claude and Codex manifests with `tojson`, and reject invalid aggregated JSON before writing or diffing with harness, path, and parser diagnostics. Fixes [#34](https://github.com/tuvren/skillprism/issues/34).
- **YAML frontmatter rendering** — Add the `yaml_str` filter to quote and escape template values, and reject invalid rendered frontmatter during build with field and location diagnostics. Fixes [#34](https://github.com/tuvren/skillprism/issues/34).
- **Configuration diagnostics** — Distinguish YAML syntax errors from schema errors, render graphical reports with help and source labels, and remove the rejected `name` field from the quickstart project configuration. Fixes [#31](https://github.com/tuvren/skillprism/issues/31).
- **`init project` default harnesses** — Without `-H`, non-interactive runs apply the documented `claude`, `opencode` default and print how to choose other harnesses. The prompt appears only when stdin and stdout are both a terminal. Fixes [#33](https://github.com/tuvren/skillprism/issues/33).
- **`skill_ref` template helper** — `skillprism validate` and `skillprism build` no longer reject templates that call the registered `skill_ref` helper as an `Undefined template variable`, and `skill_ref(name)` now renders the current harness's `skill_ref_pattern` (falling back to `/{name}`) instead of hardcoding `/{name}`.
- **MiniJinja built-in global functions** — `validate` no longer rejects MiniJinja's built-in global functions (`range`, `dict`, `cycler`, `joiner`, `lipsum`, `debug`, `namespace`) as undefined variables.

### Changed

- **Custom manifest validation (breaking)** — Hand-quoted custom manifest templates that build today fail when values contain quotes, backslashes, or newlines; migrate to the `tojson` filter without surrounding quotes and drop the unsupported `format: json` key. Existing built-in manifests whose values contain `<`, `>`, `&`, or `'` show a byte-level diff on the first rebuild because `tojson` escapes these characters, with no semantic change.
- **Rendered frontmatter validation (breaking)** — `build`, `add`, and `update` now fail when rendered `SKILL.md` frontmatter isn't a closed YAML mapping, doesn't parse (for example an unquoted value containing `: `), or has a `name` or `description` that parses as a non-string (`null`/`~`, numbers such as `123`, `true`/`false`, lists or maps; `yes` stays a string under YAML 1.2); migrate templates to the `yaml_str` filter. A rendered file whose first line is `---` must have a closing fence; a body that starts with a `---` horizontal rule needs frontmatter first. Empty frontmatter also fails.
- **`init project` overwrite protection** — Project initialization refuses a target directory containing `skillprism.yaml` with a usage error, preserving the existing project files.
- **Reserved helper function names** — A skill variable named after a skillprism helper function (currently `skill_ref`) is now rejected by `validate` as a reserved name because it shadowed the helper.

## v0.2.0 — 2026-07-26

### Dual Product Model & Breaking Changes

- **Compiler vs Package Manager separation (`build` compile-only)** — `skillprism build` is now compile-only into `dist/<harness-id>/<skill-name>/` and never modifies live agent directories (`.claude/skills/`, `~/.claude/skills/`, etc.). Removed `--target` flag from `build`.
- **Package Manager commands (`add`, `list`, `remove`, `update`)** — Live agent skill management is handled exclusively by `skillprism add`, `list` (alias `ls`), `remove` (alias `rm`), and `update` (alias `up`).
- **Interactive Scope Selection** — Running `skillprism add` without `--target` interactively prompts to select between `project` (`.claude/skills/`, etc.) and `user` (`~/.claude/skills/`, etc.) scopes.
- **Skill-level `overrides:` block rename** — Renamed `harnesses:` block in `skill.yaml` to `overrides:` (e.g. `overrides.claude.variables`). Loading a `skill.yaml` with the legacy `harnesses:` key fails with a clear migration error.
- **`skillprism: '1'` Schema Requirement** — `skill.yaml` must specify `skillprism: '1'` to be recognized as a skillprism-format skill. Bare `SKILL.md` files without `skill.yaml` are auto-detected as plain-format.
- **Schema Honesty & Strict Validation** — Enforced `#[serde(deny_unknown_fields)]` across project, skill, and harness configuration files. Stripped dead schema fields (`harnesses_dir` and `name` from `skillprism.yaml`, `discovery` and `functions` from harness definitions).
- **JSON State Storage** — Installed skills are tracked in atomic JSON manifest files (`.skillprism/state.json` for project scope, `~/.config/skillprism/state.json` for user scope).
- **Custom Harnesses On-Demand** — Removed empty `harnesses/` directory from default `init project` scaffolding. Custom harness definitions are advanced and created on demand via `skillprism init harness <name>`.
- **CLI-only Crate Metadata** — Removed docs.rs library claims from Cargo metadata; skillprism is a binary CLI.

### Migration Guide (v0.1.x → v0.2.0)

| Workflow / Goal | Legacy Command (v0.1.x) | Current Command (v0.2.0) | Notes |
|---|---|---|---|
| **Author / Compile** | `skillprism build` (wrote to `.claude/`) | `skillprism build` | Writes exclusively to `dist/<harness>/` |
| **Preview Dist** | `skillprism build --target dist` | `skillprism build` | `build` is now compile-only into `dist/` |
| **Filter Harnesses** | `skillprism build` (all configured) | `skillprism build -H claude,opencode` | Use `-H` / `--harness` |
| **Install Live (Project)** | `skillprism build --target project` | `skillprism add ./ --target project` | Use `add` for live agent paths |
| **Install Live (User)** | `skillprism build --target user` | `skillprism add ./ --target user` | Interactive prompt if `--target` omitted |
| **Install Remote Skill** | `skillprism add owner/repo` | `skillprism add owner/repo` | Auto-detects format & prompts scope |
| **List Installed Skills** | `skillprism list` | `skillprism list` (or `skillprism ls`) | Filter by `--target` / `-H` |
| **Remove Skills** | `skillprism remove <name>` | `skillprism remove <name>` (or `rm`) | Supports `--all`, `--all-scopes` |
| **Update Skills** | `skillprism update` | `skillprism update` (or `up`) | Performs lightweight `git ls-remote` check |

---

## v0.1.3 — 2026-07-22

- **Multi-project state store isolation** — Fixed identity collision in state tracking where project-scoped skills were keyed strictly by `(name, scope)`. Skills are now keyed by `(name, scope, project_root)` so the same skill installed across multiple project directories coexists without overwriting state or cross-contaminating `update` calls.
- **Non-interactive / CI safety** — `build` and `remove` now handle non-TTY environments cleanly: `resolve_overwrite` exits with code 2 when existing files would be overwritten without `--force`, preventing silent build skips in CI. `remove` checks `is_terminal()` before prompting for confirmation.
- **Overwrite prompt option** — Added `[o]verwrite all` option to interactive overwrite prompts during build/install operations.
- **Template context aliases** — Exposed `model`, `context`, and `paths` directly in MiniJinja template context alongside `model_override`, `context_fork`, and `activation_paths`, eliminating variable name friction.
- **Explicit project root searching** — Added `find_project_root_from` helper allowing root discovery from explicit starting directories.

## v0.1.0 — Release Readiness

### Epic H — Release Readiness

- **Shell completions** — New `completions` subcommand generates shell completion scripts for Bash, Fish, and Zsh.
- **`--dry-run` alias** — `build --dry-run` is now a visible alias for `build --diff`, showing a diff preview without writing files.
- **Man page** — A `skillprism.1` man page can be generated via `cargo run -- __generate_man > skillprism.1`.
- **CLI help polish** — All subcommands and flags now have consistent, professional descriptions in `--help` output.
- **Release CI** — Tag-based GitHub Actions workflow builds and attaches binaries for Linux (x86_64) and macOS (x86_64 + ARM) to GitHub Releases.
- **`.gitignore` polish** — Added `.direnv/`, `dist/`, and `*.tmp` entries.
- **`cargo publish` readiness** — Cargo metadata verified and ready for crates.io publication.

### Epic G — Code Quality

- Removed dead code across the codebase (unused variants, functions, and modules).
- Replaced all module-level `#![allow(...)]` attributes with targeted per-item annotations.
- No ambient `#[allow(dead_code)]` remains without justification.

### Epic F — Testing & CI

- Integration test suite with end-to-end CLI tests covering the full pipeline.
- Fixture project with skills x harnesses for reproducible testing.
- GitHub Actions CI workflow running build, test, clippy, and format checks on every push and PR.
- Pre-commit hooks for `cargo fmt` and `cargo clippy` via devenv.

### Epic E — Scaffolding Enhancements

- `init project` now accepts `--harnesses` to specify which harnesses to scaffold for. *(Note: `--harnesses` is supported on `init project`; `init skill` scaffolds a skill within a project)*.
- `init harness` subcommand generates a new custom harness definition in `harnesses/`.
- Scaffolded skills include `references/` and `scripts/` asset directories.
- Sample skill templates use variable references like `{{ skill_name }}` and `{{ harness.id }}`.

### Epic D — Safety & Robustness

- Path traversal protection with canonicalization and component-level checks.
- Atomic file writes (write to temp, then rename) prevent partial output.
- Interactive overwrite confirmation (y/n/s/a) with automatic non-interactive detection.
- SIGINT/SIGTERM signal handling with graceful exit.
- Verbose mode with per-phase timing and resolved variable listing.
- Path collision detection before rendering.
- Template source line numbers in render errors for easier debugging.

### Epic C — Developer Experience

- `build --diff` preview mode showing colored unified diffs.
- `build --force` flag to skip user-scope file safety checks.
- `init project` and `init skill` scaffolding commands.
- Rustdoc for all public items.
- README with installation, quickstart, and development guides.

### Epic B — Pipeline

- Template resolution engine (MiniJinja) with variable substitution and custom helpers.
- Harness resolver pairing skills to their target harnesses with capability checks.
- Validator checking template syntax, variable references, and macro references.
- Output router for deterministic path resolution, atomic file writing, and asset copying.

### Epic A — Foundation

- CLI framework with `build`, `validate`, and `init` subcommands via clap derive.
- Harness registry with 5 built-in harnesses: Claude, Codex, OpenCode, Factory, and Pi.
- Skill project model with YAML-based project configuration and skill metadata.
- Project loader for discovering and parsing skill projects.

## License

Licensed under the Apache License, Version 2.0. See `LICENSE` for details.

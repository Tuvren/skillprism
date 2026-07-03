# Changelog

## Unreleased — Distribution CLI

- **Distribution commands** — New `add`, `list` (`ls`), `remove` (`rm`), and `update` (`up`) subcommands for installing skills from remote Git repositories and local paths, managing their lifecycle, and keeping them up to date. Each installed skill is auto-detected as skillprism-format (rendered through MiniJinja per harness) or plain-format (copied as-is). Update performs lightweight `git ls-remote` up-to-date checks and per-file SHA-256 content comparison. State is tracked in `~/.config/skillprism/installed.yaml`.

## Unreleased — QoL for skill authors

- **Spec-compliant scaffolds** — `init skill` and `init project` now emit `SKILL.md` with the YAML frontmatter (`name` + `description`) the [Agent Skills spec](https://agentskills.io/specification) requires. Previously the scaffold produced frontmatter-less skills that no client could discover.
- **Spec validation** — `validate` now enforces Agent Skills spec constraints: skill name format (`^[a-z0-9]+(-[a-z0-9]+)*$`), name matches directory, non-empty description, per-harness length caps, and compatibility 1–500 chars. Values over the spec's portable cap but within a harness's own cap (e.g. 1025–1536 chars for Claude) are warnings, not errors. Previously `name_max_length` and `description_max_length` were parsed but never enforced.
- **Removed unused `frontmatter_mode` capability** — The harness capability `frontmatter_mode` ("strict"/"lenient"/"extended") was parsed but never used. skillprism now always emits spec-compliant `name` + `description` frontmatter, so the field has been removed from built-in harnesses, custom harness scaffolds, and docs.
- **Removed dead `init skill --harnesses` flag** — The `-H`/`--harnesses` flag on `init skill` was accepted but silently ignored (the parameter was unused). Harness scoping is project-wide in `skillprism.yaml`; per-skill scoping isn't a concept. The flag has been removed.
- **Scaffold polish** — Scaffolded skills default to a minimal, spec-compliant template with `variables:` and `harnesses:` shown as commented optional examples instead of active defaults. `init project` now generates a `.gitignore` (for harness output dirs) and a short project `README.md`. Placeholder descriptions guide authors to include trigger keywords.
- **CLI output polish** — `validate` now lists each skill×harness pair and any portability warnings. `build` prints "No skills to build" when zero pairs resolve instead of silent success.
- **README fixes** — Corrected the CLI reference (was `--targets`, now `--harnesses`; added `init harness`, `completions`, `--verbose`, `--dry-run`), the clippy command (was `-W`, now `-D warnings`), and `prek` → `pre-commit`. Replaced internal `.constitution/` schema references with links to the public docs.

## v0.1.0 — Release Readiness

### Epic H — Release Readiness

- **Shell completions** — New `completions` subcommand generates shell completion scripts for Bash, Fish, and Zsh.
- **`--dry-run` alias** — `build --dry-run` is now a visible alias for `build --diff`, showing a diff preview without writing files.
- **Man page** — A `skillprism.1` man page can be generated via `scripts/generate-man.sh`.
- **CLI help polish** — All subcommands and flags now have consistent, professional descriptions in `--help` output.
- **Release CI** — Tag-based GitHub Actions workflow builds and attaches binaries for Linux (x86_64) and macOS (x86_64 + ARM) to GitHub Releases.
- **`.gitignore` polish** — Added `.direnv/`, `dist/`, and `*.tmp` entries.
- **`cargo publish` readiness** — Cargo metadata verified and ready for crates.io publication.

### Epic G — Code Quality

- Removed dead code across the codebase (unused variants, functions, and modules).
- Replaced all module-level `#![allow(...)]` attributes with targeted per-item annotations.
- No ambient `#[allow(dead_code)]` remains without justification.

### Epic F — Testing & CI

- Integration test suite with 3 end-to-end CLI tests covering the full build pipeline.
- Fixture project with 2 skills x 2 harnesses for reproducible testing.
- GitHub Actions CI workflow running build, test, clippy, and format checks on every push and PR.
- Pre-commit hooks for `cargo fmt` and `cargo clippy` via devenv.

### Epic E — Scaffolding Enhancements

- `init project` now accepts `--harnesses` to specify which harnesses to scaffold for.
- `init skill --harnesses` replaces `--targets` for naming consistency.
- `init harness` subcommand generates a new custom harness definition in `harnesses/`.
- Scaffolded skills include `references/` and `scripts/` asset directories.
- Sample skill templates use variable references like `{{ skill_name }}` and `{{ harness.id }}`.

### Epic D — Safety & Robustness

- Path traversal protection with canonicalization and component-level checks.
- Atomic file writes (write to temp, then rename) prevent partial output.
- Interactive overwrite confirmation (y/n/s/a) with automatic non-interactive detection.
- SIGINT/SIGTERM signal handling with graceful exit (codes 130/143).
- Verbose mode with per-phase timing and resolved variable listing.
- Path collision detection before rendering.
- Template source line numbers in render errors for easier debugging.
- Missing asset directory warnings.
- Actionable `$HOME` check instead of falling back to `/tmp`.

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

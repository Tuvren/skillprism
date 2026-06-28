# Changelog

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

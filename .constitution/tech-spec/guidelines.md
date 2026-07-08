# Guidelines & Project Structure

**Version:** v0.1.0

## Developer Environment (Phase 0)

The project uses [devenv](https://devenv.sh/) for reproducible developer environments.
This is a Phase 0 concern (outside the scope of any epic) — it provides the Rust toolchain,
formatters, linters, and any system dependencies needed for local development.

Configuration lives in `devenv.nix` / `devenv.yaml` at the project root, managed by `devenv shell`
or integrated with `direnv`.

## Project Structure

```
skillprism/
├── Cargo.toml              # Package manifest (single crate)
├── Cargo.lock              # Lockfile (committed)
├── rust-toolchain.toml     # MSRV pin (1.85, edition 2024)
 ├── .github/workflows/
│   ├── ci.yml              # GitHub Actions CI (matrix build, test, clippy, fmt)
│   └── release.yml         # GitHub Actions release (tag-triggered, matrix, GH Release)
├── scripts/
│   └── generate-man.sh     # Man page regeneration script
├── src/
│   ├── main.rs             # CLI entrypoint (clap dispatch + hidden __generate_man)
│   ├── cli.rs              # Command/flag definitions (clap derive) + pipeline dispatch
│   ├── distribution/       # Distribution CLI (Epic I): add/list/remove/update
│   │   ├── mod.rs          # Curated command entrypoints + CommandError + shared helpers
│   │   ├── add.rs          # `add` command (fetch, prompt scope/harness, install)
│   │   ├── list.rs         # `list`/`ls` command
│   │   ├── remove.rs       # `remove`/`rm` command
│   │   ├── update.rs       # `update`/`up` command
│   │   ├── install.rs      # Shared install logic (discovery, format detect, render/copy)
│   │   ├── detect.rs       # Installed-agent auto-detection
│   │   ├── network.rs      # Git fetch + auth chain (git → gh → SSH), credential masking
│   │   └── source.rs       # Source URL parser (v1 forms) + credential redaction
│   ├── state/              # Installation state tracking (Epic I)
│   │   ├── mod.rs          # Module exports
│   │   └── installed.rs    # StateStore — atomic, schema-versioned installed.yaml
│   ├── loader/
│   │   ├── mod.rs          # Module exports + public API
│   │   └── project.rs      # Project discovery, YAML parsing, variable merge
│   ├── registry/
│   │   ├── mod.rs          # Harness Registry — built-in + user overrides
│   │   └── types.rs        # HarnessDefinition + all sub-types
│   ├── resolver/
│   │   └── mod.rs          # Harness Resolver — skill-harness pairing + capability checks
│   ├── validator/
│   │   ├── mod.rs          # Validator — batch check all skills
│   │   ├── syntax.rs       # MiniJinja parse-only check
│   │   ├── macros.rs       # Macro reference resolution
│   │   └── variables.rs    # Variable reference resolution
│   ├── engine/
│   │   ├── mod.rs          # Template Engine — MiniJinja rendering + manifest entries
│   │   ├── context.rs      # Build template context (harness, skill, helpers)
│   │   └── helpers.rs      # Custom MiniJinja functions (skill_ref, etc.)
│   ├── router/
│   │   ├── mod.rs          # Output Router — path resolution, writing, diffs, manifests
│   │   ├── paths.rs        # Target scope path resolution
│   │   ├── write.rs        # Atomic writes (temp → rename) + asset copy
│   │   ├── overwrite.rs    # Shared overwrite prompt/decision (force/skip-all/abort)
│   │   ├── manifest.rs     # Manifest entry records + aggregation
│   │   └── diff.rs         # Diff computation against installed files
│   ├── scaffold/
│   │   ├── mod.rs          # Scaffolder — init command handlers
│   │   ├── project.rs      # Full project scaffold (SC-1)
│   │   └── skill.rs        # Single skill scaffold (SC-2)
│   ├── types/
│   │   ├── mod.rs          # Shared domain types
│   │   ├── project.rs      # ProjectModel, SkillModel, SkillGroup
│   │   ├── harness.rs      # Re-exports HarnessDefinition from registry
│   │   └── error.rs        # Unified error types (miette)
│   └── builtin_harnesses/  # Compiled-in harness YAML (embedded via include_str!)
│       ├── claude.yaml
│       ├── codex.yaml
│       ├── opencode.yaml
│       ├── factory.yaml
│       └── pi.yaml
├── tests/
│   ├── fixtures/
│   │   └── valid/          # Integration test fixture project (2 skills, 2 harnesses)
│   ├── integration.rs      # CLI integration tests (build, validate, diff)
│   └── examples.rs         # Builds examples/ end-to-end, asserts on rendered output
├── examples/                # A single skillprism project showing off skillprism's own
│   ├── skillprism.yaml      # mechanics (see examples/README.md), not a ready-to-use
│   └── skills/              # skill library
│       ├── quickstart/      # Synthetic, minimal — tours every mechanism in one file
│       ├── mcp-builder/     # Real skill ported from anthropics/skills
│       └── webapp-testing/  # Real skill ported from anthropics/skills
└── harnesses/              # Users' override directory (documented, not shipped)
```

## Coding Standards

### Formatting & Linting

- Follow the [Official Rust Style Guide](https://doc.rust-lang.org/style-guide/) — the definitive reference for formatting, naming conventions (UpperCamelCase types, snake_case functions/variables, SCREAMING_SNAKE_CASE constants), and expression-oriented style.
- Configure `rustfmt` with the 2024 style edition via `.rustfmt.toml`:
  ```toml
  style_edition = "2024"
  ```
- `cargo fmt` required before every commit
- `#![deny(clippy::all, clippy::pedantic, clippy::nursery)]` at crate root
- Allowlist exceptions with `#[allow(...)]` on the narrowest scope, with a `// reason:` comment
- Module-level `pub(crate)` visibility to enforce internal boundaries — no `#[path]` annotations

### Error Handling

- All errors use `miette::Diagnostic` via the unified `types::error` module
- Use `thiserror` or manual `Diagnostic` derive — never `Box<dyn Error>` for user-facing errors
- Diagnostic context follows two patterns:
  - **File-backed diagnostics** (template read, syntax error, write error) carry `source_file`, `source_line` when available, and a human-readable `help` message with actionable file-level guidance
  - **Path/environment diagnostics** (path traversal, missing `$HOME`, absolute path rejection) carry the actionable path, scope, or environment context — not file/line references, since the issue is in configuration or environment state, not a specific source file
- Every `#[error(...)]` attribute must be paired with a `#[diagnostic(help(...))]` attribute providing the user-facing suggestion
- The Validator accumulates errors into a `Vec<SkillError>` — never short-circuits on first error

### Testing

- Unit tests co-located with source (`#[cfg(test)] mod tests` in each module)
- Integration tests in `tests/integration.rs` exercise the full build pipeline against a fixtures directory
- CLI tests use `assert_cmd` for end-to-end CLI binary validation with `predicates` for exit codes and stderr checking

### Module Exports

- Each library module re-exports its public API via `mod.rs`
- Internal submodules are `pub(crate)` unless an explicit interface boundary justifies `pub`
- Main.rs only calls `cli::run()` — no business logic in the entrypoint file

### Observability

- `--verbose` flag enables per-stage timing and per-skill progress via `eprintln!`
- No logging framework in v1 — structured `eprintln!` with consistent prefix format `[stage] message`
- Stdout is reserved for `--diff` output and scaffold confirmation messages; all other diagnostics go to stderr
- Distribution commands (`add`/`list`/`remove`/`update`): stdout carries only machine-readable data — the `list` table and `update --diff` patch. All prompts, install/removal summaries, confirmations, and per-skill status ("Updated X", "is up to date", "No installed skills", etc.) go to stderr so piped stdout stays clean

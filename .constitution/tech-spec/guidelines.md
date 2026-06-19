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
├── src/
│   ├── main.rs             # CLI entrypoint (clap dispatch)
│   ├── cli.rs              # Command/flag definitions (clap derive) + pipeline dispatch
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
- Every error must carry `source_file`, `source_line`, and a human-readable `help` message
- Use `thiserror` or manual `Diagnostic` derive — never `Box<dyn Error>` for user-facing errors
- The Validator accumulates errors into a `Vec<SkillError>` — never short-circuits on first error

### Testing

- Unit tests co-located with source (`#[cfg(test)] mod tests` in each module)
- Integration tests in `tests/integration/` exercise the full build pipeline against a fixtures directory
- CLI tests use `clap::Command::try_get_matches` or `assert_cmd` for end-to-end flag validation

### Module Exports

- Each library module re-exports its public API via `mod.rs`
- Internal submodules are `pub(crate)` unless an explicit interface boundary justifies `pub`
- Main.rs only calls `cli::run()` — no business logic in the entrypoint file

### Observability

- `--verbose` flag enables per-stage timing and per-skill progress via `eprintln!`
- No logging framework in v1 — structured `eprintln!` with consistent prefix format `[stage] message`
- Stdout is reserved for `--diff` output and scaffold confirmation messages; all other diagnostics go to stderr

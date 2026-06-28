# Stack — Bill of Materials

**Version:** v0.1.0

## Language & Runtime

| Concern | Selection | Version | Rationale |
| :--- | :--- | :--- | :--- |
| Language | Rust | 1.85+ (Edition 2024) | Single static binary, zero runtime dependencies, MiniJinja native ecosystem, strongest PRD constraint alignment |
| Build system | Cargo | Bundled with Rust | Standard Rust build tool, single-binary output via `cargo build --release` |
| Edition | 2024 | 1.85+ | Stabilized Feb 2025; enables `unsafe_op_in_unsafe_fn`, `macro_rules` hygiene improvements, and `cargo` resolver v3 |

## Dependencies

| Concern | Crate | Version | Justification |
| :--- | :--- | :--- | :--- |
| CLI argument parsing | `clap` | 4.6.x | Industry standard Rust CLI parser; derive macros for declarative command/flag definitions |
| Shell completion generation | `clap_complete` | 4.6.x | Generates Bash, Fish, and Zsh completion scripts from clap command definitions |
| Man page generation | `clap_mangen` | 0.3.x | Renders roff man pages from clap command definitions for `skillprism.1` |
| Template engine | `minijinja` | 2.20.x | Chosen by PRD; authored by Jinja2 creator Armin Ronacher; designed for text/YAML codegen; parse-only mode for validation without side effects |
| YAML deserialization | `yaml_serde` | 0.10.x | Serde-based YAML parsing; actively maintained fork of the deprecated `serde_yaml` by The YAML Organization |
| Error diagnostics | `miette` | 7.6.x | Structured error reporting with source spans, file/line references, and rich terminal output; satisfies the PRD's actionable-error constraint |
| Diff computation | `similar` | 2.7.x | Colored unified diffs for `--diff` mode; features = ["text"] |

## Tooling

| Concern | Tool | Version | Rationale |
| :--- | :--- | :--- | :--- |
| Code formatting | `rustfmt` | Bundled | Project-wide consistent formatting enforced in CI |
| Linting | `clippy` | Bundled | Enforce `#![deny(clippy::all, clippy::pedantic)]` in CI; allowlist exceptions per module where justified |
| Testing | `cargo test` | Bundled | Unit tests per pipeline stage; inline assertions for template rendering |
| CI (push/PR) | GitHub Actions | N/A | Matrix build (Linux, macOS) on push to master and all PRs; steps: build, test, clippy, fmt |
| Release CI | GitHub Actions | N/A | Tag-triggered (`v*`) release workflow; matrix: linux x86_64, macOS x86_64 + ARM; uploads binaries to GitHub Release |
| CLI test harness | `assert_cmd` | 2.x (dev) | End-to-end CLI binary testing in integration tests |
| CLI predicates | `predicates` | 3.x (dev) | Exit code and stderr matching for CLI assertions |
| Temp directories | `tempfile` | 3.x (dev) | Isolated temp directories for fixture-based integration tests |
| Task automation | `just` | Latest (devenv) | Ad-hoc automation recipes in justfile |

## Compatibility Policy

| Concern | Policy |
| :--- | :--- |
| Dependency pinning | Pin minor versions in `Cargo.toml` (e.g., `minijinja = "2.20"`); lockfile (`Cargo.lock`) committed to repo for reproducible builds |
| MSRV | Minimum supported Rust version: 1.85 (Edition 2024). Tracked in `Cargo.toml` via `package.rust-version` |
| Upgrade window | Minor-version dependency upgrades within 90 days of release. Major-version upgrades require an ADR |
| Breaking changes | Any dependency upgrade that changes rendered output or CLI behavior must update snapshot tests before merge |

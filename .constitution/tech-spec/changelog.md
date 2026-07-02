# Changelog — Stage 3 (TechSpec)

### v0.11.0 — 2026-07-02 — ADR-008: Network Layer for Distribution

- **New ADR: `ADR-008-network-layer-for-distribution.md`** documents the design for the `add` and `update` distribution commands' network access.
- Shells out to `git` directly for shallow clones (no new Rust dependencies).
- Three-layer auth chain (per Vercel parity): `git clone` → `gh repo clone` (GitHub HTTPS only) → SSH with `BatchMode=yes`.
- Env vars: `GIT_TERMINAL_PROMPT=0`, `GIT_SSH_COMMAND=ssh -o BatchMode=yes`, `GIT_LFS_SKIP_SMUDGE=1`.
- Companion changes: PRD `constraints.md` v0.2.0 (allows `git` as documented runtime dep for distribution commands), Architecture `strategy.md` v0.2.2 (scopes the network exception to distribution commands).
- Implementation: new `src/distribution/network.rs` module; no changes to existing ADR-003/004/005 contracts.

### v0.10.0 — 2026-06-27 — Epic H Release Readiness

- **Epic H (Release Readiness)** fully implemented and archived
- 8 tickets completed: RELS-H001 through RELS-H008
- 13 story points delivered
- Added `clap_complete` and `clap_mangen` to BOM
- Updated BOM with CI/release workflow entries and dependency additions
- Updated CLI contract in `contracts/cli.rs` with Completions subcommand, ShellKind, --dry-run alias, and consistent doc comments
- Updated project structure tree in `guidelines.md` with `.github/workflows/release.yml` and `scripts/`
- Added `CHANGELOG.md` at project root for user-facing release notes

### v0.6.0 — 2026-06-23 — Epic F Testing & CI

- **Epic F (Testing & CI)** fully implemented and archived
- 3 tickets completed: TEST-F001 through TEST-F003
- 8 story points delivered
- Added `assert_cmd`, `predicates`, `tempfile` dev-dependencies
- Created `tests/integration.rs` with 3 CLI binary end-to-end tests
- Created `tests/fixtures/valid/` fixture project
- Added `.github/workflows/ci.yml` — matrix build with locked dependencies
- Added CI badge to README.md
- Added rustfmt + clippy (denyWarnings) pre-commit hooks via devenv `git-hooks.hooks`
- Added `git-hooks` input to devenv configuration
- Updated project structure tree in guidelines.md to include `tests/` and `.github/` directories
- Updated BOM in stack.md with dev-dependencies and CI tooling

### v0.5.0 — 2026-06-23 — Epic E Scaffolding Enhancements

- **Epic E (Scaffolding Enhancements)** fully implemented and archived
- 4 tickets completed: SCAFF-E001 through SCAFF-E004
- 7 story points delivered
- Added `--harnesses` flag to `init project` (defaults to claude, opencode)
- Renamed `init skill --targets` to `--harnesses` for naming consistency
- `init project` now scaffolds a sample skill with variable references
- `init skill` now creates `references/` and `scripts/` asset directories
- Added `init harness` subcommand with placeholder YAML generation
- Updated CLI contract in `contracts/cli.rs`

### v0.4.1 — 2026-06-18 — Constitutional Gap Fixes

- Fixed manifest aggregation bug: manifests now batch-aggregate all skills into a JSON array instead of per-skill overwrite (last-write-wins)
- Added `manifest:` block with entry-level templates to Claude and Codex builtin harnesses
- Removed `insta` from BOM (never added to `Cargo.toml`; no snapshot tests implemented)
- Added `similar` to BOM table (was missing despite being in actual deps since v0.4.0)
- Fixed module structure tree in `guidelines.md` to match actual simpler file layout
- Removed stale `serde_json` mention from changelog (never added to `Cargo.toml`)

### v0.4.0 — 2026-06-17 — Epic C Implementation

- Epic C (Developer Experience) fully implemented: 3 milestones, 10 story points
- Added `similar = "2.7"` with `text` feature for diff computation
- Implemented `src/router/diff.rs` — unified diff with ANSI color highlighting
- Implemented CLI pipeline wiring: load → resolve → validate → render → route
- Added `--diff` flag for preview mode (colored unified diffs without writing)
- Added `--force` flag to skip user-scope file safety checks
- Added `src/scaffold/` module for `init project` and `init skill` commands
- `TargetScope` made `Copy` for efficient pass-by-value
- Added `WriteResult` type with `written` and `skipped` tracking
- Added `SkippedFile` variant to `RouterError`
- Added rustdoc for all public items
- Added README.md with installation, quickstart, and development sections

### v0.3.0 — 2026-06-17 — Epic B Implementation

- Epic B (Pipeline) fully implemented: 4 milestones, 16 story points
- Added `minijinja = "2.20"` with `json` feature to BOM
- Implemented `src/resolver/` — `HarnessResolver`, `ResolvedPair`, `ResolveError`
- Implemented `src/validator/` — syntax, variables, macros checkers, collect-all-errors pattern
- Implemented `src/engine/` — MiniJinja rendering, context building, helpers (skill_ref)
- Implemented `src/router/` — path resolution, atomic writes, asset copying
- Added `required_capabilities` field to SkillModel and skill.yaml parsing
- `TargetScope` made `pub` for cross-module access
- No structural API contract deviations from Phase 2 TechSpec

### v0.2.0 — 2026-06-17 — Epic A Implementation

- Epic A (Foundation) fully implemented: 4 milestones, 9 story points
- Added `thiserror = "2"` and `serde = "1"` to BOM; enabled `features = ["derive"]` on relevant crates
- Implemented `src/registry/types.rs` — full `HarnessDefinition` with all fields per harness-schema.json
- Tightened crate-root clippy to `#![deny(all, pedantic, nursery)]` (exceeds guidelines baseline)
- No structural API contract deviations from Phase 0/1 TechSpec

### v0.1.1 — 2026-06-17

- Added ADR-006 (yaml_serde for YAML deserialization) and ADR-007 (miette for error diagnostics)
- Added `SkillGroup` data model for directory hierarchy before variable flattening
- Fixed terminology drift in `harness-schema.json` description ("target" → "harness")
- Fixed changelog to include `project-config-schema.json` in contracts list

## v0.1.0 — 2026-06-17

- Initial TechSpec established from PRD (v0.1.1) and Architecture (v0.1.0)
- BOM: Rust 1.85+ (Edition 2024), minijinja 2.20, clap 4.6, yaml_serde 0.10, miette 7.6
- Single crate project structure defined (6 library modules + CLI entrypoint)
- ADRs: Rust language, MiniJinja, single crate, synchronous pipeline, atomic writes, yaml_serde, miette
- Contracts: CLI command tree, harness definition JSON Schema, project configuration JSON Schema, skill YAML JSON Schema
- Data models: in-memory domain types documented with Rust struct sketches

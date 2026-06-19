# Changelog — Stage 3 (TechSpec)

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

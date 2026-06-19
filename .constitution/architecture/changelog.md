# Architecture Changelog

### v0.2.1 — 2026-06-18 — Manifest Aggregation Fix

- Fixed manifest aggregation bug: manifest entries now collected per-skill and batch-aggregated into a JSON array after all skills are rendered
- Manifest writing moved from per-skill `Router::write()` to batch `Router::write_aggregated_manifests()`
- Engine now exposes `render_manifest_entry()` as a standalone method separate from `render()`
- Manifest `ManifestDef.template` now renders a single entry (not the full file); aggregation wraps entries in appropriate format

### v0.2.0 — 2026-06-17 — Epic B Implementation

- **Epic B (Pipeline)** fully implemented: 4 containers built (Resolver, Validator, Engine, Router)
- Pipe-and-filter pipeline now extends through Load → Resolve → Validate → Render → Route/Write
- Resolver stage added between Load and Validate; produces resolved skill-harness pairs
- Validator implements collect-all-errors pattern (VA-1) with miette diagnostics
- Template Engine uses MiniJinja for rendering with harness macros as context values
- Output Router implements atomic writes (temp → rename) and asset directory copying
- No structural container boundary changes from Architecture v0.1.0

### v0.1.1 — 2026-06-17

- Fixed BD-2 flag listing in `flows/flow-deploy-user.md`: `(project | user)` → `(project | user | dist)` to match PRD v0.1.1

## v0.1.0 — Initial Architecture

- Established local-first compilation pipeline (pipe-and-filter) pattern
- Defined 7 logical containers: CLI Entrypoint, Project Loader, Harness Registry, Validator, Template Engine, Output Router, Scaffolder
- Documented collect-all-errors strategy and atomic write safety
- Created 12 flow files mapping to all P0 capabilities
- PRD corrected (v0.1.1): BD-1/BD-2 default scope changed to project-level agent paths (deploy-first model)
- Identified 6 logical risks with mitigations

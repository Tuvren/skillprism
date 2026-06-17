# Changelog — Stage 3 (TechSpec)

### v0.1.1 — 2026-06-17

- Added ADR-006 (yaml_serde for YAML deserialization) and ADR-007 (miette for error diagnostics)
- Added `SkillGroup` data model for directory hierarchy before variable flattening
- Fixed terminology drift in `harness-schema.json` description ("target" → "harness")
- Fixed changelog to include `project-config-schema.json` in contracts list

## v0.1.0 — 2026-06-17

- Initial TechSpec established from PRD (v0.1.1) and Architecture (v0.1.0)
- BOM: Rust 1.85+ (Edition 2024), minijinja 2.20, clap 4.6, yaml_serde 0.10, miette 7.6, insta 1.48
- Single crate project structure defined (6 library modules + CLI entrypoint)
- ADRs: Rust language, MiniJinja, single crate, synchronous pipeline, atomic writes, yaml_serde, miette
- Contracts: CLI command tree, harness definition JSON Schema, project configuration JSON Schema, skill YAML JSON Schema
- Data models: in-memory domain types documented with Rust struct sketches

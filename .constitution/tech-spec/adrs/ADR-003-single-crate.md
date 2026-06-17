# ADR-003: Single Crate Project Structure

**Status:** Accepted

## Context

The architecture defines 7 logical containers that communicate via in-process function calls. The project is a solo-developer CLI tool with no library consumers. A workspace with multiple crates would add build complexity (dependency version coordination, inter-crate publish ordering) without clear benefit.

## Decision

Use a single Cargo crate with the following internal module isolation:

- `src/types/` — Shared domain types (the "pipe" data structures)
- `src/loader/`, `src/registry/`, `src/validator/`, `src/engine/`, `src/router/`, `src/scaffold/` — One module per container
- `src/cli.rs` — Clap definitions and dispatch
- `src/builtin_harnesses/` — YAML files embedded via `include_str!`

Modules communicate via `pub(crate)` types defined in `src/types/`. The module boundary is enforced by convention and code review, not by crate boundaries.

## Consequences

- **Positive:** Single `cargo build`, single `Cargo.toml`, no workspace resolution overhead. Faster compile times than multi-crate. Easier refactoring for a solo developer.
- **Negative:** No compiler-enforced module boundary. A future team split would require extracting crates.
- **Mitigation:** Module-level `pub(crate)` visibility and CI clippy rules prevent cross-module coupling from becoming implicit.

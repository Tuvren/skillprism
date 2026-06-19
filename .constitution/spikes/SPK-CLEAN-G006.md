# Spike Report: CLEAN-G006 Evaluate yaml_serde → serde_yml Migration

## 1. Context & Objective
- **Triggering upstream file/section:** `.constitution/tech-spec/stack.md` — YAML deserialization via `yaml_serde 0.10.x`
- **Target:** Determine whether `yaml_serde` (a fork of the deprecated `serde_yaml`) should be replaced with `serde_yml`, and estimate migration effort.

## 2. Codebase Baseline
- **Current State:** Project depends on `yaml_serde = "0.10"`. Used in: `loader/project.rs` (skill.yaml/skillprism.yaml parsing), `registry/types.rs` (harness YAML deserialization via serde), engine context building (yaml_serde::Value conversions).
- **Discovered Constraints:** All YAML parsing is via serde `Deserialize` derives or direct `from_str` calls. No manual YAML event-level processing.

## 3. Options & Trade-offs
- **Option A — Stay on yaml_serde:** Lowest risk short-term. The fork is functional for current needs. Risk: if unmaintained, may block future Rust editions.
- **Option B — Migrate to serde_yml:** Drop-in replacement candidate. Same serde-based API. Requires vetting API compatibility for `Location`, error types, and `Value` type.

## 4. Execution Directives
- **Chosen Option:** *[To be determined by spike findings]*
- **Why it fits:** *[TBD]*
- **Downstream Backlog Impact:** If migration is chosen, a follow-up implementation ticket should be added to the active backlog.

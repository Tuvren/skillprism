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

### 3.1 API Compatibility Matrix

| Concern | yaml_serde 0.10 | serde_yml (latest) | Compatible? |
|---------|-----------------|--------------------|-------------|
| `Deserialize` trait derive | — | — | *to verify* |
| `from_str<T: Deserialize>` | — | — | *to verify* |
| `Value` type | — | — | *to verify* |
| Error type (`Location`, `line()`) | — | — | *to verify* |
| Serde `Deserializer` impl | — | — | *to verify* |

## 4. Recommendation
- **Chosen Option:** *[migrate now | defer | don't migrate]*
- **Rationale:** *[to be determined by spike findings]*
- **Migration Effort Estimate:** *[hours or SP]*

## 5. Execution Directives
- **Chosen Option:** *[mirrors section 4]*
- **Why it fits:** *[to be determined by spike findings]*
- **Downstream Backlog Impact:** If migration is chosen, a follow-up implementation ticket should be added to the active backlog.

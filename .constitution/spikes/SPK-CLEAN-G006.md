# Spike Report: CLEAN-G006 Evaluate yaml_serde → serde_yml Migration

## 1. Context & Objective
- **Triggering upstream file/section:** `.constitution/tech-spec/stack.md` — YAML deserialization via `yaml_serde 0.10.x`
- **Target:** Determine whether `yaml_serde` (a fork of the deprecated `serde_yaml`) should be replaced with `serde_yml`, and estimate migration effort.

## 2. Codebase Baseline
- **Current State:** Project depends on `yaml_serde = "0.10"`. Used in: `loader/project.rs` (skill.yaml/skillprism.yaml parsing), `registry/types.rs` (harness YAML deserialization via serde), engine context building (`yaml_serde::Value` conversions), and type annotations across 5 source files (~23 usage sites).
- **Discovered Constraints:** All YAML parsing is via serde `Deserialize` derives or direct `from_str` calls. No manual YAML event-level processing. `yaml_serde::Value` is used as a dynamic DOM type for variable storage throughout the pipeline.

## 3. Options & Trade-offs
- **Option A — Stay on yaml_serde:** Low risk. Actively maintained by the official YAML organization. No migration cost.
- **Option B — Migrate to serde_yml:** **Counterproductive.** `serde_yml` is itself deprecated since 0.0.13 (May 2026). The crate's own `MIGRATION.md` tells users to migrate *away* from it. This would be trading one deprecated fork for another.
- **Option C — Migrate to noyalib:** The backend that `serde_yml` 0.0.13 wraps. Pure-Rust (`#![forbid(unsafe_code)]`), drop-in `compat-serde-yaml` feature. However, `noyalib` is version 0.0 (pre-1.0), raising stability concerns.
- **Option D — Migrate to serde-saphyr:** Serde-integrated typed deserialisation. **No `Value` DOM** — incompatible with the codebase's use of `yaml_serde::Value` for dynamic variable trees.

### 3.1 Crate Health Comparison

| Metric | yaml_serde 0.10.4 | serde_yml 0.0.13 |
|--------|-------------------|-------------------|
| **Maintainer** | The YAML Organization (Ingy döt Net) | Sebastien Rousseau (unmaintained) |
| **Latest release** | 2026-03-11 | 2026-05-27 (final — deprecation shim) |
| **Status** | Active | **DEPRECATED** |
| **MSRV** | 1.82 | 1.85 |
| **Downloads (90d)** | 420,345 | 6,392,267 |
| **Reverse deps** | 115 | 376 |
| **Security** | Regular audits by YAML Company | RUSTSEC-2025-0068 (fixed in final shim) |
| **C-FFI** | `unsafe-libyaml` | Removed in 0.0.13 (forwards to `noyalib`) |

### 3.2 API Compatibility Matrix

| Concern | yaml_serde 0.10 | serde_yml 0.0.13 | Compatible? |
|---------|-----------------|--------------------|-------------|
| `Deserialize` trait derive | `#[derive(Deserialize)]` | Same (re-exports from noyalib compat) | ✅ |
| `from_str<T: Deserialize>` | `yaml_serde::from_str` | `serde_yml::from_str` (deprecated wrapper) | ✅ but deprecated |
| `Value` type | `yaml_serde::Value` | `serde_yml::Value` (deprecated re-export) | ✅ but deprecated |
| `Mapping` type | `yaml_serde::Mapping` | `serde_yml::Mapping` (deprecated re-export) | ✅ but deprecated |
| Error type (`Location`, `Display`) | `yaml_serde::Error` with `Location` | `serde_yml::Error` wrapper | ⚠️ location API differs |
| Serde `Deserializer` impl | `yaml_serde::Deserializer` | Removed in 0.0.13 | ❌ |
| `to_string` / `to_value` | `yaml_serde::to_string` / `to_value` | `serde_yml::to_string` / `to_value` (deprecated) | ✅ but deprecated |
| `with::singleton_map*` | `yaml_serde::with` | `serde_yml::with` (deprecated re-export) | ✅ but deprecated |
| `libyml` / low-level loader | Not used by this project | Removed in 0.0.13 | N/A (not used) |
| `unsafe-libyaml` C-FFI | Yes | Removed (noyalib pure-Rust) | N/A (architectural) |

### 3.3 Live Upgrade Paths from Current yaml_serde

| Destination | Change required | Fits this project? |
|-------------|-----------------|-------------------|
| **Stay on yaml_serde** | None | ✅ — actively maintained |
| **noyalib** (via compat-serde-yaml) | Rename imports `yaml_serde` → `noyalib::compat::serde_yaml` | ✅ — drop-in API, but pre-1.0 |
| **serde-saphyr** | Replace `Value` usage with typed structs | ❌ — no `Value` DOM, would require architectural changes |
| **yaml-rust2** | Replace serde with manual parsing | ❌ — not serde-integrated, would require rewrite |

## 4. Recommendation
- **Chosen Option:** **Don't migrate** — stay on `yaml_serde`
- **Rationale:** The original spike premise (migrate from `yaml_serde` to `serde_yml`) is based on an incorrect assumption. `yaml_serde` is **not** unmaintained — it is the **actively maintained** fork of `serde_yaml` by the official YAML organization, with regular releases, security audits, and professional stewardship by The YAML Company. Meanwhile, `serde_yml` is **itself deprecated** (final release 0.0.13 is a migration shim). Migrating from an actively maintained crate to a deprecated one would be a regression.

  The only legitimate reason to migrate away from `yaml_serde` would be to eliminate the C-FFI dependency (`unsafe-libyaml`). If this becomes a priority, the correct destination would be `noyalib` (pure-Rust, `#![forbid(unsafe_code)]`), not `serde_yml`. However, `noyalib` is pre-1.0 (version 0.0.x) and would introduce its own stability risk.

- **Migration Effort Estimate:** 0 hours — no migration recommended at this time.

## 5. Execution Directives
- **Chosen Option:** Don't migrate
- **Why it fits:** `yaml_serde` 0.10.4 meets all current needs, is actively maintained by the official YAML standards body, and has no known blockers for Rust edition 2024 compatibility. There is no maintenance-driven urgency to move.
- **Downstream Backlog Impact:** None recommended. If C-FFI elimination becomes a future requirement, a separate spike should evaluate `noyalib` directly.

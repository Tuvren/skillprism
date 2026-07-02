# Tasks Changelog

## v0.11.0 — Epic I Activated and Specified: Distribution CLI

- **Epic I (Distribution CLI)** planned, activated, and specified — 7 implementation tickets, 32 story points.
- Expands skillprism from build-time compiler to distribution CLI (Vercel `skills` CLI competitor with per-harness templating).
- New commands: `add` (fetch + auto-detect + render/copy), `list`, `remove`, `update`.
- Deferred to future epics: `find` (requires directory/registry backend), `use` (render-to-temp + agent launching).
- **Spike SPK-DIST-I001 complete** (`.constitution/spikes/SPK-DIST-I001.md`): remote fetch methodology resolved by reading the actual source of `vercel-labs/skills`. Recommendation: shell out to `git` directly for shallow clones. Three-layer auth chain (Vercel parity): `git clone` → `gh repo clone` (GitHub HTTPS only) → SSH with `BatchMode=yes`. Vercel-parity source URL parser (all 7 forms in v1). State file: single YAML at `~/.config/skillprism/installed.yaml`. Change detection: per-file SHA-256.
- **Tickets specified** (7 tickets, DIST-I001–DIST-I007): the spike lives in `.constitution/spikes/` as a free-floating prerequisite, not as a ticket inside the epic. The implementation tickets reference the spike for their contracts and mechanisms. Renumbered DIST-I001–DIST-I007 (state layer, `add`, `list`, `remove`, `update`, tests, docs).
- **State tracking:** `~/.config/skillprism/installed.yaml` (system-wide, not per-project), mode 0o700, schema-versioned, atomic writes per ADR-005.
- Reuses `--target project|user|dist` scope flag, atomic writes, scope confinement, collect-all-errors, `--diff` preview.
- Source format is **manifest-declared**: each skill directory's `skill.yaml` carries a `skillprism: '<version>'` field whose presence declares skillprism-format; absence of `skill.yaml` defaults to plain-format. The marker heuristic (`{{` / `{%` detection) is not the discriminator — markers in the template are allowed but do not determine format. The full format-decision rules and malformed-manifest error cases are in DIST-I002.
- **Upstream amendments in the same PR:**
  - Stage 1 (PRD) v0.2.0: `prd/constraints.md` allows `git` as a documented runtime dep for distribution commands only.
  - Stage 2 (Architecture) v0.2.2: `architecture/strategy.md` line 24 scopes the "no network" rule to non-distribution commands.
  - Stage 3 (TechSpec) v0.11.0: `ADR-008: Network Layer for Distribution` documents the design.
- **Critical path:** `DIST-I001` (state layer) → `DIST-I002` (`add`) → `DIST-I005` (`update`). After I001 lands, I002/I003/I004 can be worked in parallel. I006 and I007 depend on all command tickets.
- **Release plan:** Epic I (32 SP) is the gate for the `v1.0.0` tag. Once archived, Epic J (deferred scope) is the natural successor.
- PRD non-goal `plugin-marketplace.md` reopened by operator directive (recorded in `prd/changelog.md` v0.2.0); PRD revision is a downstream follow-up.

## v0.10.0 — Epic H Complete — All Release Readiness Tickets Delivered

- **Epic H (Release Readiness)** fully implemented and archived via `git mv`
- 8 tickets completed: RELS-H001 through RELS-H008 (13 story points)
- All 125 unit tests and 4 integration tests pass
- `cargo publish --dry-run` validates cleanly
- Total delivery trajectory: 88 SP across 8 completed epics

## v1.0.0 — Epic H Rewritten for Full Release Readiness

- **Epic H restructured**: Removed RELS-H001 (license — externally completed in commit `79c4211` as Apache 2.0). Renumbered remaining tickets and added 5 new items.
- **8 active tickets**, 13 total story points
- **New scope:**
  - RELS-H004 — CLI help polish (2 SP)
  - RELS-H005 — Man page generation (1 SP)
  - RELS-H006 — Release CI workflow (3 SP)
  - RELS-H007 — User-facing CHANGELOG (1 SP)
  - RELS-H008 — Cargo publish readiness (2 SP)
- **Preserved scope:**
  - RELS-H001 — Shell completions (2 SP, was H002)
  - RELS-H002 — `--dry-run` alias (1 SP, was H003)
  - RELS-H003 — `.gitignore` polish (1 SP, was H004)
- Wave 1: H001, H002, H003, H006 (fully parallel). Wave 2: H004, H005 (after H001/H002). Wave 3: H007, H008 (final)
- Total delivery trajectory: 75 SP completed + 13 SP active = 88 SP

## v0.9.0 — Epic G Complete

- **Epic G (Code Quality)** fully implemented and archived
- 8 tickets completed: CLEAN-G001 through CLEAN-G008
- 9 story points delivered
- **CLEAN-G001–G003:** Removed dead code (TemplateCollision variant, MissingField variant, skill_output_dir function)
- **CLEAN-G004–G008:** Replaced all module-level `#![allow(...)]` attributes with targeted per-item annotations
- **CLEAN-G006 (Spike):** Evaluated `yaml_serde` → `serde_yml` migration; recommended staying on `yaml_serde` (actively maintained by The YAML Organization; `serde_yml` is itself deprecated)
- No ambient `#[allow(dead_code)]` or `#![allow(...)]` remains at module level without justification
- Active backlog reduced to 5 story points (Epic H only)
- Total delivery trajectory: 75 SP across 7 completed epics

## v0.8.0 — Epic F Complete

- **Epic F (Testing & CI)** fully implemented and archived
- 3 tickets completed: TEST-F001 through TEST-F003
- 8 story points delivered
- Integration test suite: 3 end-to-end CLI tests in `tests/integration.rs`
- Fixture project under `tests/fixtures/valid/` with 2 skills × 2 harnesses
- GitHub Actions CI workflow with matrix build (Linux, macOS)
- Pre-commit hooks via devenv (rustfmt, clippy with `denyWarnings`)
- Fixed `is_builtin()` in validator to handle dotted variable names (e.g., `harness.id`)
- Active backlog reduced to 14 story points (Epics G/H)

## v0.7.0 — Epic E Complete

- **Epic E (Scaffolding Enhancements)** fully implemented and archived
- 4 tickets completed: SCAFF-E001 through SCAFF-E004
- 7 story points delivered
- New flags: `init project --harnesses`, `init skill --harnesses` (renamed from `--targets`)
- New subcommand: `init harness`
- New module: `src/scaffold/harness.rs`
- Scaffold now generates sample skill with `{{ skill_name }}` and `{{ harness.id }}`
- `init skill` creates `references/` and `scripts/` asset directories
- Active backlog reduced to 22 story points (Epics F/G/H)

## v0.6.0 — Epic D Complete

- **Epic D (Safety & Robustness)** fully implemented and archived
- 9 tickets completed: SAFE-D001 through SAFE-D009
- 16 story points delivered
- Active backlog reduced to 29 story points (Epics E/F/G/H)
- New modules: `router/paths` traversal checks, `router/write` atomic write, signal handling via `ctrlc` crate
- Path traversal protection with canonicalization and component-level fallback
- Interactive overwrite confirmation (y/n/s/a) with non-interactive detection
- SIGINT/SIGTERM signal handling with graceful exit (codes 130/143)
- Verbose phase timing and resolved variable listing
- Path collision detection before rendering
- Template source line numbers in render errors (minijinja `debug` feature)
- Missing asset directory warnings
- `$HOME` check returns actionable error instead of `/tmp` fallback

## v0.5.0 — Stable Release Planning

- **5 new epics planned (D/E/F/G/H)** for stable v1.0 readiness
- 28 active tickets written: 9 Safety, 4 Scaffolding, 3 Testing, 8 Code Quality, 4 Release
- 45 total active story points
- Execution sequence follows alphabetical epic order: D (SAFE) → E (SCAFF) → F (TEST) → G (CLEAN) → H (RELS)
- Epic D addresses 9 documented-but-unbuilt items from architecture/resilience.md and risks.md
- Epic E enhances init project/skill/harness scaffolding to match PRD spec
- Epic F creates integration test suite and CI pipeline
- Epic G removes dead code and cleans up lint suppressants
- Epic H adds release artifacts (LICENSE, completions, --dry-run, .gitignore)
- Post-audit correction: added CLEAN-G007 (loader/mod.rs lint) and CLEAN-G008 (types/mod.rs lint) — 2 module-level allows missed in original scan

## v0.4.0 — Epic C Complete

- **Epic C (Developer Experience)** fully implemented and archived
- 3 tickets completed: DX-C001, DX-C002, DX-C003
- 10 story points delivered
- Active backlog reduced to 0 story points
- New features: `--diff` preview mode, `--force` flag, scaffold commands
- New modules: `scaffold`, `router/diff`
- Added rustdoc for all public items and README.md

## v0.3.0 — Epic B Complete

- **Epic B (Pipeline)** fully implemented and archived
- 4 tickets completed: PIPE-B001, PIPE-B002, PIPE-B003, PIPE-B004
- 16 story points delivered
- Active backlog reduced to 10 story points (Epic C)
- New modules: `resolver`, `validator`, `engine`, `router`

## v0.2.0 — Epic A Complete

- **Epic A (Foundation)** fully implemented and archived
- 4 tickets completed: FND-A001, FND-A002, FND-A003, FND-A004
- 9 story points delivered
- Active backlog reduced to 26 story points (Epics B + C)

## v0.1.0 — Initial Engineering Backlog

- Initial task decomposition across 3 active Epics (A/B/C)
- 11 tickets: 4 Foundation, 4 Pipeline, 3 DX
- P1 Scaffolding deferred to future scope
- 35 total active story points

# Tasks Changelog

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

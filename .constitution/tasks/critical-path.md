# Critical Path — Stage 4 v0.5.0

## Active Backlog Summary

- **Total Active Story Points:** 22
- **Active Epics:** F (TEST, 8 SP), G (CLEAN, 9 SP), H (RELS, 5 SP)
- **Completed:** Epic A (Foundation) — 9 points, Epic B (Pipeline) — 16 points, Epic C (DX) — 10 points, Epic D (SAFE) — 16 points, Epic E (SCAFF) — 7 points = 58 total delivered
- **Critical Path:** TEST-F001 → TEST-F002 → TEST-F003 → (CLEAN-G001–G008) → (RELS-H001–H004)

TEST-F002 depends on TEST-F001. CLEAN-G001–G008 are independent of each other once TEST is green. RELS-H001–H004 are independent once CLEAN is clean.

- **Parallel Windows:** CLEAN-G001–G008 (8 tickets), RELS-H001–H004 (4 tickets)

## Build Order Diagram

```mermaid
flowchart LR
  SCAFF_DONE["✅ SCAFF Complete (epic gate)"]
  SCAFF_DONE --> TEST_F["F TEST-F001 Integration Tests"]

  TEST-F001 --> TEST-F002["F TEST-F002 CI Pipeline"]
  TEST-F002 --> TEST-F003["F TEST-F003 Pre-commit Hooks"]
  TEST-F003 --> CLEAN-G001["G CLEAN-G001-G008 Code Quality"]
  CLEAN-G008 --> RELS-H001["H RELS-H001-H004 Release"]
```

## Phasing Strategy

| Phase | Scope | Status |
|---|---|---|---|
| Phase 0–3 | Developer environment, Foundation, Pipeline, DX | ✅ Epics A–C — Completed |
| Phase 4 | Safety & Robustness: path security, signals, error quality | ✅ Epic D — Completed |
| Phase 5 | Scaffolding Enhancements: init flags, sample skill, init harness | ✅ Epic E — Completed |
| Phase 6 | Testing & CI: integration tests, CI pipeline, hooks | 🔲 Epic F — Planned |
| Phase 7 | Code Quality: remove dead code, clean lint allows | 🔲 Epic G — Planned |
| Phase 8 | Release Readiness: LICENSE, completions, polish | 🔲 Epic H — Planned |

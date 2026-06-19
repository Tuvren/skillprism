# Critical Path — Stage 4 v0.5.0

## Active Backlog Summary

- **Total Active Story Points:** 45
- **Active Epics:** D (SAFE, 16 SP), E (SCAFF, 7 SP), F (TEST, 8 SP), G (CLEAN, 9 SP), H (RELS, 5 SP)
- **Completed:** Epic A (Foundation) — 9 points, Epic B (Pipeline) — 16 points, Epic C (DX) — 10 points = 35 total delivered
- **Critical Path:** SAFE-D001 → SAFE-D006 → SAFE-D002 → SAFE-D003 → (SAFE-D004–D009) → (SCAFF-E001–E004) → TEST-F001 → TEST-F002 → TEST-F003 → (CLEAN-G001–G008) → (RELS-H001–H004)

SAFE-D006 and SAFE-D002 can be worked in parallel after SAFE-D001. SAFE-D004–D009 are independent after SAFE-D003. SCAFF-E001–E004 are independent of each other. TEST-F002 depends on TEST-F001. CLEAN-G001–G008 are independent of each other once TEST is green. RELS-H001–H004 are independent once CLEAN is clean.

- **Parallel Windows:** SAFE-D004–D009 (5 tickets), SCAFF-E001–E004 (4 tickets), CLEAN-G001–G008 (8 tickets), RELS-H001–H004 (4 tickets)

## Build Order Diagram

```mermaid
flowchart LR
  SAFE-D001["D SAFE-D001 Path Traversal"] --> SAFE-D006["D SAFE-D006 Collision Detection"]
  SAFE-D001 --> SAFE-D002["D SAFE-D002 Overwrite Confirm"]
  SAFE-D002 --> SAFE-D003["D SAFE-D003 Signal Handling"]
  SAFE-D003 --> SAFE-D004["D SAFE-D004 Verbose Timing"]
  SAFE-D003 --> SAFE-D005["D SAFE-D005 Verbose Variables"]
  SAFE-D003 --> SAFE-D007["D SAFE-D007 Source Mapping"]
  SAFE-D003 --> SAFE-D008["D SAFE-D008 Missing Assets"]
  SAFE-D003 --> SAFE-D009["D SAFE-D009 home_dir Error"]

  SAFE-D009 --> SCAFF-E001["E SCAFF-E001 Scaffold Enhancements"]
  SAFE-D009 --> TEST-F001["F TEST-F001 Integration Tests"]

  TEST-F001 --> TEST-F002["F TEST-F002 CI Pipeline"]
  TEST-F002 --> TEST-F003["F TEST-F003 Pre-commit Hooks"]
  TEST-F003 --> CLEAN-G001["G CLEAN-G001-G008 Code Quality"]
  CLEAN-G008 --> RELS-H001["H RELS-H001-H004 Release"]
```

## Phasing Strategy

| Phase | Scope | Status |
|---|---|---|
| Phase 0–3 | Developer environment, Foundation, Pipeline, DX | ✅ Epics A–C — Completed |
| Phase 4 | Safety & Robustness: path security, signals, error quality | 🔲 Epic D — Planned |
| Phase 5 | Scaffolding Enhancements: init flags, sample skill, init harness | 🔲 Epic E — Planned |
| Phase 6 | Testing & CI: integration tests, CI pipeline, hooks | 🔲 Epic F — Planned |
| Phase 7 | Code Quality: remove dead code, clean lint allows | 🔲 Epic G — Planned |
| Phase 8 | Release Readiness: LICENSE, completions, polish | 🔲 Epic H — Planned |

# Critical Path — Stage 4 v0.9.0

## Active Backlog Summary

- **Total Active Story Points:** 5
- **Active Epics:** H (RELS, 5 SP)
- **Completed:** Epic A (Foundation) — 9 points, Epic B (Pipeline) — 16 points, Epic C (DX) — 10 points, Epic D (SAFE) — 16 points, Epic E (SCAFF) — 7 points, Epic F (TEST) — 8 points, Epic G (CLEAN) — 9 points = 75 total delivered
- **Critical Path:** RELS-H001–H004

RELS-H001–H004 are independent of each other.

- **Parallel Windows:** RELS-H001–H004 (4 tickets)

## Build Order Diagram

```mermaid
flowchart LR
  CLEAN_DONE["✅ CLEAN Complete (epic gate)"]
  CLEAN_DONE --> RELS-H001["H RELS-H001-H004 Release"]
```

## Phasing Strategy

| Phase | Scope | Status |
|---|---|---|
| Phase 0–3 | Developer environment, Foundation, Pipeline, DX | ✅ Epics A–C — Completed |
| Phase 4 | Safety & Robustness: path security, signals, error quality | ✅ Epic D — Completed |
| Phase 5 | Scaffolding Enhancements: init flags, sample skill, init harness | ✅ Epic E — Completed |
| Phase 6 | Testing & CI: integration tests, CI pipeline, hooks | ✅ Epic F — Completed |
| Phase 7 | Code Quality: remove dead code, clean lint allows | ✅ Epic G — Completed |
| Phase 8 | Release Readiness: LICENSE, completions, polish | 🔲 Epic H — Active |

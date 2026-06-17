# Critical Path — Stage 4 v0.1.0

## Active Backlog Summary

- **Total Active Story Points:** 35
- **Critical Path:** FND-A001 → FND-A002 → FND-A003 → FND-A004 → PIPE-B001 → PIPE-B003 → PIPE-B004 → DX-C001 → DX-C002 → DX-C003
- **Parallel Window:** PIPE-B002 (Validator) may run concurrently with PIPE-B003 (Template Engine) — both depend on PIPE-B001.

## Build Order Diagram

```mermaid
flowchart LR
  FND-A001["FND-A001 Phase 0"] --> FND-A002["FND-A002 CLI Parsing"]
  FND-A002 --> FND-A003["FND-A003 Config & Skill Loading"]
  FND-A003 --> FND-A004["FND-A004 Harness Registry + Builtins"]
  FND-A004 --> PIPE-B001["PIPE-B001 Harness Resolution"]
  PIPE-B001 --> PIPE-B002["PIPE-B002 Validator"]
  PIPE-B001 --> PIPE-B003["PIPE-B003 Template Engine"]
  PIPE-B002 --> PIPE-B004["PIPE-B004 Output Router"]
  PIPE-B003 --> PIPE-B004
  PIPE-B004 --> DX-C001["DX-C001 Diff Preview"]
  DX-C001 --> DX-C002["DX-C002 Force Flag + Error UX"]
  DX-C002 --> DX-C003["DX-C003 Docs + README"]
```

## Phasing Strategy

| Phase | Scope | Status |
|---|---|---|
| Phase 0 | Developer environment (devenv, crate skeleton) | Included in Epic A (FND-A001) |
| Phase 1 | Foundation: CLI, Loader, Harness Registry | Epic A — Active |
| Phase 2 | Pipeline: Validator, Template Engine, Output Router | Epic B — Active |
| Phase 3 | DX: Diff preview, error UX, documentation | Epic C — Active |
| Future | Scaffolding (SC-1, SC-2 — P1) | Deferred |

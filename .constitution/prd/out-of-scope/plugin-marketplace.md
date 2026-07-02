# Out of Scope: Plugin Marketplace Integration

> **\[REOPENED 2026-07-02\]** — Operator directive (Epic I activation) reopens this non-goal. The distribution CLI (`add` / `list` / `remove` / `update`) is now in scope per `.constitution/tasks/active/EPIC-I-distribution.md`; the marketplace-specific facets below (storefronts, publishing, discovery) remain deferred. A full PRD revision lifting this file out of `out-of-scope/` is tracked as a downstream follow-up in `prd/changelog.md` (v0.2.0). Until that revision lands, treat the original "out of scope" status as the marketplace-specific scope and the distribution CLI commands as the in-scope operator-approved expansion.

**Context:** Determined out of scope during initial scope definition.

**Reasoning:** Marketplace distribution, publishing, and discovery are distribution-layer concerns that depend on platform-specific storefronts and review processes. skillprism's responsibility ends at generating correct files on disk. Integration with distribution channels belongs to a separate product scope or a future evolution.

> **\[Footer — In scope as of 2026-07-02, see banner above\]** — The distribution CLI commands (`add` / `list` / `remove` / `update`) introduced by Epic I are the **in-scope** portion of this file's original scope. Only the marketplace-specific facets (storefronts, publishing, discovery) remain deferred. A future PRD revision will either move this file out of `out-of-scope/` or split it into an in-scope and an out-of-scope file; until then, the `[REOPENED 2026-07-02]` banner at the top is the canonical record.

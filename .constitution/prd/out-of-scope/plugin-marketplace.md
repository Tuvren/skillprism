# Out of Scope: Plugin Marketplace Integration

> **\[REOPENED 2026-07-02\]** — Operator directive (Epic I activation) reopens this non-goal. The distribution CLI (`add` / `list` / `remove` / `update`) is now in scope per `.constitution/tasks/active/EPIC-I-distribution.md`; the marketplace-specific facets below (storefronts, publishing, discovery) remain deferred. A full PRD revision lifting this file out of `out-of-scope/` is tracked as a downstream follow-up in `prd/changelog.md` (v0.2.0). Until that revision lands, the canonical record is this banner: the original "out of scope" status applies to the marketplace-specific facets only; the distribution CLI commands are the in-scope operator-approved expansion.

**Context:** Determined out of scope during initial scope definition.

**Reasoning:** Marketplace distribution, publishing, and discovery are distribution-layer concerns that depend on platform-specific storefronts and review processes. skillprism's responsibility ends at generating correct files on disk. Integration with distribution channels belongs to a separate product scope or a future evolution.

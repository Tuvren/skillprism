# Changelog — Stage 1 (PRD)

### v0.2.0 — 2026-07-02 — Epic I Activation + Constraints Amendment

- Reopened `out-of-scope/plugin-marketplace.md` by operator directive (Epic I activation).
  - The distribution CLI capabilities (`add` / `list` / `remove` / `update` for skill sources) are now in scope; the marketplace-specific facets (storefronts, publishing, discovery) remain deferred.
  - `out-of-scope/plugin-marketplace.md` carries a `[REOPENED 2026-07-02]` banner explaining the partial-reopen status.
  - Full PRD revision (capability additions, glossary updates, fresh out-of-scope re-categorization) is a downstream follow-up; this entry is the canonical record of the operator's directive until that revision lands.
- **Amended `constraints.md` Binary Distribution section** to allow the `git` binary as a documented runtime dependency for the `add` and `update` distribution commands only.
  - The amendment is a focused exception: the `build`, `validate`, `init`, and `completions` commands remain purely static-binary with no runtime dependencies.
  - Rationale: spike DIST-I001 (`.constitution/spikes/SPK-DIST-I001.md`) recommended shelling out to `git` directly, matching Vercel's two-year production track record and avoiding the ~500KB binary hit of a native HTTP client. The amendment is the smallest change that unblocks the network layer.
  - `git` is assumed to be present on the user's PATH; the CLI verifies at startup and surfaces a clear, actionable error if missing.

### v0.1.2 — 2026-06-18

- Fixed operator preference appendix: "Clap v5" → "Clap v4.6" to match actual dependency

## v0.1.0 — 2026-06-17

- Initial PRD established from operator interview and project analysis.
- Vision, glossary, actors, capabilities, constraints, domain model, and out-of-scope items defined.
- Technology preferences captured in vision.md appendix.

### v0.1.1 — 2026-06-17

- Corrected BD-1/BD-2 default output scope from dist/ to project-level agent paths (deploy-first model).
- Updated OB-1 to reference diff preview against target paths instead of --target deploy flag.
- Updated constraints.md safety rules to remove stale --target deploy reference and generalize overwrite protection.

# Spike Report: DIST-I001 Remote Fetch Methodology

## 1. Context & Objective
- **Triggering upstream file/section:** `.constitution/prd/out-of-scope/plugin-marketplace.md` (reopened by operator directive — skillprism expands from build-time compiler to distribution CLI)
- **Target:** Determine how `skillprism add` should fetch skill sources from remote repositories, following learnings from Vercel's `skills` CLI (https://github.com/vercel-labs/skills) source code.

## 2. Codebase Baseline
- **Current State:** skillprism has no network layer. All existing commands (`build`, `validate`, `init`, `completions`) operate on local files only. The constitution's `strategy.md` states "no network, no daemon, no IPC" and `constraints.md` states "single static binary, no runtime deps." ADR-004 mandates a synchronous pipeline (no async runtime).
- **Discovered Constraints:**
  - ADR-004 (synchronous) — any HTTP client must be blocking, not async
  - ADR-003 (single crate) — new functionality lives in `src/` submodules
  - ADR-005 (atomic writes) — downloaded files must be written atomically
  - `constraints.md` — no telemetry; no silent fallbacks
  - The existing `TargetScope` (project/user/dist) and `HarnessPaths` (project_scope_path/user_scope_path) are the install targets

## 3. Options & Trade-offs
- **Option A: Git clone (shell out to `git`)** — Zero new Rust deps. Works with any git URL (GitHub, GitLab, private, SSH). Requires `git` on PATH. Consistent with Vercel's approach if they also shell out. Stays synchronous. Risk: depends on external `git` binary being installed and the right version.
- **Option B: Native HTTP (`ureq` + `rustls`)** — Adds ~2 Rust deps (~500KB binary). Can fetch tarballs/zipballs via GitHub APIs without git. Enables future HTTP registry. Still synchronous. Risk: TLS complexity, larger binary, API rate limits.
- **Option C: Hybrid** — Git clone for repos now, add `ureq`+`rustls` only when a directory/registry API is built later. Smallest dep surface now.
- **Vercel's actual method (preliminary, to be confirmed by reading source):** — Quick scan of `vercel-labs/skills` `src/git.ts` indicates Vercel uses `simple-git` (a Node wrapper that shells out to the `git` binary) with a `gh` CLI fallback for SSO-blocked HTTPS and an SSH retry path for auth errors. **TODO during spike:** confirm the full sub-path (caching policy, shallow-clone defaults, auth precedence) by reading the actual `src/` tree end-to-end, and verify that no `gh` runtime dependency is required when `gh` is not installed.

## 4. Execution Directives
- **Chosen Option:** (To be determined by spike research — fill in after reading Vercel's source)
- **Why it fits:** (To be filled after research)
- **Downstream Backlog Impact:** Unlocks DIST-I002 (state tracking layer) and DIST-I003 (`add` command implementation). No implementation tickets may proceed until this spike is complete.

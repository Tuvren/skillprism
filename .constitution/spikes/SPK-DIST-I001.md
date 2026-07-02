# Spike Report: DIST-I001 Remote Fetch Methodology

## 1. Context & Objective
- **Triggering upstream file/section:** `.constitution/prd/out-of-scope/plugin-marketplace.md` (reopened by operator directive — skillprism expands from build-time compiler to distribution CLI)
- **Target:** Determine how `skillprism add` should fetch skill sources from remote repositories, following learnings from Vercel's `skills` CLI (https://github.com/vercel-labs/skills) source code.

## 2. Codebase Baseline
- **Current State:** skillprism has no network layer. All existing commands (`build`, `validate`, `init`, `completions`) operate on local files only. The constitution's `.constitution/architecture/strategy.md` (line 24) states "no network, no daemon, no IPC" and `.constitution/prd/constraints.md` states "single static binary, no runtime deps." ADR-004 mandates a synchronous pipeline (no async runtime).
- **Discovered Constraints:**
  - ADR-004 (synchronous) — any HTTP client must be blocking, not async
  - ADR-003 (single crate) — new functionality lives in `src/` submodules
  - ADR-005 (atomic writes) — downloaded files must be written atomically
  - `constraints.md` — no telemetry; no silent fallbacks
  - The existing `TargetScope` (project/user/dist) and `HarnessPaths` (project_scope_path/user_scope_path) are the install targets

## 3. Options & Trade-offs
- **Option A: Git clone (shell out to `git`)** — Zero new Rust deps. Works with any git URL (GitHub, GitLab, private, SSH). Requires `git` on PATH. Stays synchronous. Risk: depends on external `git` binary being installed and the right version, **and** conflicts with `.constitution/prd/constraints.md` "single static binary, no runtime deps" (the spike must resolve this — either bias toward Option B or propose a `constraints.md` amendment as a downstream PRD task).
- **Option B: Native HTTP (`ureq` + `rustls`)** — Adds ~2 Rust deps (~500KB binary). Can fetch tarballs/zipballs via GitHub APIs without git. Enables future HTTP registry. Still synchronous. No external command dependency (compatible with `constraints.md`). Risk: TLS complexity, larger binary, API rate limits.
- **Option C: Hybrid** — Git clone for repos now, add `ureq`+`rustls` only when a directory/registry API is built later. Smallest dep surface now.

## 3.1 Hypothesis (to be tested by the spike, not a finding)

A preliminary scan of `vercel-labs/skills` `src/git.ts` *suggests* Vercel uses `simple-git` (a Node wrapper that shells out to the `git` binary) with a `gh` CLI fallback for SSO-blocked HTTPS and an SSH retry path for auth errors. **This is a starting hypothesis, not a conclusion.** The spike MUST read the actual `vercel-labs/skills` `src/` tree end-to-end and confirm or contradict it (caching policy, shallow-clone defaults, auth precedence, whether `gh` is a hard runtime dependency). The recommendation in §4 must be grounded in the confirmed finding, not the hypothesis.

## 4. Execution Directives
- **Chosen Option:** (To be determined by spike research — fill in after reading Vercel's source)
- **Why it fits:** (To be filled after research)
- **Downstream Backlog Impact:** Unlocks DIST-I002 (state tracking layer) and DIST-I003 (`add` command implementation). No implementation tickets may proceed until this spike is complete. If the chosen option requires amending `.constitution/prd/constraints.md` (e.g., to allow a documented external `git` dependency), the spike MUST also propose the amendment text and flag it as a Stage 1 (PRD) follow-up; the implementation PR cannot land until the PRD is updated.

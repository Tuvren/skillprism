# ADR-008: Network Layer for Distribution

**Status:** Accepted

## Context

Epic I (Distribution CLI) introduces the `add` and `update` commands, which fetch skill sources from remote repositories and require network access. This conflicts with two upstream decisions:

- `.constitution/prd/constraints.md` "single static binary with no runtime dependencies" (now amended in v0.2.0 to allow `git` for distribution commands only).
- `.constitution/architecture/strategy.md` line 24 "No network, no daemon, no IPC" (now amended in v0.2.2 to allow network access for distribution commands only).

The spike at `.constitution/spikes/SPK-DIST-I001.md` was conducted to determine the fetch methodology. The spike grounded its recommendation in the actual implementation of Vercel's `skills` CLI (https://github.com/vercel-labs/skills) and concluded that shelling out to `git` directly is the right approach: it matches Vercel's two-year production track record, adds zero new Rust dependencies, and supports the full git URL space (private SSH-key repos, GitLab, self-hosted, refs, subpaths).

## Decision

The `add` and `update` commands fetch remote sources by shelling out to the `git` binary. The fetch layer is implemented in `src/distribution/network.rs` as a thin wrapper around `std::process::Command`.

**Primary invocation:**

```
git clone --depth 1 [--branch <ref>] [--single-branch] <url> <tempdir>
```

**Environment variables set on the child process:**

- `GIT_TERMINAL_PROMPT=0` — disables interactive password prompts.
- `GIT_SSH_COMMAND=ssh -o BatchMode=yes` — SSH fails fast on missing keys (no interactive password prompts).
- `GIT_LFS_SKIP_SMUDGE=1` — skips LFS filtering (skills are plain text; LFS-tracked files are out of scope for v1).

**Auth chain (three layers, GitHub HTTPS only):**

1. **`git clone`** with the env above. Uses git's own credential resolution (SSH agent, `~/.netrc`, git credential helper, etc.).
2. **`gh repo clone` fallback** if the primary fails with an auth error. Non-blocking probe of `gh auth status -h github.com`; skipped if `gh` is not on PATH or not authenticated.
3. **SSH fallback** if both above fail. Retry with `git@github.com:owner/repo.git` and `BatchMode=yes`.

For non-GitHub hosts (GitLab, self-hosted), only layer 1 is used. The auth chain is GitHub-specific because `gh` is a GitHub-only tool.

**Auth error message** (modeled after Vercel's `buildGitHubAuthError` in `vercel-labs/skills` `src/git.ts:120-145`): when all three layers are exhausted, the error tells the user exactly what to do — "Re-authorize your GitHub credentials/app for that org's SSO policy / Or retry with SSH: skillprism add git@github.com:owner/repo.git / Check `gh auth status` or `ssh -T git@github.com`."

**Other defaults:**

- Default clone timeout: 5 minutes (configurable via env var — name decided in implementation PR).
- Temp dir: `std::env::temp_dir()` with a `skillprism-` prefix (decided in implementation PR); cleaned up on error or after use, with a safety check that the dir is within `temp_dir()` before deletion (defense against `cleanupTempDir` being called with arbitrary paths).
- LFS handling: skipped defensively. Skills are plain text; LFS-tracked files are not a v1 concern.

## Consequences

- **Positive:**
  - Matches Vercel's battle-tested three-layer auth chain (Vercel: `src/git.ts`; same pattern).
  - No new Rust dependencies (the `git` binary does the TLS, auth, protocol, disk I/O).
  - Supports the full git URL space: private SSH-key repos, GitLab, self-hosted, refs (branch/tag/SHA), subpaths.
  - Honors ADR-003 (single crate — new code in `src/distribution/network.rs`).
  - Honors ADR-004 (synchronous — `Command::status` is blocking, no async runtime).
  - Honors ADR-005 (atomic writes — rendered output is written via the existing `Router::write`; plain-format assets are copied via the existing `copy_assets` helper at `src/router/write.rs:34`; the state file uses the same temp-rename pattern. `Router::write` is the rendered-output writer, not a unified write path; copying a file is not a render).
  - Binary stays small (~500KB saved vs `ureq` + `rustls`).
- **Negative:**
  - Adds a documented external dependency on `git` for the `add`/`update` commands only.
  - The `gh` CLI fallback requires another optional binary on PATH.
  - Vercel's two-file JSON lock design is intentionally not copied; we use a single YAML file (see `.constitution/spikes/SPK-DIST-I001.md` §4.4).
- **Mitigation:**
  - The `git` dependency is documented in the README and the `init`/`build` commands remain purely static-binary.
  - The `gh` fallback is non-blocking and degrades gracefully if `gh` is missing.
  - The `find` command (deferred to a future epic) would be the natural place to revisit the auth chain for directory/registry use cases.

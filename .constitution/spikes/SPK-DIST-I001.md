# Spike Report: DIST-I001 Remote Fetch Methodology

## 1. Context & Objective

- **Triggering upstream file/section:** `.constitution/prd/out-of-scope/plugin-marketplace.md` (reopened by operator directive — skillprism expands from build-time compiler to distribution CLI).
- **Target:** Determine how `skillprism add` should fetch skill sources from remote repositories, grounded in the actual implementation of Vercel's `skills` CLI at https://github.com/vercel-labs/skills.
- **Outcome:** A recommended fetch methodology for skillprism, justified against `.constitution/prd/constraints.md` and `.constitution/architecture/strategy.md`, with the implementation tickets in Epic I pinned to the spike's findings.

## 2. Codebase Baseline

- **skillprism today:** Single static binary, no runtime dependencies, synchronous pipeline. All existing commands (`build`, `validate`, `init`, `completions`) operate on local files only. ADR-003 (single crate), ADR-004 (synchronous), ADR-005 (atomic writes).
- **Constraints in force (before this spike):** `.constitution/prd/constraints.md` "single static binary with no runtime dependencies"; `.constitution/architecture/strategy.md` line 24 "No network, no daemon, no IPC."
- **Existing target infrastructure:** `TargetScope` (project/user/dist) and `HarnessPaths` (project_scope_path/user_scope_path) are the install targets. The `find_template_path` helper at `src/loader/project.rs:121` returns one of `SKILL.md.j2`, `SKILL.md`, or `None` — the canonical source for template detection.

## 3. Vercel's actual implementation (confirmed by reading source)

Every claim below is grounded in the file paths cited at https://github.com/vercel-labs/skills (main branch).

### 3.1 Fetch layer (`src/git.ts`, 222 lines)

- **Primary path:** `simple-git` (a Node wrapper around the `git` binary) called with `['--depth', '1', '--branch', ref]` flags. The simple-git client configures `GIT_TERMINAL_PROMPT=0` (disable interactive password prompts) and `GIT_LFS_SKIP_SMUDGE=1` plus `filter.lfs.{required,smudge,clean,process}=false/empty/empty/empty` to avoid the `git-lfs filter-process: command not found` failure mode when git-lfs is not installed. See `src/git.ts:60-95`. The skillprism analog of Vercel's per-clone `config: [...]` block is `GIT_CONFIG_COUNT=4` plus `GIT_CONFIG_KEY_0..3` / `GIT_CONFIG_VALUE_0..3` env vars on the child process — the same overrides applied via git's env-var interface, which keeps the temp-dir lifecycle simple (no per-clone config file to clean up). The skillprism implementation MUST apply all four LFS filter overrides; relying on `GIT_LFS_SKIP_SMUDGE=1` alone is insufficient because that env var has no effect when git-lfs is not installed, and a source repo with LFS-tracked files anywhere in the tree (e.g. binary assets alongside skills) will fail to clone on minimal images. Vercel's empirical fix for this is `heygen-com/hyperframes#407`; skillprism inherits the same defense.
- **GitHub HTTPS auth fallback A — `gh repo clone`:** on auth failure, the code calls `tryGhClone` (`src/git.ts:97-118`) which probes `gh auth status -h github.com`, then runs `gh repo clone <slug> <dir> -- --depth=1 [--branch <ref>]`. The `gh` CLI is **optional** — `tryGhClone` returns `false` if the `gh auth status` probe fails, and the code falls through to SSH.
- **GitHub HTTPS auth fallback B — SSH:** on `tryGhClone` failure, the code retries with `git@github.com:owner/repo.git` and `BatchMode=yes` (`src/git.ts:148-158`). `BatchMode=yes` means SSH fails fast on missing keys — no interactive password prompts.
- **Default clone timeout:** 5 minutes, configurable via `SKILLS_CLONE_TIMEOUT_MS` env var (`src/git.ts:5-13`).
- **Temp dir:** `os.tmpdir()` with prefix `skills-` via `fs.mkdtemp`. `cleanupTempDir` validates the dir is within `tmpdir()` before `rm -rf` to prevent deletion of arbitrary paths (`src/git.ts:178-189`).
- **Auth error message (`src/git.ts:120-145`):** when the auth chain is exhausted, the error tells the user exactly what to do — "Re-authorize your GitHub credentials/app for that org's SSO policy / Or rerun with SSH."

### 3.2 Source URL parser (`src/source-parser.ts`, 340 lines)

Vercel accepts many input forms; the parser produces a `ParsedSource` enum (`src/types.ts:75-82`) with `type: 'github' | 'gitlab' | 'git' | 'local' | 'well-known'`, `url`, optional `subpath`, `ref`, `skillFilter`. Supported forms:

- **Local paths:** absolute, `./`, `../`, `.`, `..`, Windows (`C:\`)
- **Prefix shorthand:** `github:owner/repo`, `gitlab:owner/repo` (expanded to `https://gitlab.com/...`)
- **GitHub URLs:** full, `.../tree/<ref>` (branch), `.../tree/<ref>/<subpath>` (branch + subpath)
- **GitLab URLs:** full, `.../-/tree/<ref>[/<subpath>]` (the `/-/tree/` pattern is GitLab-specific), including self-hosted GitLab with subgroups
- **Shorthand:** `owner/repo`, `owner/repo/<subpath>`, `owner/repo@<skill-filter>`
- **Fragment ref:** `owner/repo#<ref>`, `owner/repo#<ref>@<skill>` (ref + skill filter combined; only honored for git-like sources)
- **Well-known URLs:** any HTTP(S) URL that isn't GitHub/GitLab is checked for `/.well-known/agent-skills/index.json` (with `/.well-known/skills/index.json` as fallback) — the future-extensibility hook
- **Source aliases:** a `SOURCE_ALIASES` map (e.g., `coinbase/agentWallet` → `coinbase/agentic-wallet-skills`)
- **Fallback:** direct git URL

### 3.3 State tracking — TWO separate JSON lock files

Vercel uses two distinct files, not one:

| File | Scope | Path | Schema | Hash strategy |
|---|---|---|---|---|
| `src/skill-lock.ts` (`.skill-lock.json`, v3) | **Global** (user) | `$XDG_STATE_HOME/skills/.skill-lock.json` → fallback `~/.agents/.skill-lock.json` | JSON, versioned; per-skill fields: `source`, `sourceType`, `sourceUrl`, `ref?`, `skillPath?`, `skillFolderHash`, `installedAt`, `updatedAt`, `pluginName?` | **GitHub tree SHA** (server-side, fetched via Trees API) |
| `src/local-lock.ts` (`skills-lock.json`, v1) | **Local** (project, committed to git) | `./skills-lock.json` | JSON, versioned; per-skill fields: `source`, `ref?`, `sourceType`, `skillPath?`, `computedHash`, `subagents?` | **SHA-256 of all files in skill folder** (client-side, sorted by relative path; includes path in hash so renames are detected) |

**Other Vercel state-file quirks:**
- No atomic rename — both files just `writeFile` directly
- No `0o700` mode on the directory; `mkdir` with no mode argument (relies on umask)
- Old-version files are **wiped** (not migrated): if `version < CURRENT_VERSION`, return empty lock and start fresh
- Local file is alphabetically sorted for git-merge friendliness
- Global file has extra `dismissed: { findSkillsPrompt?: boolean }` and `lastSelectedAgents: string[]` metadata

### 3.4 Install flow (`src/installer.ts`)

- `cleanAndCreateDirectory` — deletes and recreates the target directory on every install. Not atomic per-file.
- **Symlink mode (default):** writes to canonical location (`.agents/skills/<name>`) and symlinks to agent-specific locations (e.g., `.claude/skills/<name>`). Symlink failures fall back to copy. Universal agents skip the symlink (write to canonical only).
- **Copy mode (`--mode copy`):** writes directly to each agent's dir without symlinks.
- Per-harness detection: walks the source for skill dirs, parses frontmatter to extract `name` and `description`, sanitizes names to kebab-case (rejects path traversal).

### 3.5 Update flow (`src/update-source.ts`)

- `update` rebuilds the source input string (preserving `ref` and `skillPath`) and re-invokes `add` with it.
- The install flow's `cleanAndCreateDirectory` handles "delete and recreate" semantics — the dir is fully re-built on each install/update.
- Per-file change detection is **not** done; the whole target is re-written.
- `fetchSkillFolderHash` (in `src/skill-lock.ts:185-194`) calls GitHub's Trees API to get a server-side hash, used for change detection in the global lock.

## 4. Decision: skillprism's fetch methodology

### 4.1 Fetch method: shell out to `git` directly (Option A, per operator decision)

skillprism's `add` and `update` will use `std::process::Command::new("git")` directly:

```
git clone --depth 1 [--branch <ref>] [--single-branch] <url> <tempdir>
```

**Rationale:**
- Matches Vercel's battle-tested primary path; two-year production track record.
- Honors ADR-003 (single crate — new code in `src/distribution/network.rs`), ADR-004 (synchronous — `Command::status` is blocking), ADR-005 (atomic writes — downloaded files written via the existing `Router::write` infrastructure).
- No new Rust dependencies.
- Supports the full git URL space (private SSH-key repos, GitLab, self-hosted, refs, subpaths, partial clones).
- Binary stays small; no TLS code we have to maintain.

### 4.2 Auth: Vercel parity — three-layer chain (per operator decision)

- **Layer 1 — `git clone`:** uses git's own credential resolution. Env: `GIT_TERMINAL_PROMPT=0` (disable interactive prompts), `GIT_SSH_COMMAND=ssh -o BatchMode=yes` (SSH fail-fast), `GIT_LFS_SKIP_SMUDGE=1` (skip LFS).
- **Layer 2 — `gh repo clone` fallback (GitHub HTTPS only):** non-blocking probe of `gh auth status -h github.com`. If `gh` is installed and authenticated, retry the clone via `gh`. If the probe fails (binary not installed, not authenticated), skip to Layer 3.
- **Layer 3 — SSH fallback (GitHub HTTPS only):** retry with `git@github.com:owner/repo.git` and `BatchMode=yes`.
- **Auth error message** (modeled after Vercel's `buildGitHubAuthError` in `src/git.ts:120-145`): when all three layers fail, surface a clear, actionable error telling the user to (a) re-authorize for SSO, (b) retry with the SSH form, (c) check `gh auth status` or `ssh -T git@github.com`.
- **Other env defaults:** default clone timeout 5 minutes (configurable via env var decided in implementation PR); temp dir `std::env::temp_dir()` with prefix (decided in implementation PR).

### 4.3 Source URL scope: Vercel parity (per operator decision)

All seven forms from Vercel's parser are in v1 scope:

| Form | v1 | Notes |
|---|---|---|
| Local paths | ✅ | Used by integration test fixture and users with checked-out skills. |
| `github:owner/repo`, `gitlab:owner/repo` | ✅ | Convenience prefixes. |
| Full GitHub/GitLab URLs (incl. self-hosted GitLab with subgroups) | ✅ | The common case. |
| `owner/repo`, `owner/repo/<subpath>`, `owner/repo@<skill>` | ✅ | The single-skill shorthand. |
| `owner/repo#<ref>`, `owner/repo#<ref>@<skill>` | ✅ | Ref + skill filter. |
| `.well-known/agent-skills/index.json` discovery | ✅ | Vercel-parity future-extensibility hook. |
| `SOURCE_ALIASES` map | ✅ | Vercel-parity shorthand map. |

The `ParsedSource` enum has variants `GitHub { url, ref, subpath, skill_filter }`, `GitLab { url, ref, subpath, skill_filter }`, `Git { url, ref }`, `Local { path }`, `WellKnown { url, index_path }` (in `src/distribution/source.rs`). Source alias map is configured in a small file (decided in implementation PR).

### 4.4 State tracking: one YAML file (per operator decision)

skillprism uses a single state file at `~/.config/skillprism/installed.yaml`:

- **Format:** YAML, schema-versioned (top-level `version: 1`).
- **Directory mode:** `0o700` (per-user only) — divergence from Vercel's umask-based mkdir, justified by `prd/constraints.md`'s safety section.
- **Resolution:** `XDG_CONFIG_HOME` with `~/.config/skillprism/` fallback (per the spike's decision to align with XDG conventions rather than Vercel's `XDG_STATE_HOME` + `~/.agents/` split).
- **Atomicity:** read-all / write-all via single temp-rename per ADR-005.
- **Per-skill fields** (modeled on Vercel's union of global+local fields, with skillprism-specific additions):

```yaml
version: 1
skills:
  - name: my-skill
    source: anthropics/skills@pdf            # input string the user passed
    sourceUrl: https://github.com/anthropics/skills.git
    sourceType: github                       # github | gitlab | git | local | wellknown
    ref: main                                # branch/tag/SHA at install time (null for local)
    skillPath: skills/pdf                    # subpath within the source (null if root)
    scope: project                           # project | user
    harnesses: [claude, opencode]
    format: skillprism                       # skillprism | plain
    installedAt: 2026-07-02T14:23:45Z
    updatedAt: 2026-07-02T14:23:45Z
    files:                                   # per-file records for change detection
      - path: .claude/skills/my-skill/SKILL.md
        hash: sha256:abc123...
      - path: .claude/skills/my-skill/references/api.md
        hash: sha256:def456...
```

- **Concurrency:** v1 does not support concurrent `add` calls in the same state directory. `flock` is out of scope for v1 — add it as a follow-up ticket if real-world usage demands it. The state module MUST document the single-writer limitation.

### 4.5 Change detection on `update`: per-file SHA-256 (per operator decision)

`update` re-fetches the source, re-renders/re-copies, computes SHA-256 of each output file, and compares against the `files` array in the state record. Only files whose hash differs from the stored hash are written (atomically per ADR-005). Strictly better than Vercel's whole-folder `cleanAndCreateDirectory` approach and implementable in skillprism because the build pipeline already has the content-equality infrastructure.

## 5. Constraint and strategy tension resolution

### 5.1 `prd/constraints.md` — focused exception (per operator decision)

The "no runtime dependencies" rule is amended to allow `git` for distribution commands only. See `.constitution/prd/constraints.md` (amended in this PR) and `.constitution/prd/changelog.md` v0.2.0.

### 5.2 `architecture/strategy.md` — focused exception inline (per operator decision)

Line 24 "No network, no daemon, no IPC" is amended to scope the exception to the distribution commands. See `.constitution/architecture/strategy.md` (amended in this PR) and `.constitution/architecture/changelog.md` v0.2.2.

### 5.3 `ADR-008: Network Layer for Distribution` (per operator decision)

The design is recorded as a formal ADR. See `.constitution/tech-spec/adrs/ADR-008-network-layer-for-distribution.md` (new in this PR) and `.constitution/tech-spec/changelog.md` v0.11.0.

## 6. Downstream Backlog Impact

The implementation tickets in Epic I (renumbered to DIST-I001–DIST-I007 in this PR) reference the spike for their contracts and mechanisms:

- **DIST-I001 — State Tracking Layer** — schema informed by §4.4.
- **DIST-I002 — `add` Command** — fetch method from §4.1, source parser from §4.3, auth from §4.2.
- **DIST-I003 — `list` Command** — reads from the state file per §4.4.
- **DIST-I004 — `remove` Command** — deletes files + state records per §4.4.
- **DIST-I005 — `update` Command** — re-fetches per §4.1, change-detects per §4.5.
- **DIST-I006 — Integration Tests** — uses a local fixture repo (no network in CI) per the existing test convention.
- **DIST-I007 — Docs and Website Updates** — document the new commands and the `git` dependency.

The constraints/strategy amendments and ADR-008 land in the same PR as this spike, so DIST-I002 (network layer) can begin implementation immediately after this PR merges.

## 7. Open questions (deferred to implementation PRs, not blocking)

- Source alias map content: implementation PR decides what aliases to register in v1.
- Env var name for the clone-timeout override: implementation PR picks a name consistent with the existing `SKILLS_*` conventions if any, or invents one.
- Temp dir prefix: implementation PR picks a prefix (e.g., `skillprism-`).
- `well-known` URL handling: how to merge the discovered skills into the install flow (single fetch of the index, then N renders; or fetch each skill individually). Implementation PR decides.

## 8. Spike closure

This spike is complete. The chosen fetch methodology is documented in §4, the constraint and strategy tensions are resolved in §5, and the downstream impact is enumerated in §6. The spike's output is a stable basis for the Epic I implementation tickets.

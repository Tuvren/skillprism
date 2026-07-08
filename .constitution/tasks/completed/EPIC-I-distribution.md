# Epic I — Distribution CLI

Acronym: **DIST** | Story Points: **37** (32 original + 5 Phase 2)

**Status:** ✅ Completed and archived (2026-07-04)

**Dependencies:** Epic H (RELS) — release artifacts must exist before the v1.0.0 tag is cut; PRD non-goal `plugin-marketplace.md` reopened by operator directive (see `.constitution/prd/out-of-scope/plugin-marketplace.md` for the `[REOPENED 2026-07-02]` annotation; full PRD revision is a downstream follow-up tracked in `prd/changelog.md`).

**Phase 2 note:** agent auto-detection (DIST-I010) is used as contextual information (e.g. a "Detected agents" hint in the interactive `add` prompt), not as a default selection. Users must explicitly choose which harnesses to install to.

**Operator directive (audit trail):** the directive that reopened `plugin-marketplace.md` and triggered Epic I is recorded in `prd/changelog.md` v0.2.0 (2026-07-02 entry) and `tasks/changelog.md` v0.11.0 (Epic I Activated and Specified entry). The git history of this PR is the source of truth; the changelog entries are the canonical pointer.

**Spike (prerequisite, COMPLETE):** `.constitution/spikes/SPK-DIST-I001.md` — the remote fetch methodology is decided. The spike recommends shelling out to `git` directly for shallow clones (§4.1), with a three-layer auth chain (§4.2) and a Vercel-parity source-URL parser (§4.3). All implementation tickets below reference the spike for the contracts and mechanisms they need.

**Upstream amendments (this PR):** the network layer required by Epic I is unblocked by three constitutional changes in the same PR:
- `prd/constraints.md` v0.2.0 — `git` is a documented runtime dep for distribution commands only.
- `architecture/strategy.md` v0.2.2 — network access is permitted for distribution commands only.
- `ADR-008: Network Layer for Distribution` (new) — design for the `add`/`update` fetch layer.

## Overview

Expand skillprism from a build-time compiler into a distribution CLI — a Vercel `skills` CLI competitor that adds per-harness templating and build-time validation to the same install/list/remove/update workflow.

**Operator directive:** skillprism's model replaces Vercel's single-generic-skill model with one where authors write skillprism MiniJinja skills and the CLI handles per-harness tailoring. The CLI consumes both skillprism-format sources (`skill.yaml` declaring `skillprism: '<version>'` + `SKILL.md` → render per-harness) and plain sources (no `skill.yaml` → copy as-is). Format is **declared by the manifest**, not inferred from file contents.

**Scope of this epic:**
- `add` — fetch from remote, auto-detect format, render/copy, write to target scope, record state
- `list` — show installed skills from state
- `remove` — delete installed skills + update state
- `update` — re-fetch latest, render/copy, update state
- **Interactive `add`** (GAP-A) — when no `--harnesses`/`--target` flags, prompt user for selections
- **npm launcher** (GAP-B) — thin npm package that downloads + execs the Release binary
- **Agent auto-detection** (GAP-C) — probe common agent paths when no config exists

**Deferred to future epics:**
- `find` — ecosystem skill search. Can be implemented by querying Vercel's `skills.sh` API directly (no registry needed). Deferred on implementation priority, not infrastructure availability.
- `use` — render to temp + launch agent (convenience command, adds process-launching complexity). Explicitly ruled out for now.
- **`.well-known` index installs** (DIST-I002 `WellKnown` form) — the source parser recognizes `https://example.com` index URLs, but `install_from_well_known` intentionally returns a clear "not supported yet" error. Full index-driven installs are deferred until a registry backend is available; the DIST-I002 Gherkin for well-known indexes is deferred with it.
<!-- - Harness coverage expansion — keep 5 built-in harnesses; grow via community contributions (`init harness` + `harnesses/<name>.yaml`) -->

**Key contracts (resolved by the spike):**
- **Fetch method:** `git clone --depth 1 [--branch <ref>] [--single-branch] <url> <tempdir>` via `std::process::Command`. No new Rust dependencies.
- **Auth handling:** `GIT_TERMINAL_PROMPT=0`, `GIT_SSH_COMMAND=ssh -o BatchMode=yes`, `GIT_LFS_SKIP_SMUDGE=1`. Three-layer chain: `git clone` → `gh repo clone` (GitHub HTTPS only) → SSH. Clear, actionable error on auth failure.
- **Source URL forms (v1):** local paths, `github:`/`gitlab:` prefixes, full GitHub/GitLab URLs (with `tree/<ref>` and `tree/<ref>/<subpath>`, self-hosted GitLab with subgroups), `owner/repo` shorthand, `owner/repo@<skill>` for skill filter, `owner/repo#<ref>` for git ref, `owner/repo#<ref>@<skill>` for ref+filter, `.well-known/agent-skills/index.json` discovery, source alias map.
- **State file:** YAML at `~/.config/skillprism/installed.yaml` (per the project's `yaml_serde` choice), mode 0o700, schema-versioned, atomic writes per ADR-005.
- **Change detection on `update`:** per-file SHA-256 of rendered/copied content, stored in the state file's per-skill `files` array. Only files whose hash differs from the stored hash are written.
- **Concurrency:** v1 of the state layer does not support concurrent `add` calls in the same state directory. The `flock` follow-up is not in scope.

---

#### DIST-I001 State Tracking Layer
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** None
- **Description:** Implement a system-wide state tracking layer in `~/.config/skillprism/` that records installed skills. The state file is `installed.yaml` (YAML, schema-versioned). The directory is created with mode `0o700` (per-user only) and resolved via `XDG_CONFIG_HOME` with `~/.config/skillprism/` fallback. On macOS, `~/.config/` is a non-standard path (Apple's HIG is `~/Library/Application Support/skillprism/`); skillprism follows the XDG convention because `XDG_CONFIG_HOME` is respected by enough cross-platform tooling to outweigh Apple-platform aesthetics, and a future ticket can layer in a macOS-specific path if it becomes a real complaint. A new `src/state/` module encapsulates read/write/query operations.

  **State schema (`installed.yaml`):**
  ```yaml
  version: 1
  skills:
    - name: my-skill
      source: anthropics/skills@pdf                   # the input string the user passed
      sourceUrl: https://github.com/anthropics/skills.git
      sourceType: github                              # github | gitlab | git | local | wellknown
      ref: main                                       # branch/tag/SHA at install time (null for local)
      resolvedRef: 9f2c...                            # (impl extension) concrete commit SHA HEAD resolved to; powers the `update` ls-remote no-op check (DIST-I005)
      skillPath: skills/pdf                           # subpath within the source (null if root)
      projectRoot: /path/to/project                   # (impl extension) project root captured at install; needed for update/remove path resolution (null for user scope)
      scope: project                                  # project | user
      harnesses: [claude, opencode]
      format: skillprism                              # skillprism | plain
      installedAt: 2026-07-02T14:23:45Z
      updatedAt: 2026-07-02T14:23:45Z
      files:                                          # per-file records for change detection
        - path: .claude/skills/my-skill/SKILL.md      # (impl) stored ABSOLUTE for project scope — resolved under `projectRoot`; the relative form here is illustrative
          hash: sha256:abc123...
        - path: .claude/skills/my-skill/references/api.md
          hash: sha256:def456...
  ```

  **File-path storage (implementation note):** `files[].path` is stored as the
  absolute resolved output path (for project scope, under `projectRoot`; for
  user scope, under `$HOME`). `remove`/`update` recompute prefixes from the same
  roots and match with `Path::starts_with`, so records are self-consistent. The
  tradeoff: `installed.yaml` is not portable across a moved project directory —
  relocating a project would strand its absolute `files[]` prefixes. Accepted for
  v1; normalizing to project-relative paths is a possible future refinement.

  **Field order and merge friendliness:** the top-level keys are emitted in declaration order (`version`, `skills`); the `skills[]` array is sorted alphabetically by `name`; per-record keys are emitted in declaration order. This matches Vercel's `src/local-lock.ts:80-85` pattern (alphabetical sort for deterministic output and clean git diffs) and prevents the implementation PR from picking a `serde` default that produces hard-to-merge diffs on concurrent updates.

  **File mode:** `installed.yaml` is created with mode `0o600` (owner read/write only) via an explicit `OpenOptions::create().write(true).mode(0o600)`, matching the directory's `0o700` per-user guarantee. The umask-default `0o644` is NOT acceptable — the file contains source URLs, refs, and per-file SHA-256 hashes of every installed skill and must be unreadable to other users on the box.

  **Atomicity model:** every mutation of the state file (add, remove, update) MUST read the entire `installed.yaml`, compute the new in-memory state, and rewrite the entire file via a single temp-rename (the same ADR-005 pattern the build pipeline uses for output files). Per-record atomicity is not enough — partial-state writes would split the file across records.

  **Concurrency note:** ADR-005's temp-rename protects against partial writes within a single process but does **not** protect against two concurrent `skillprism add` invocations — both readers will compute new state, both will rename, and the second writer silently clobbers the first. v1 of the state layer does not support concurrent `add` calls in the same state directory; the state module MUST document this limitation and MUST recommend running the CLI in a single-writer fashion (CI jobs serialize, humans don't background two `add`s in parallel). A `flock` is intentionally out of scope for v1 — add it as a follow-up ticket if real-world usage demands it.

  This is the foundation for `list`, `remove`, and `update` — no CLI commands are wired yet.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given a fresh environment with no ~/.config/skillprism/ directory
When the state layer is initialized
Then ~/.config/skillprism/ is created with mode 0o700 (per-user only)
And a state file (installed.yaml, schema version 1) is created empty
And the state layer's public API documents the v1 single-writer limitation (no concurrent `add` calls in the same state directory)

Given an installed skill record with name "my-skill", source "owner/repo", ref "abc123", scope "project", harnesses ["claude", "opencode"], format "skillprism", files [{path: ".claude/skills/my-skill/SKILL.md", hash: "sha256:abc"}]
When the record is written to the state file
Then the state file is atomically written (temp file + rename per ADR-005)
And the state file contains the complete record with all fields
And the state file is human-readable YAML

Given $XDG_CONFIG_HOME is set to /custom/config
When the state layer resolves the state directory
Then it uses /custom/config/skillprism/ instead of ~/.config/skillprism/

Given a fresh state file on macOS with $XDG_CONFIG_HOME unset
When the state layer resolves the state directory
Then it uses ~/.config/skillprism/ (XDG fallback)
And no ~/Library/Application Support path is consulted

Given the state file is freshly created (no installs yet)
When the file is read
Then it contains exactly:
  version: 1
  skills: []

Given the state file contains 3 installed skill records
When the state layer queries for all installed skills
Then it returns all 3 records with their full metadata

Given the state file contains a record for "my-skill"
When the state layer removes the record for "my-skill"
Then the state file no longer contains "my-skill"
And the state file is atomically rewritten
```

---

#### DIST-I002 `add` Command — Fetch, Auto-Detect, Render, Write
- **Type:** Feature
- **Effort:** 8
- **Dependencies:** DIST-I001, `.constitution/spikes/SPK-DIST-I001.md` (spike complete), `prd/constraints.md` v0.2.0, `architecture/strategy.md` v0.2.2, `ADR-008`
- **Description:** Implement the `skillprism add <source>` command. The source is one of the v1-scoped forms from spike §4.3 (local paths, `github:`/`gitlab:` prefixes, full GitHub/GitLab URLs, `owner/repo` shorthand with `@skill` and `#<ref>` variants, `.well-known/agent-skills/index.json` discovery, source alias map). The command parses the source, fetches the repo via the spike's mechanism, walks it for skill directories, and for each skill auto-detects the format.

  **Fetch mechanism (per spike §4.1 and ADR-008):** `git clone --depth 1 [--branch <ref>] [--single-branch] <url> <tempdir>` via `std::process::Command`. The child env sets `GIT_TERMINAL_PROMPT=0` (disable interactive prompts), `GIT_SSH_COMMAND=ssh -o BatchMode=yes` (SSH fail-fast), and `GIT_LFS_SKIP_SMUDGE=1` (skip LFS). Default clone timeout: 5 minutes (configurable via env var). Temp dir: `std::env::temp_dir()` with prefix; cleaned up on error. New code lives in `src/distribution/network.rs`.

  **Auth chain (per spike §4.2 and ADR-008):** on auth failure from the primary `git clone`, the code probes `gh auth status -h github.com` and, if `gh` is installed and authenticated, retries via `gh repo clone`. If that also fails, retries with `git@github.com:owner/repo.git` and `BatchMode=yes`. When all three layers are exhausted, surface a clear, actionable error modeled after Vercel's `buildGitHubAuthError`.

  **Source URL parser (per spike §4.3):** produces a `ParsedSource` enum with variants `GitHub { url, ref, subpath, skill_filter }`, `GitLab { url, ref, subpath, skill_filter }`, `Git { url, ref }`, `Local { path }`, `WellKnown { url, index_path }`. Implemented in `src/distribution/source.rs`. Unit-tested against every accepted form and several rejection cases.

  **Auto-detection (manifest-declared):** the discriminator is the `skill.yaml` manifest, not the template file's contents. The `skill.yaml` MUST carry a `skillprism:` field whose value is a non-empty version string (e.g., `skillprism: '1'`). The version is the manifest schema version; presence of the field is the declaration of skillprism-format. The marker heuristic (`{{` / `{%` detection) is not the discriminator — markers in the template are still allowed but do not determine format.

  **Format decision rules:**

  1. **`skill.yaml` absent** → the skill is `plain-format`. Copy template + assets as-is. No manifest = no skillprism claim.
  2. **`skill.yaml` present, `skillprism: '<non-empty-version>'` (e.g. `skillprism: '1'`)** → the skill is `skillprism-format`. Render per configured harness via the existing Load → Resolve → Validate → Render pipeline. The state record carries `format: skillprism`.
  3. **`skill.yaml` present, no `skillprism:` field** → malformed manifest. Surface a clear error: "skill.yaml is present but missing the `skillprism:` field; either add `skillprism: '1'` to declare skillprism-format, or remove `skill.yaml` to declare plain-format." Exit code 1 (runtime error per `src/cli.rs:131-137`).
  4. **`skill.yaml` present, `skillprism: ''` (empty)** → malformed manifest. Same error as case 3.
  5. **`skill.yaml` present, `skillprism:` is not a string** (e.g. `skillprism: true` or `skillprism: 1` without quotes) → malformed manifest. The field MUST be a quoted string. Same error as case 3.

  **Template handling (per skill directory, after format is determined):** the existing `find_template_path` from `src/loader/project.rs:121` is the canonical source-of-truth for which file is the template and MUST be reused, not re-implemented. It returns one of `SKILL.md.j2`, `SKILL.md`, or `None`. The `find_template_path` ambiguity check (`Both SKILL.md and SKILL.md.j2 exist` → `ProjectError::AmbiguousTemplate`) is preserved verbatim. The visibility fix (below) covers the `pub(crate)` lift.

  - `skillprism-format` → render per configured harness. The `skill.yaml` provides the variable context (top-level `variables:` and per-harness `harnesses.<harness>.variables`).
  - `plain-format` → copy the template bytes verbatim to each harness's output path (renamed to `SKILL.md` if the source was `SKILL.md.j2`), plus copy every direct subdirectory of the skill's source directory via the existing `discover_asset_dirs` / `copy_assets` helpers (the new module MUST reuse them, not re-implement — see visibility fix below). The state record carries `format: plain`.

  **Visibility fix:** `find_template_path` and `discover_asset_dirs` are currently module-private in `src/loader/project.rs`. The new `src/distribution/` module needs to call them. The implementation PR MUST lift them to `pub(crate)` in `src/loader/project.rs:121` and `src/loader/project.rs:257` and add explicit re-exports to `src/loader/mod.rs`. Because the items are `pub(crate)`, the explicit re-exports MUST be written as `pub(crate) use project::find_template_path;` and `pub(crate) use project::discover_asset_dirs;` — a literal `pub use` of a `pub(crate)` item would itself default to `pub(crate)` (the more restrictive of the two wins), so the implementation PR can type either, but the planning doc pins the explicit form for clarity. `src/loader/mod.rs` already contains `mod project;` and `pub use project::*;`; the lift adds the two explicit re-exports rather than starting from an empty file. This is the minimum-surface change — no free-function extraction needed. The `copy_assets` helper at `src/router/write.rs:34` is already `pub` and is the right reference for the asset-copy side.

  **Note on the `find_template_path` test citation:** the test at `src/loader/project.rs:406` (`load_valid_project_with_bare_skill_md`) verifies the loader maps a bare `SKILL.md` to the template path; it does not run the renderer. The MiniJinja render behavior is the engine's responsibility. The loader's role is to locate the template file; the format decision is made by reading `skill.yaml` (or its absence). DIST-I006 should add a test that invokes the engine on a `skillprism:`-declared skill with markers, to anchor the render path.

  The `--target` flag (default: project) controls where output is written. **`add` enforces `--target` at parse time via a restricted `InstallScope { Project, User }` enum** (not the full `TargetScope` enum used by `build`) — clap rejects `dist` with a parse error (exit code 2) before the command runs. The `--skill` flag filters which skills to install from a multi-skill repo. The `--harnesses` flag (`-H`, comma-separated) filters which harnesses to render to. The flag name reuses the existing `init project` / `init skill` convention for internal consistency, but the **default-value semantic follows `init skill`** ("all harnesses in `skillprism.yaml` or all built-in if no project config"), NOT `init project` (which defaults to `[claude, opencode]` per `src/cli.rs:108-109`). The implementation PR must NOT pick the `init project` default by accident. After writing, the command records each installed skill in the state tracking layer (per DIST-I001 schema). Overwrite confirmation applies per existing safety model unless `--force`.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given a skillprism-format skill repo with `skill.yaml` declaring `skillprism: '1'` and `SKILL.md`
When the user runs `skillprism add owner/repo`
Then the repo is fetched to a temporary directory via `git clone --depth 1`
And each skill directory's `skill.yaml` is read and the `skillprism:` field is consulted
And the skill is recognized as `skillprism-format` (manifest-declared)
And the skill is rendered once per configured harness (the `skill.yaml` provides the variable context)
And the rendered SKILL.md files are written to each harness's project scope path
And each installed skill is recorded in the state tracking layer with `format: skillprism`

Given a plain-format skill repo with only `SKILL.md` (no `skill.yaml`)
When the user runs `skillprism add owner/repo`
Then the repo is fetched to a temporary directory via `git clone --depth 1`
And each skill directory has no manifest, so it is recognized as `plain-format` (default)
And the SKILL.md is copied as-is to each harness's project scope path
And each installed skill is recorded in the state tracking layer with `format: plain`

Given a multi-skill repo with skills "alpha" and "beta"
When the user runs `skillprism add owner/repo --skill alpha`
Then only the "alpha" skill is installed
And "beta" is not installed

# Source form Gherkin (v1 — Vercel parity per spike §4.3)

Given a public GitHub repo accessible via the `github:owner/repo` prefix
When the user runs `skillprism add github:anthropics/skills`
Then the prefix is normalized to `https://github.com/anthropics/skills.git`
And the install proceeds the same as the `owner/repo` shorthand

Given a GitLab.com repo accessible via the `gitlab:owner/repo` prefix
When the user runs `skillprism add gitlab:mygroup/myskill`
Then the prefix is normalized to `https://gitlab.com/mygroup/myskill.git`
And the install proceeds

Given a public GitHub repo with a tree URL including branch and subpath
When the user runs `skillprism add https://github.com/owner/repo/tree/main/skills/pdf`
Then the URL is normalized to clone `https://github.com/owner/repo.git` with `--branch main` and the source-walk is scoped to the `skills/pdf` subpath

Given a self-hosted GitLab instance at `https://gitlab.example.com/`
When the user runs `skillprism add https://gitlab.example.com/team/project`
Then the install proceeds with `https://gitlab.example.com/team/project.git` — the auth chain is **`git clone` only**: no `gh` fallback (the `gh` CLI is GitHub-only) and no SSH fallback (the SSH retry in the auth chain is also GitHub-only). Skillprism relies on the user's own git credential resolution (SSH agent, `~/.netrc`, git credential helper) for the GitLab case.

Given the source `owner/repo` and a `ref` fragment
When the user runs `skillprism add owner/repo#v1.2.3`
Then the install fetches the tag `v1.2.3` (not the default branch) and records `ref: v1.2.3` in the state

Given the source `owner/repo` and a `ref` and `skill` fragment
When the user runs `skillprism add owner/repo#main@pdf`
Then the install fetches `main` and filters to the `pdf` skill only (same as `--skill pdf`)

Given a `.well-known/agent-skills/index.json` endpoint published at `https://example.com/.well-known/agent-skills/index.json`
When the user runs `skillprism add https://example.com`
Then the index is fetched and the listed skills are installed per the manifest

Given a source alias `coinbase/agentWallet` mapped to `coinbase/agentic-wallet-skills` in the alias map
When the user runs `skillprism add coinbase/agentWallet`
Then the alias is resolved to `coinbase/agentic-wallet-skills` and the install proceeds

Given a source `unknown/repo` that has no entry in the alias map
When the user runs `skillprism add unknown/repo`
Then the parser falls through to the `owner/repo` shorthand path (no special "unknown alias" error — an unknown alias is indistinguishable from a regular shorthand)

Given a source `   ` (whitespace only) entered as an alias
When the user runs `skillprism add '   '`
Then the command fails with a clear error: "source cannot be empty or whitespace"
And the exit code is 2 (usage error)

Given a repo with skills targeting claude and opencode
When the user runs `skillprism add owner/repo --harnesses claude`
Then skills are only rendered/copied to .claude/skills/
And .opencode/skills/ is not written

Given a `skill.yaml` present but missing the `skillprism:` field
When the user runs `skillprism add owner/repo --skill that-skill`
Then the command fails with a clear error: "skill.yaml is present but missing the `skillprism:` field; either add `skillprism: '1'` to declare skillprism-format, or remove `skill.yaml` to declare plain-format."
And no files are written
And the state tracking layer is not modified
And the exit code is 1

Given a `skill.yaml` with an empty `skillprism:` value (`skillprism: ''`)
When the user runs `skillprism add owner/repo --skill that-skill`
Then the command fails with the same malformed-manifest error
And the exit code is 1

Given a `skill.yaml` with a non-string `skillprism:` field (e.g. `skillprism: true` or `skillprism: 1` without quotes)
When the user runs `skillprism add owner/repo --skill that-skill`
Then the command fails with the malformed-manifest error naming the bad field type
And the exit code is 1

Given a fetched repo with no skill directories anywhere (no SKILL.md* reachable)
When the user runs `skillprism add owner/repo`
Then the command exits with code 1
And a clear error is printed to stderr naming the source and explaining that no skillprism-format or plain-format skills were found
And no files are written
And the state tracking layer is not modified

Given the user invokes `skillprism add` with --target dist
When the command runs
Then it exits with a non-zero status
And the error names `--target` and lists the valid values: project, user
And the state tracking layer is not modified

Given the user is in a directory with no skillprism.yaml (no project root)
When the user runs `skillprism add owner/repo --target user`
Then the command does not invoke the build-style `find_project_root()` resolution
And the skill is installed to ~/.config/<harness>/skills/ (the user scope) for each configured harness
And the installed skill is recorded in the state tracking layer with scope: user

Given the user is in a directory with no skillprism.yaml
When the user runs `skillprism add owner/repo` (default --target project)
Then the command fails with a clear error explaining that --target project requires being inside a project directory
And the error suggests using --target user
And the exit code is 2 (usage error)
<!-- SUPERSEDED by DIST-I008 (Phase 2): the no-flags path no longer silently defaults to `--target project`. With no `--target`, `add` now prompts interactively for scope (offering `user` when outside a project), or errors with actionable guidance on a non-TTY. The exit-2 usage error still applies to the explicit `--target project` outside-a-project path (add.rs `resolve_scope`) and to malformed sources (empty/whitespace). -->


Given a skill directory containing both SKILL.md and SKILL.md.j2 (ambiguous template)
When the user runs `skillprism add owner/repo --skill that-skill`
Then the command fails with ProjectError::AmbiguousTemplate (re-uses the existing loader rule from src/loader/project.rs)
And no files are written
And the state tracking layer is not modified

Given a plain-format skill directory containing only `SKILL.md` (no `skill.yaml`) and a `references/` subdirectory
When the user runs `skillprism add owner/repo --skill that-skill --harnesses claude`
Then that skill is recognized as `plain-format` (no manifest, default)
And the SKILL.md is copied as-is to .claude/skills/that-skill/SKILL.md
And the references/ directory tree is copied to .claude/skills/that-skill/references/ via the existing copy_assets helper
And the installed skill is recorded in the state tracking layer with `format: plain` and `harnesses: [claude]`

Given a bare `SKILL.md` that contains MiniJinja markers (e.g. `# {{ name }}`), with no `skill.yaml`
When the user runs `skillprism add owner/repo --skill that-skill`
Then that skill is recognized as `plain-format` (no manifest declares skillprism) — the markers in the file are **not consulted**
And the SKILL.md is copied as-is to each harness's project scope path (the markers will be visible in the output, which is the correct behavior for a plain copy)
And the installed skill is recorded in the state tracking layer with `format: plain`

Given an already-installed skill "my-skill" at the target scope
When the user runs `skillprism add owner/repo --skill my-skill` without --force
Then the user is prompted to overwrite (y/n/s/a)
And if the user declines, no files are written

Given a skillprism-format skill (manifest-declared) whose template references variables that are NOT defined in `skill.yaml`'s `variables:` block
When the user runs `skillprism add owner/repo`
Then the render fails with a validation error identifying the undefined variable
And no output files are written

Given a fetch failure (network error, invalid URL, repo not found, auth failure)
When the user runs `skillprism add invalid-source`
Then a clear error message is printed to stderr identifying the failure
And the actionable guidance is shown for auth failures (per spike §4.2)
And the exit code is 1

Given a source URL that is neither a v1-scoped source form nor a local path
When the user runs `skillprism add <url>`
Then the command fails with a clear error listing the supported source forms
And the exit code is 2 (usage error)

Given `git` is not on the user's PATH
When the user runs `skillprism add owner/repo`
Then the command exits with code 1
And a clear error is printed to stderr explaining that `git` is required and how to install it
```

---

#### DIST-I003 `list` Command
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I001
- **Description:** Implement the `skillprism list` command (alias: `ls`). Reads the state tracking layer and displays installed skills in a tab-separated table: name, source, ref (short SHA or branch name), format (skillprism|plain), scope, harnesses. (The `format` column is an implementation addition to the original five-column sketch; it is cheap, useful, and kept.) The `--target` flag filters by scope (project|user). The `--harnesses` flag (`-H`, comma-separated) filters by harness — same flag name as the existing `init` and `add` commands. Output goes to stdout (machine-parseable table); diagnostics to stderr per the stdout/stderr discipline. If no skills are installed, prints an empty-state notice ("No installed skills"). (Implementation note: per `guidelines.md` stdout-discipline, this status notice is emitted to **stderr** so piped stdout stays empty/clean — superseding the "to stdout" wording in the Gherkin below, exactly as the `is up to date` status is handled in DIST-I005.)
- **Acceptance Criteria (Gherkin):**
```gherkin
Given 3 skills installed across project and user scopes
When the user runs `skillprism list`
Then a table is printed to stdout showing all 3 skills with name, source, ref, format, scope, and harnesses

Given skills installed in both project and user scopes
When the user runs `skillprism list --target user`
Then only user-scoped skills are listed

Given skills installed for claude and opencode
When the user runs `skillprism list --harnesses claude`
Then only skills installed for the claude harness are listed

Given no skills installed
When the user runs `skillprism list`
Then "No installed skills" is printed to stderr (stdout stays empty per stdout-discipline)
And the exit code is 0
```

---

#### DIST-I004 `remove` Command
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I001
- **Description:** Implement the `skillprism remove [skills...]` command (alias: `rm`). Removes installed skills from the filesystem and the state tracking layer. The `--target` flag filters by scope. The `--harnesses` flag (`-H`, comma-separated) removes only from a specific harness's directory. The `--all` flag removes all installed skills. Interactive confirmation is shown unless `--force` is passed (the same flag name as the existing `build` command's skip-confirmation behavior; using `--force` for consistency, not `--yes`). Removal respects scope confinement (never deletes outside the determined scope path). After removing files, the state record is updated atomically per the DIST-I001 atomicity model.

  **Note on `--target dist`:** the `TargetScope::Dist` enum variant exists for `build --target dist` (writes to `./dist/` for inspection) but is not an install target — `add` and `remove` only operate on `project` and `user` scopes. The `--all` flag MUST therefore iterate `project` + `user` only, never `dist`; iterating `dist` would risk deleting files that were never installed and could nuke unrelated build output.

  **Known UX sharp edge:** `skillprism remove --all --force` removes every installed skill across both `project` and `user` scopes with no preview. The implementation PR MUST mitigate this — two reasonable contracts: (a) print the affected skills (name + scope) to stdout before deleting, and require the user to confirm with a final `y/N` prompt even when `--force` is set; or (b) require an additional `--all-scopes` flag to cross the project/user boundary (so `remove --all --force` defaults to the current scope, and `remove --all --force --all-scopes` is the explicit cross-scope variant). The planning doc pins the contract as one of (a) or (b); the implementation PR picks.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given an installed skill "my-skill" in project scope for claude and opencode
When the user runs `skillprism remove my-skill --force`
Then the skill files are deleted from .claude/skills/my-skill/ and .opencode/skills/my-skill/
And the state record for "my-skill" is removed
And the exit code is 0

Given an installed skill "my-skill" for claude and opencode
When the user runs `skillprism remove my-skill --harnesses claude --force`
Then only .claude/skills/my-skill/ is deleted
And .opencode/skills/my-skill/ remains
And the state record is updated to reflect only opencode

Given 3 installed skills in project scope and 2 in user scope
When the user runs `skillprism remove --all --force --all-scopes`
Then all 5 installed skills are removed from both project and user scopes
And the state file is empty

Given 3 installed skills in project scope and 2 in user scope
When the user runs `skillprism remove --all --force`
Then only the 3 project-scope skills are removed
And the 2 user-scope skills remain

Given the user invokes `skillprism remove` with --target dist
When the command runs
Then it exits with a non-zero status
And the error names `--target` and lists the valid values: project, user

Given an installed skill "my-skill"
When the user runs `skillprism remove my-skill` without --force
Then an interactive confirmation prompt is shown
And if the user declines, no files are deleted

Given a skill "not-installed" that is not in the state
When the user runs `skillprism remove not-installed`
Then a clear error is printed to stderr saying the skill is not installed
And the exit code is 1
```

---

#### DIST-I005 `update` Command
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** DIST-I001, DIST-I002, `.constitution/spikes/SPK-DIST-I001.md` (spike complete), `ADR-008`
- **Description:** Implement the `skillprism update [skills...]` command. For each named skill (or all if none named), re-fetches the source at the latest version (per the spike's fetch mechanism in DIST-I002), re-renders/re-copies, and decides whether each file needs writing via per-file SHA-256 comparison against the `files` array stored in the state record (per spike §4.5). Only files whose hash differs from the stored hash are written, via the same atomic-write infrastructure as the build pipeline. The existing `Router::diff` is the display-only diff renderer used solely for the `--diff` flag's user-facing output, not as the change test.

  If a skill is already at the latest version (same `ref` in the upstream), no action is taken and an "up to date" message is printed. The `--harnesses` flag (`-H`, comma-separated) restricts the update to a specific harness subset — same flag name as `init`, `add`, and `list`. The `--diff` flag shows what would change without writing. The `--force` flag skips confirmation (same flag name as `build`, `add`, and `remove`; using `--force` for consistency, not `--yes`). Update respects all safety models (atomic writes, scope confinement, overwrite confirmation) and writes only to `project` and `user` scopes (same `--target dist` rejection as `add` and `remove`).

  **"At the latest ref" check:** the upstream ref is established via `git ls-remote <url> <ref-spec>` — a lightweight, sub-second query that returns the SHA-1 the remote resolves the ref to, without cloning. The `git clone --depth 1` from DIST-I002 only runs when the SHA differs (or on first call, when no baseline exists). This keeps `update` cheap for the no-op case (which is the common case: most days, most skills are at the latest ref). The 5-minute timeout from the spike's fetch mechanism applies to the `git clone` path, not to the `ls-remote` query (which has a much shorter default timeout in `git` itself).

  **CLI flag consistency:** the `--diff` flag SHOULD have `dry-run` as a `visible_alias` (matching the existing `build --diff --dry-run` surface at `src/cli.rs:50`), so `skillprism update --dry-run` is a natural user expectation. The implementation PR is responsible for the alias; this planning doc just pins the intent.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given an installed skill "my-skill" at ref "abc123"
When a newer ref "def456" is available and the user runs `skillprism update my-skill --force`
Then a fresh clone of the latest source is fetched via `git clone --depth 1 --branch def456 <url> <tempdir>` (full re-clone, not `git fetch` against a prior checkout — the per-file SHA-256 + atomic-rename infrastructure from DIST-I001 is designed around a fresh temp dir)
And the new rendered output is computed per file
And the SHA-256 of each file is compared against the state record's `files` array
And only files whose hash differs are written atomically
And the state record's `files` array is updated with the new hashes and `ref` is updated to "def456"
And the exit code is 0

Given an installed skill "my-skill" at the latest ref
When the user runs `skillprism update my-skill`
Then "my-skill is up to date" is printed to stdout
And no files are modified
And the exit code is 0

Given an installed skill "my-skill" with a newer ref available
When the user runs `skillprism update my-skill --diff`
Then a unified diff of the changes is printed to stdout
And no files are modified

Given 3 installed skills with updates available
When the user runs `skillprism update --force`
Then all 3 skills are updated to their latest refs
And all state records are updated

Given a skill "my-skill" whose source repo no longer exists
When the user runs `skillprism update my-skill`
Then a clear error is printed to stderr identifying the fetch failure
And the existing installed files are not modified
And the exit code is 1
```

---

#### DIST-I006 Integration Tests for Distribution Commands
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** DIST-I002, DIST-I003, DIST-I004, DIST-I005
- **Description:** Write integration tests in `tests/distribution.rs` covering the full `add` → `list` → `update` → `remove` lifecycle end-to-end via `assert_cmd`. Tests use a local fixture repo (created in a temp dir with both skillprism-format and plain-format skills) as the add source to avoid network dependency in CI — the `add` command accepts a local path as a v1 source form. Each test asserts on both filesystem state (files exist/don't exist at expected paths) and the state tracking layer (records present/absent with correct metadata).
- **Acceptance Criteria (Gherkin):**
```gherkin
Given a local fixture repo with one skillprism-format skill and one plain-format skill
When the integration test runs `skillprism add <fixture-path> --force`
Then both skills are installed to the expected harness paths
And the state tracking layer contains records for both skills

Given both skills are installed
When the integration test runs `skillprism list`
Then stdout contains both skill names with correct metadata

Given both skills are installed
When the integration test runs `skillprism remove --all --force --all-scopes`
Then both skills are removed from all harness paths
And the state tracking layer is empty
And `skillprism list` emits "No installed skills" to stderr (stdout stays empty)

Given a skillprism-format skill installed at version A
When the fixture is updated to version B and the test runs `skillprism update --force`
Then the installed files reflect version B
And the state record is updated to version B
```

---

#### DIST-I007 Docs and Website Updates
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I002, DIST-I003, DIST-I004, DIST-I005
- **Description:** Update the README, CHANGELOG, Hugo website docs, and CLI reference to cover the new distribution commands. Add a "Distribution" section to the website (install from remote sources, the add/list/remove/update workflow, auto-detection of skillprism vs plain format, per-harness rendering on install, the `git` runtime dependency for distribution commands). Update the CLI reference page with the new commands and flags. Update the homepage to position skillprism as a distribution CLI with per-harness templating, not just a build tool. Add a "skillprism vs Vercel skills CLI" comparison page that is honest about what each tool does, grounded in the spike's analysis (Vercel's `simple-git` + `gh` + SSH fallback; skillprism's direct `git` shell-out). Update AGENTS.md with any new devenv commands.

  **In addition:** update `Cargo.toml`'s `description` field (line 6) to reflect the v1.0.0 positioning. The current `"Build-time compiler for multi-harness agent skills"` is visibly wrong once Epic I ships — the package description is what `cargo search`, `cargo install --dry-run`, and clap's `#[command(about)]` (currently at `src/cli.rs:32`) all surface to the user. The new description MUST drop "build-time" and position skillprism as a distribution CLI with per-harness templating, in line with the README and Hugo homepage rewrites.

  **Completions test update:** the `completions_bash_includes_subcommands` test at `src/cli.rs:593-615` currently asserts on the exact strings `"build"`, `"validate"`, `"init"`, `"completions"`. The implementation PR MUST update this test (or add an equivalent) to assert on the new subcommands `add`, `list`, `remove`, `update` (and any aliases — e.g., `ls` for `list`, `rm` for `remove`). Without this, the first completions run after Epic I lands will fail CI on a stale assertion.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given the Hugo website at site/
When the docs are updated
Then a new docs/distribution.md page exists covering add/list/remove/update
And the CLI reference page includes the new commands with all flags
And the homepage positions skillprism as a distribution CLI with per-harness templating
And a comparison page honestly contrasts skillprism with Vercel's skills CLI (citing vercel-labs/skills as the upstream reference)

Given the README.md
When it is updated
Then the CLI reference includes add/list/remove/update commands
And a new section explains the distribution workflow with examples
And a "Prerequisites" note documents that `git` must be on PATH for `add` and `update`

Given the CHANGELOG.md
When it is updated
Then an "Unreleased" entry documents the new distribution commands
```

---

#### DIST-I008 Interactive `add` Prompts
- **Type:** Feature
- **Effort:** 2
- **Dependencies:** DIST-I002, DIST-I010
- **Description:** When `--harnesses` is not provided and no `skillprism.yaml` exists, prompt the user interactively to select which harnesses to install to. When `--target` is not provided, prompt the user to choose project or user scope. Show a summary before executing. Reuse `--force` to skip prompts. Use `dialoguer` crate for interactive multi-select and confirm prompts. No harnesses are pre-selected; detected agents are shown as a hint only. The user must explicitly choose which agents to install to.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given no --harnesses flag and no skillprism.yaml
When the user runs skillprism add owner/repo
Then the user is prompted to select harnesses interactively
And no harnesses are pre-selected
And detected agents are shown as a hint

Given no --target flag
When the user runs skillprism add owner/repo
Then the user is prompted to choose project or user scope

Given all selections made and user confirms
When the installation proceeds
Then the selected scope and harnesses are used
```

---

#### DIST-I009 npm Launcher
- **Type:** Feature
- **Effort:** 1
- **Dependencies:** Release CI (Epic H)
- **Description:** Create a thin npm package (`npm/` at repo root) whose `bin` entry is a small JS launcher script. On `npx skillprism` or `npm install -g skillprism`, the launcher detects the platform, downloads the correct pre-built binary from the latest GitHub Release, caches it, and execs it. The binary is never built from npm — the npm package is purely a download + exec gateway. Modeled after Biome's approach (`@biomejs/biome`).
- **Acceptance Criteria (Gherkin):**
```gherkin
Given the npm package is scaffolding exists
When node npm/bin/cli.mjs --help is run
Then it downloads the correct binary for the platform
And forwards --help to the binary
And prints the help output

Given the binary is cached
When node npm/bin/cli.mjs list is run a second time
Then it uses the cached binary
```

---

#### DIST-I010 Agent Auto-Detection
- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Implement a module that probes common agent installation paths (`~/.claude/`, `~/.config/opencode/`, `~/.codex/`, `~/.factory/`, `~/.pi/`) to detect which agents the user has installed. Detection is purely filesystem-based (no API calls). Used by DIST-I008 to display a "Detected agents" hint in the interactive `add` prompt; detected agents are **not** pre-selected.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given no skillprism.yaml and no --harnesses flag
When add probes for installed agents
Then it checks ~/.claude, ~/.config/opencode, ~/.codex, ~/.factory, ~/.pi

Given ~/.claude exists but ~/.opencode does not
When detection runs
Then only claude is returned as detected

Given the interactive add harness prompt is shown
When agents are detected
Then a hint lists the detected agents
And no harnesses are pre-selected
```

# Epic I — Distribution CLI

Acronym: **DIST** | Story Points: **35**

**Dependencies:** Epic H (RELS) — release artifacts must exist before the v1.0.0 tag is cut; PRD non-goal `plugin-marketplace.md` reopened by operator directive (see `.constitution/prd/out-of-scope/plugin-marketplace.md` for the `[REOPENED 2026-07-02]` annotation; full PRD revision is a downstream follow-up tracked in `prd/changelog.md`).

## Overview

Expand skillprism from a build-time compiler into a distribution CLI — a Vercel `skills` CLI competitor that adds per-harness templating and build-time validation to the same install/list/remove/update workflow.

**Operator directive:** skillprism's model replaces Vercel's single-generic-skill model with one where authors write skillprism MiniJinja skills and the CLI handles per-harness tailoring. The CLI consumes both skillprism-format sources (skill.yaml + SKILL.md → render per-harness) and plain SKILL.md sources (copy as-is), auto-detected per skill directory.

**Scope of this epic:**
- `add` — fetch from remote, auto-detect format, render/copy, write to target scope, record state
- `list` — show installed skills from state
- `remove` — delete installed skills + update state
- `update` — re-fetch latest, render/copy, update state

**Deferred to future epics:**
- `find` — requires a directory/registry backend that doesn't exist yet
- `use` — render to temp + launch agent (convenience command, adds process-launching complexity)
- Harness coverage expansion — keep 5 built-in harnesses; grow via community contributions (`init harness` + `harnesses/<name>.yaml`)

**Key decisions (from Tasks-stage interview):**
- Network layer: deferred to spike DIST-I001 (research Vercel's actual method from source)
- Installed-skill state: system-wide `~/.config/skillprism/` directory (not per-project)
- Install scope: reuse existing `--target project|user|dist` flag
- Source format auto-detection: per-skill directory at fetch time
- Epic structure: one epic, phased tickets, one PR

---

#### DIST-I001 Spike: Remote Fetch Methodology
- **Type:** Spike
- **Effort:** 3
- **Dependencies:** None
- **Description:** Research Vercel's `skills` CLI (https://github.com/vercel-labs/skills) source code to determine their actual fetch methodology — do they shell out to `git clone`, use a GitHub API client, download tarballs, or something else? How do they handle auth, caching, shallow clones, and partial fetches? Based on those learnings, recommend skillprism's fetch approach: git clone (shell out), native HTTP (ureq + rustls), or hybrid. The recommendation must account for ADR-004 (synchronous), ADR-003 (single crate), the "no runtime deps" constraint, and the need to support GitHub, GitLab, and any git URL. Write findings to `.constitution/spikes/SPK-DIST-I001.md`. No production code changes.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given the Vercel skills CLI source code at https://github.com/vercel-labs/skills
When the spike reads the fetch/install source code in src/
Then the spike report documents Vercel's actual fetch method (git clone, API, tarball, or other)
And the report documents how Vercel handles auth, caching, and shallow/partial fetches
And the report recommends a specific fetch approach for skillprism with justification
And the recommendation respects ADR-004 (synchronous) and the single-static-binary constraint
And the spike report is saved to .constitution/spikes/SPK-DIST-I001.md
```

---

#### DIST-I002 State Tracking Layer
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** DIST-I001
- **Description:** Implement a system-wide state tracking layer in `~/.config/skillprism/` that records installed skills. Each install record tracks: skill name, source URL, installed version (git SHA or tag), target scope (project|user), harnesses rendered to, install timestamp, and the source-format detection result (skillprism-format vs plain-format) so `update` can re-render consistently. The state file must be human-readable, atomically written (per ADR-005), and survive partial failures. The state file format is fixed to YAML (`installed.yaml`) — matches the rest of the project's `yaml_serde` choice and avoids a parallel JSON reader/writer. The state directory is created with mode `0o700` (per-user only) and is resolved via `XDG_CONFIG_HOME` with `~/.config/skillprism/` fallback. On macOS, `~/.config/` is a non-standard path (Apple's HIG is `~/Library/Application Support/skillprism/`); skillprism follows the XDG convention anyway because `XDG_CONFIG_HOME` is respected by enough cross-platform tooling to outweigh Apple-platform aesthetics, and a future ticket can layer in a macOS-specific path if it becomes a real complaint. A new `src/state/` module encapsulates read/write/query operations. This is the foundation for `list`, `remove`, and `update` — no CLI commands are wired yet. **Atomicity model:** every mutation of the state file (add, remove, update) MUST read the entire `installed.yaml`, compute the new in-memory state, and rewrite the entire file via a single temp-rename (the same ADR-005 pattern the build pipeline uses for output files). Per-record atomicity is not enough — partial-state writes would split the file across records. **Concurrency note:** ADR-005's temp-rename protects against partial writes within a single process but does **not** protect against two concurrent `skillprism add` invocations — both readers will compute new state, both will rename, and the second writer silently clobbers the first. v1 of the state layer does not support concurrent `add` calls in the same state directory; the state module MUST document this limitation and MUST recommend running the CLI in a single-writer fashion (CI jobs serialize, humans don't background two `add`s in parallel). A `flock` is intentionally out of scope for v1 — add it as a follow-up ticket if real-world usage demands it. The exact field set, ordering, and timestamp format (`installed.yaml` schema) is owned by DIST-I002 itself when it is implemented — the spike DIST-I001 does not gate this; DIST-I002 should pin the schema in its own implementation PR (with a `tests/fixtures/installed.yaml` and a schema-version field) so this ticket doesn't preempt the implementation design.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given a fresh environment with no ~/.config/skillprism/ directory
When the state layer is initialized
Then ~/.config/skillprism/ is created with mode 0o700 (per-user only)
And a state file (installed.yaml, fixed format) is created empty
And the state layer's public API documents the v1 single-writer limitation (no concurrent `add` calls in the same state directory)

Given an installed skill record with name "my-skill", source "owner/repo", version "abc123", scope "project", harnesses ["claude", "opencode"]
When the record is written to the state file
Then the state file is atomically written (temp file + rename per ADR-005)
And the state file contains the complete record with all fields
And the state file is human-readable

Given $XDG_CONFIG_HOME is set to /custom/config
When the state layer resolves the state directory
Then it uses /custom/config/skillprism/ instead of ~/.config/skillprism/

Given the state file contains 3 installed skill records
When the state layer queries for all installed skills
Then it returns all 3 records with their full metadata

Given the state file contains a record for "my-skill"
When the state layer removes the record for "my-skill"
Then the state file no longer contains "my-skill"
And the state file is atomically rewritten
```

---

#### DIST-I003 `add` Command — Fetch, Auto-Detect, Render, Write
- **Type:** Feature
- **Effort:** 8
- **Dependencies:** DIST-I001, DIST-I002
- **Description:** Implement the `skillprism add <source>` command. The source is a git URL, GitHub owner/repo shorthand, or local path. The command fetches the source (per the spike recommendation — DIST-I001), walks it for skill directories, and for each skill auto-detects the format. **Auto-detection (semantic contract only; the detection methodology is owned by this ticket's implementation PR):** the discriminator is *what the template file actually contains*, not which sibling files exist. For each skill directory, locate the template via the existing `find_template_path` from `src/loader/project.rs:121` (which returns one of `SKILL.md.j2`, `SKILL.md`, or `None` — the existing `(j2, bare, ambiguous)` matching is the canonical source-of-truth and MUST be reused, not re-implemented):

  1. **Template absent** → the directory is not a skill; the walk skips it.
  2. **Both `SKILL.md` and `SKILL.md.j2` exist** → the skill is **ambiguous**; `add` MUST surface `ProjectError::AmbiguousTemplate` verbatim, matching the `init` / `build` flow.
  3. **Exactly one template file exists** → the skill is `skillprism-format` if the template contains any MiniJinja variables (`{{` or `{%`), `plain-format` otherwise. The exact detection method (regex scan, parser-based, etc.) is implementation-owned and may evolve with the spike's findings — the planning doc only pins the *semantic* of "plain-format = a file with no template variables." The semantic matters because the existing loader (`src/loader/project.rs:117-120` and the test at line 405) treats a bare `SKILL.md` as a MiniJinja template, so a plain-format copy MUST only run on files that are actually plain (no markers) — copying a marker-bearing file verbatim would leave unrendered `{{ ... }}` in the installed file.
     - `skillprism-format` → render per configured harness via the existing Load → Resolve → Validate → Render pipeline. The skill is recorded in the state layer with `format: skillprism`.
     - `plain-format` → copy the template bytes verbatim to each harness's output path (renamed to `SKILL.md` if the source was `SKILL.md.j2`), plus copy every direct subdirectory of the skill's source directory via the existing `discover_asset_dirs` / `copy_assets` helpers (the new module MUST reuse them, not re-implement). The skill is recorded in the state layer with `format: plain`.

  `skill.yaml` is not a discriminator — it is consulted only when rendering a `skillprism-format` skill, to populate the variable context. The `--target` flag (default: project) controls where output is written (reuses `TargetScope`, but `add` only accepts `project` and `user` — see Gherkin below for the `dist` rejection). The `--skill` flag filters which skills to install from a multi-skill repo. The `--harnesses` flag (`-H`, comma-separated) filters which harnesses to render to (default: all in `skillprism.yaml` or all built-in if no project config) — reuses the flag name from the existing `init project` and `init skill` subcommands for internal consistency. After writing, the command records each installed skill in the state tracking layer. Overwrite confirmation applies per existing safety model unless `--force`.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given a skillprism-format skill repo with skill.yaml + SKILL.md
When the user runs `skillprism add owner/repo`
Then the repo is fetched to a temporary directory
And each skill directory is auto-detected as skillprism-format
And the skill is rendered once per configured harness
And the rendered SKILL.md files are written to each harness's project scope path
And each installed skill is recorded in the state tracking layer

Given a plain-format skill repo with only SKILL.md (no skill.yaml)
When the user runs `skillprism add owner/repo`
Then the repo is fetched to a temporary directory
And each skill directory is auto-detected as plain-format
And the SKILL.md is copied as-is to each harness's project scope path
And each installed skill is recorded in the state tracking layer

Given a multi-skill repo with skills "alpha" and "beta"
When the user runs `skillprism add owner/repo --skill alpha`
Then only the "alpha" skill is installed
And "beta" is not installed

Given a repo with skills targeting claude and opencode
When the user runs `skillprism add owner/repo --harnesses claude`
Then skills are only rendered/copied to .claude/skills/
And .opencode/skills/ is not written

Given a fetched repo with no skill directories anywhere (no SKILL.md* reachable)
When the user runs `skillprism add owner/repo`
Then the command exits with code 1
And a clear error is printed to stderr naming the source and explaining that no skillprism-format or plain-format skills were found
And no files are written
And the state tracking layer is not modified

Given the user invokes `skillprism add` with --target dist
When clap parses the arguments
Then the command exits with code 2 (clap parse error)
And the error message names `--target` and lists the valid values: project, user
And the state tracking layer is not modified

Given a skill directory containing both SKILL.md and SKILL.md.j2 (ambiguous template)
When the user runs `skillprism add owner/repo --skill that-skill`
Then the command fails with ProjectError::AmbiguousTemplate (re-uses the existing loader rule from src/loader/project.rs)
And no files are written
And the state tracking layer is not modified

Given a plain-format skill directory containing only SKILL.md (no SKILL.md.j2) where the file's text contains no `{{` or `{%` MiniJinja markers, and the directory has a `references/` subdirectory
When the user runs `skillprism add owner/repo --skill that-skill --harnesses claude`
Then that skill is auto-detected as plain-format (markers absent, not based on skill.yaml presence)
And the SKILL.md is copied as-is to .claude/skills/that-skill/SKILL.md
And the references/ directory tree is copied to .claude/skills/that-skill/references/ via the existing copy_assets helper
And the installed skill is recorded in the state tracking layer with format: plain and harnesses: [claude]

Given a bare SKILL.md that contains MiniJinja markers (e.g. `# {{ name }}`), with no skill.yaml
When the user runs `skillprism add owner/repo --skill that-skill`
Then that skill is auto-detected as skillprism-format (markers present, independent of skill.yaml)
And the render fails the same way `build` would fail on the same input (variable context is missing)
And no files are written
And the state tracking layer is not modified

Given an already-installed skill "my-skill" at the target scope
When the user runs `skillprism add owner/repo --skill my-skill` without --force
Then the user is prompted to overwrite (y/n/s/a)
And if the user declines, no files are written

Given a skillprism-format skill with undefined template variables
When the user runs `skillprism add owner/repo`
Then the build fails with a validation error identifying the undefined variable
And no output files are written

Given a fetch failure (network error, invalid URL, repo not found)
When the user runs `skillprism add invalid-source`
Then a clear error message is printed to stderr identifying the failure
And the exit code is 1
```

---

#### DIST-I004 `list` Command
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I002
- **Description:** Implement the `skillprism list` command (alias: `ls`). Reads the state tracking layer and displays installed skills in a table: name, source, version (short SHA), scope, harnesses. The `--target` flag filters by scope (project|user). The `--harnesses` flag (`-H`, comma-separated) filters by harness — same flag name as the existing `init` and `add` commands. Output goes to stdout (machine-parseable table); diagnostics to stderr per the stdout/stderr discipline. If no skills are installed, prints "No skills installed" to stdout.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given 3 skills installed across project and user scopes
When the user runs `skillprism list`
Then a table is printed to stdout showing all 3 skills with name, source, version, scope, and harnesses

Given skills installed in both project and user scopes
When the user runs `skillprism list --target user`
Then only user-scoped skills are listed

Given skills installed for claude and opencode
When the user runs `skillprism list --harnesses claude`
Then only skills installed for the claude harness are listed

Given no skills installed
When the user runs `skillprism list`
Then "No skills installed" is printed to stdout
And the exit code is 0
```

---

#### DIST-I005 `remove` Command
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I002
- **Description:** Implement the `skillprism remove [skills...]` command (alias: `rm`). Removes installed skills from the filesystem and the state tracking layer. The `--target` flag filters by scope. The `--harnesses` flag (`-H`, comma-separated) removes only from a specific harness's directory. The `--all` flag removes all installed skills. Interactive confirmation is shown unless `--force` is passed (the same flag name as the existing `build` command's skip-confirmation behavior; using `--force` for consistency, not `--yes`). Removal respects scope confinement (never deletes outside the determined scope path). After removing files, the state record is updated atomically. **Note on `--target dist`:** the `TargetScope::Dist` enum variant exists for `build --target dist` (writes to `./dist/` for inspection) but is not an install target — `add` and `remove` only operate on `project` and `user` scopes. The `--all` flag MUST therefore iterate `project` + `user` only, never `dist`; iterating `dist` would risk deleting files that were never installed and could nuke unrelated build output.
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
When the user runs `skillprism remove --all --force`
Then all 5 installed skills are removed from both project and user scopes
And the state file is empty
And the `dist/` directory is NOT touched (dist is a build inspection target, not an install scope)

Given the user invokes `skillprism remove` with --target dist
When clap parses the arguments
Then the command exits with code 2 (clap parse error)
And the error message names `--target` and lists the valid values: project, user

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

#### DIST-I006 `update` Command
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** DIST-I002, DIST-I003
- **Description:** Implement the `skillprism update [skills...]` command. For each named skill (or all if none named), re-fetches the source at the latest version, computes the new render output (or copy output for plain-format skills) for the configured harnesses, and decides whether each file needs writing via a content-equality check against the existing file at the target path. Only files whose content actually changed are written. The change-detection method is owned by DIST-I006 when implemented (the spike does not gate it; the existing `Router::diff` is the display-only diff renderer used solely for the `--diff` flag's user-facing output, not as the change test). If a skill is already at the latest version (same SHA), no action is taken and an "up to date" message is printed. The `--harnesses` flag (`-H`, comma-separated) restricts the update to a specific harness subset — same flag name as `init`, `add`, and `list`. The `--diff` flag shows what would change without writing. The `--force` flag skips confirmation (same flag name as `build`, `add`, and `remove`; using `--force` for consistency, not `--yes`). Update respects all safety models (atomic writes, scope confinement, overwrite confirmation) and writes only to `project` and `user` scopes (same `--target dist` rejection as `add` and `remove`).
- **Acceptance Criteria (Gherkin):**
```gherkin
Given an installed skill "my-skill" at version "abc123"
When a newer version "def456" is available and the user runs `skillprism update my-skill --force`
Then the latest source is fetched
And the new rendered output is compared against the installed files
And the updated files are written atomically
And the state record is updated to version "def456"
And the exit code is 0

Given an installed skill "my-skill" at the latest version
When the user runs `skillprism update my-skill`
Then "my-skill is up to date" is printed to stdout
And no files are modified
And the exit code is 0

Given an installed skill "my-skill" with a newer version available
When the user runs `skillprism update my-skill --diff`
Then a unified diff of the changes is printed to stdout
And no files are modified

Given 3 installed skills with updates available
When the user runs `skillprism update --force`
Then all 3 skills are updated to their latest versions
And all state records are updated

Given a skill "my-skill" whose source repo no longer exists
When the user runs `skillprism update my-skill`
Then a clear error is printed to stderr identifying the fetch failure
And the existing installed files are not modified
And the exit code is 1
```

---

#### DIST-I007 Integration Tests for Distribution Commands
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** DIST-I003, DIST-I004, DIST-I005, DIST-I006
- **Description:** Write integration tests in `tests/distribution.rs` covering the full `add` → `list` → `update` → `remove` lifecycle end-to-end via `assert_cmd`. Tests use a local fixture repo (created in a temp dir with both skillprism-format and plain-format skills) as the add source to avoid network dependency in CI. Each test asserts on both filesystem state (files exist/don't exist at expected paths) and the state tracking layer (records present/absent with correct metadata).
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
When the integration test runs `skillprism remove --all --force`
Then both skills are removed from all harness paths
And the state tracking layer is empty
And `skillprism list` outputs "No skills installed"

Given a skillprism-format skill installed at version A
When the fixture is updated to version B and the test runs `skillprism update --force`
Then the installed files reflect version B
And the state record is updated to version B
```

---

#### DIST-I008 Docs and Website Updates
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DIST-I003, DIST-I004, DIST-I005, DIST-I006
- **Description:** Update the README, CHANGELOG, Hugo website docs, and CLI reference to cover the new distribution commands. Add a "Distribution" section to the website (install from remote sources, the add/list/remove/update workflow, auto-detection of skillprism vs plain format, per-harness rendering on install). Update the CLI reference page with the new commands and flags. Update the homepage to position skillprism as a distribution CLI with per-harness templating, not just a build tool. Add a "skillprism vs Vercel skills CLI" comparison page that is honest about what each tool does. Update AGENTS.md with any new devenv commands.
- **Acceptance Criteria (Gherkin):**
```gherkin
Given the Hugo website at site/
When the docs are updated
Then a new docs/distribution.md page exists covering add/list/remove/update
And the CLI reference page includes the new commands with all flags
And the homepage positions skillprism as a distribution CLI with per-harness templating
And a comparison page honestly contrasts skillprism with Vercel's skills CLI

Given the README.md
When it is updated
Then the CLI reference includes add/list/remove/update commands
And a new section explains the distribution workflow with examples

Given the CHANGELOG.md
When it is updated
Then an "Unreleased" entry documents the new distribution commands and the spike outcome
```

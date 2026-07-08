---
title: "Distribution"
description: "Installing, listing, updating, and removing skills"
---

# Distribution

skillprism's distribution commands let you install skills from remote Git repositories or local paths, manage their lifecycle, and keep them up to date — all with per-harness rendering built in.

## Prerequisites

The `add` and `update` commands require **`git`** to be on `PATH`. They use `git clone --depth 1` for initial fetches and `git ls-remote` for lightweight up-to-date checks.

## Install (`add`)

```bash
skillprism add owner/repo
skillprism add https://github.com/owner/repo.git
skillprism add ./local/path
skillprism add owner/repo#v1.0.0
skillprism add owner/repo --skill my-skill
skillprism add owner/repo --target user
skillprism add owner/repo -H claude,opencode
```

### Source formats

`add` accepts these source forms:

| Form | Example | Description |
|------|---------|-------------|
| GitHub shorthand | `owner/repo` | Expands to `https://github.com/owner/repo.git` |
| GitHub prefix | `github:owner/repo` | Explicit `github:` prefix form |
| GitLab shorthand | `gitlab:owner/repo` | GitLab via `gitlab:` prefix |
| Full HTTPS URL | `https://github.com/owner/repo.git` | Any valid Git remote HTTPS URL |
| SSH URL | `git@github.com:owner/repo.git` | SSH Git URL |
| Local path | `./path/to/skills` | A directory with `skillprism.yaml` and `skills/` |

Append `#<ref>` to any remote source to pin to a specific branch, tag, or commit (e.g., `owner/repo#v2.0.0`). Append `/<subpath>` to install from a subdirectory within a repo (e.g., `owner/repo/skills/my-skill`). Both can be combined: `owner/repo/skills/my-skill#v1.0.0`.

### Format auto-detection

Each skill directory is inspected:

- **skillprism-format**: Has `skill.yaml` with `skillprism: '1'`. The `SKILL.md` is rendered through MiniJinja once per harness with variables from `skill.yaml`.
- **plain-format**: Has only `SKILL.md` (no `skill.yaml`, or `skill.yaml` without `skillprism: '1'`). The file is copied as-is to each harness path.

### Target scopes

- **`project`** (default): Writes to `.claude/skills/`, `.opencode/skills/`, etc. in the current project.
- **`user`**: Writes to `~/.claude/skills/`, `~/.config/opencode/skills/`, etc. — available globally.

The `--target dist` scope is not supported for `add` (it is rejected at parse time).

## List (`list` / `ls`)

```bash
skillprism list
skillprism list --target user
skillprism list -H claude
```

Lists every installed skill with its metadata: name, source URL, ref, format, scope, and the harnesses it was installed to. Filter by scope or harness with `--target` and `-H`.

## Remove (`remove` / `rm`)

```bash
skillprism remove my-skill
skillprism remove my-skill another-skill
skillprism remove --all
skillprism remove --all --all-scopes --force
skillprism remove --target project -H claude
```

Removes skill files from the filesystem and their records from the state tracking layer. By default, only removes from the current project scope and requires confirmation unless `--force` is passed.

## Update (`update` / `up`)

```bash
skillprism update
skillprism update my-skill
skillprism update --diff
skillprism update -H claude
skillprism update --target user
```

Updates check each installed skill for a newer version:

1. **Lightweight check**: `git ls-remote` compares the resolved ref SHA against the stored `resolved_ref`. If they match, the skill is up to date — no clone needed.
2. **Fetch**: If the ref changed, the repo is fetched with `git clone --depth 1` to a temporary directory.
3. **Re-render**: Each skill is rendered again (or the plain file is read).
4. **Per-file comparison**: Each output file's SHA-256 hash is compared against the stored state. Only files whose content actually changed are written.
5. **State update**: The installed record is updated with the new `resolved_ref` and file hashes.

The `--diff` flag shows a unified diff of what would change without writing any files.

Local-path sources are skipped during update (they have no git ref to compare).

## State tracking

All installed skills are recorded in `~/.config/skillprism/installed.yaml`. The state file tracks:

- Skill name and format (skillprism or plain)
- Source URL, ref, and resolved SHA
- Install scope (project or user)
- Per-harness output paths with file hashes (SHA-256)
- Timestamps

The state file is created with `600` permissions in a `700` directory. It is read and written atomically.

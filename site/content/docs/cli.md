---
title: "CLI reference"
description: "Complete command and flag reference"
---

# CLI reference

## Commands

```
skillprism build [--target project|user|dist] [--diff|--dry-run] [--force] [-v]
skillprism validate [path] [-v]
skillprism init project <name> [--out <dir>] [-H <harnesses>]
skillprism init skill <name>
skillprism init harness <name>
skillprism completions <bash|fish|zsh>
skillprism add <source> [--target project|user] [--skill <name>] [-H <harnesses>] [--force]
skillprism list [--target project|user] [-H <harnesses>]          (alias: ls)
skillprism remove [<skills>...] [--target project|user] [-H <harnesses>] [--all] [--all-scopes] [--force]  (alias: rm)
skillprism update [<skills>...] [--target project|user] [-H <harnesses>] [--diff|--dry-run] [--force]  (alias: up)
```

## Global flags

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Enable verbose progress output with per-phase timing and resolved variable listing |

## build

Compiles all skills to all configured harnesses and writes the output files.

```bash
skillprism build
skillprism build --diff
skillprism build --target user --force
skillprism build --target dist -v
```

| Flag | Description |
|------|-------------|
| `--target <scope>` | Output scope: `project` (default), `user` (global `~/.<harness>/skills/`), or `dist` (inspectable `dist/<harness>/` directory) |
| `--diff` / `--dry-run` | Show a colored unified diff of what would be written, without modifying any files |
| `--force` | Overwrite existing files without confirmation (skip the y/n/s/a prompt) |

### Target scopes

- **`project`** (default): Writes to project-local directories (`.claude/skills/`, `.opencode/skills/`, etc.) — the paths agents discover skills in when working in your repo.
- **`user`**: Writes to the user's home directory (`~/.claude/skills/`, `~/.config/opencode/skills/`, etc.) — makes skills available globally, not just in one project.
- **`dist`**: Writes to a `dist/<harness-id>/` directory inside the project — useful for inspecting output, building a distributable archive, or CI artifacts. Doesn't touch live agent directories.

## validate

Checks all skills for syntax errors, undefined variables, missing macros, and spec compliance — without writing any files.

```bash
skillprism validate
skillprism validate /path/to/project
```

Lists each skill×harness pair as `ok` and any portability warnings. Errors fail with a non-zero exit code.

| Argument | Description |
|----------|-------------|
| `path` | Path to the project root (default: `.`) |

## init

Scaffolds new projects, skills, or harness definitions.

### init project

```bash
skillprism init project my-skills
skillprism init project my-skills --out ./projects/my-skills
skillprism init project my-skills -H claude,codex,opencode
```

Creates a new project directory with `skillprism.yaml`, a sample skill, `.gitignore`, and `README.md`.

| Flag | Description |
|------|-------------|
| `--out <dir>` | Output directory (default: `./<name>`) |
| `-H, --harnesses <list>` | Comma-separated harness IDs (default: `claude,opencode`) |

### init skill

```bash
skillprism init skill my-agent
```

Scaffolds a new skill into an existing project's `skills/` directory. Creates `skill.yaml` (with spec-compliant metadata), `SKILL.md` (with frontmatter template), and `references/` + `scripts/` asset directories.

### init harness

```bash
skillprism init harness my-custom-agent
```

Scaffolds a custom harness definition in `harnesses/<name>.yaml`.

## completions

Generates shell completion scripts to stdout.

```bash
skillprism completions bash
skillprism completions fish
skillprism completions zsh
```

## add

Installs skills from a remote Git repository or local path. Each skill is auto-detected as **skillprism-format** (has `skill.yaml` with `skillprism: '1'` — rendered through MiniJinja per harness) or **plain-format** (bare `SKILL.md` — copied as-is). Installed skills are recorded in the state tracking layer at `~/.config/skillprism/installed.yaml`.

```bash
skillprism add owner/repo
skillprism add https://github.com/owner/repo.git
skillprism add ./local/path
skillprism add owner/repo --skill my-skill
skillprism add owner/repo#v1.0.0
skillprism add owner/repo --target user -H claude,opencode
```

| Argument | Description |
|----------|-------------|
| `source` | Source to install from — GitHub shorthand (`owner/repo`), full Git URL, or local path |
| `--target <scope>` | Install scope: `project` (default) or `user` |
| `--skill <name>` | Install only the named skill from a multi-skill source |
| `-H, --harnesses <list>` | Comma-separated harness IDs to install to (default: all configured) |
| `--force` | Overwrite existing files without confirmation |

> **Note:** `add` rejects `--target dist` at parse time — distribution sources can only install to `project` or `user` scopes.

## list

Lists installed skills with their metadata. Each entry shows the skill name, source, format, installed ref, and which harnesses it was installed to.

```bash
skillprism list
skillprism list --target user
skillprism list -H claude
```

| Flag | Description |
|------|-------------|
| `--target <scope>` | Filter by install scope: `project` or `user` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to filter by |

## remove

Removes installed skills from the filesystem and state tracking layer.

```bash
skillprism remove my-skill
skillprism remove my-skill another-skill
skillprism remove --all
skillprism remove --all --all-scopes --force
skillprism remove --target project -H claude
```

| Argument | Description |
|----------|-------------|
| `skills...` | One or more skill names to remove |
| `--target <scope>` | Filter by install scope: `project` or `user` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to remove from |
| `--all` | Remove all installed skills |
| `--all-scopes` | Allow removing across both project and user scopes |
| `--force` | Skip confirmation prompts |

## update

Updates installed skills to their latest source versions. Performs a lightweight up-to-date check via `git ls-remote` (checks the resolved ref SHA without a full clone). Re-renders only files whose content changed (SHA-256 per-file comparison). Output paths without changes are not touched.

```bash
skillprism update
skillprism update my-skill
skillprism update --diff
skillprism update -H claude
skillprism update --target user
```

| Argument | Description |
|----------|-------------|
| `skills...` | One or more skill names to update (default: all installed) |
| `--target <scope>` | Filter by install scope: `project` or `user` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to update |
| `--diff` / `--dry-run` | Show a diff of what would change without writing files |
| `--force` | Skip confirmation prompts |

## Finding the project root

`build` and `init skill` search upward from the current directory for `skillprism.yaml`. You can run them from anywhere inside a skillprism project — they'll find the root automatically.

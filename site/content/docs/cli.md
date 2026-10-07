---
title: "CLI reference"
description: "Complete command and flag reference"
group: "Reference"
weight: 70
---
## Commands

```
skillprism build [-H <harnesses>] [--diff|--dry-run] [--force] [-v]
skillprism validate [path] [-v]
skillprism init project <name> [--out <dir>] [-H <harnesses>]
skillprism init skill <name>
skillprism init harness <name>
skillprism completions <bash|fish|zsh>
skillprism add <source> [--target project|user] [-g|--global] [-a|--agent <id>] [-s|--skill <name>] [-H <harnesses>] [-y|--yes] [--list] [--all] [--force]
skillprism list [--target project|user] [-g|--global] [-a|--agent <id>] [-H <harnesses>]          (alias: ls)
skillprism remove [<skills>...] [-s|--skill <name>] [--target project|user] [-g|--global] [-a|--agent <id>] [-H <harnesses>] [--all] [--all-scopes] [-y|--yes] [--force]  (alias: rm)
skillprism update [<skills>...] [--target project|user] [-g|--global] [-p|--project] [-H <harnesses>] [--diff|--dry-run] [-y|--yes] [--force]  (alias: up)
```

## Global flags

| Flag | Description |
|------|-------------|
| `-v`, `--verbose` | Enable verbose progress output with per-phase timing and resolved variable listing |

## build

Compiles source skills into `dist/<harness>/` for all configured harnesses without modifying live agent directories.

```bash
skillprism build
skillprism build -H claude,opencode
skillprism build --diff
skillprism build --force -v
```

| Flag | Description |
|------|-------------|
| `-H, --harness <list>` | Target harness ID(s) to render for (comma-separated or repeated). Alias: `--harnesses` |
| `--diff` / `--dry-run` | Show a colored unified diff of what would be written, without modifying any files |
| `--force` | Overwrite existing files without confirmation (skip the y/n/s/a prompt) |

> **Note:** `build` is compile-only and always outputs into `dist/`. To install skills directly into live agent directories (e.g. `.claude/skills/`), use `skillprism add`.

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

The generated YAML starts with a `yaml-language-server` schema modeline for editor completion and validation.

| Flag | Description |
|------|-------------|
| `--out <dir>` | Output directory (default: `./<name>`) |
| `-H, --harnesses <list>` | Comma-separated harness IDs (default when no prompt is shown: `claude`, `opencode`). Without this flag, the prompt appears only when stdin and stdout are both a terminal. |

If the target directory already contains `skillprism.yaml`, the command fails with a usage error (exit code 2).

### init skill

```bash
skillprism init skill my-agent
```

Scaffolds a new skill into an existing project's `skills/` directory. Creates `skill.yaml` (with spec-compliant metadata and `skillprism: '1'`), `SKILL.md` (with frontmatter template), and `references/` + `scripts/` asset directories.

The generated YAML starts with a `yaml-language-server` schema modeline for editor completion and validation.

### init harness

```bash
skillprism init harness my-custom-agent
```

Scaffolds a custom harness definition in `harnesses/<name>.yaml`.

The generated YAML starts with a `yaml-language-server` schema modeline for editor completion and validation.

## completions

Generates shell completion scripts to stdout.

```bash
skillprism completions bash
skillprism completions fish
skillprism completions zsh
```

## add

Installs skills from a remote Git repository or local path into live agent directories. Auto-detects **skillprism-format** (has `skill.yaml` with `skillprism: '1'` — rendered per harness) or **plain-format** (bare `SKILL.md` — installed directly). Tracked in `.skillprism/state.json` (project scope) or `~/.config/skillprism/state.json` (user scope).

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
| `--target <scope>` | Install scope: `project` or `user` (prompts interactively if omitted) |
| `-g, --global` | Install into the user scope |
| `-s, --skill <name>` | Install only the named skill from a multi-skill source |
| `-a, --agent <id>` | Agent harness to install to (repeatable). Accepts skillprism ids plus `claude-code` and `droid` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to install to (default: all configured) |
| `-y, --yes` | Skip interactive prompts |
| `--list` | Print discovered skill names and write nothing |
| `--all` | Install every discovered skill |
| `--force` | Overwrite existing files and skip interactive prompts |

## list (alias: ls)

Lists installed skills with their metadata. Each entry shows the skill name, source, format, installed ref, target scope, and which harnesses it was installed to.

```bash
skillprism list
skillprism list --target user
skillprism list -H claude
```

| Flag | Description |
|------|-------------|
| `--target <scope>` | Filter by install scope: `project` or `user` |
| `-g, --global` | List only the user scope |
| `-a, --agent <id>` | Agent harness to filter by (repeatable). Accepts skillprism ids plus `claude-code` and `droid` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to filter by |

## remove (alias: rm)

Removes installed skills from live harness directories and state tracking.

```bash
skillprism remove my-skill
skillprism remove my-skill another-skill
skillprism remove --all
skillprism remove --all --all-scopes --force
skillprism remove --all --target project -H claude
```

| Argument | Description |
|----------|-------------|
| `skills...` | One or more skill names to remove |
| `-s, --skill <name>` | Skill name to remove (repeatable) |
| `--target <scope>` | Filter by install scope: `project` or `user` |
| `-g, --global` | Remove from the user scope |
| `-a, --agent <id>` | Agent harness to remove from (repeatable). Accepts skillprism ids plus `claude-code` and `droid` |
| `-H, --harnesses <list>` | Comma-separated harness IDs to remove from |
| `--all` | Remove all installed skills |
| `--all-scopes` | Allow removing across both project and user scopes |
| `-y, --yes` | Skip confirmation prompts |
| `--force` | Skip confirmation prompts |

## update (alias: up)

Updates installed skills to their latest source versions. Re-renders only files whose content changed.

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
| `-g, --global` | Update the user scope |
| `-p, --project` | Update the project scope |
| `-H, --harnesses <list>` | Comma-separated harness IDs to update |
| `--diff` / `--dry-run` | Show a diff of what would change without writing files |
| `-y, --yes` | Skip confirmation prompts |
| `--force` | Skip confirmation prompts |

## Finding the project root

Compiler commands (`build` and `init skill`) search upward from the current directory for `skillprism.yaml`. You can run them from anywhere inside a skillprism project — they'll find the root automatically.

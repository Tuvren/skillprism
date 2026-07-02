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

## Finding the project root

`build` and `init skill` search upward from the current directory for `skillprism.yaml`. You can run them from anywhere inside a skillprism project — they'll find the root automatically.

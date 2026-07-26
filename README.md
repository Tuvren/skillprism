# skillprism

[![CI](https://github.com/tuvren/skillprism/actions/workflows/ci.yml/badge.svg)](https://github.com/tuvren/skillprism/actions/workflows/ci.yml)

Distribution CLI and per-harness compiler for agent skills. Author skills once, compile for multiple harnesses (`dist/`), and manage live skill installations across agent environments — always [Agent Skills spec](https://agentskills.io/specification)-compliant.

**Documentation: [tuvren.github.io/skillprism](https://tuvren.github.io/skillprism/)**

## Installation

### Via npm / bun (Recommended)

```bash
# Using bun
bun add -g skillprism

# Using npm
npm install -g skillprism
```

### Prerequisites

- **Rust 1.85+** (edition 2024) if building from source
- **`git`** must be on `PATH` for the `add` and `update` distribution commands (they clone remote sources via `git clone --depth 1` and check for updates via `git ls-remote`)

### From source

```bash
cargo install --path .
```

### With devenv

```bash
devenv shell
```

## Dual Product Framing: Compiler + Package Manager

skillprism provides two distinct capabilities:

1. **Compiler / Authoring (`init`, `validate`, `build`)**: Scaffolds projects and compiles source skills into target harness outputs under `dist/<harness>/`.
2. **Package Manager / Lifecycle (`add`, `list`, `remove`, `update`)**: Installs skills from remote repositories or local paths into live agent directories (`.claude/skills/`, `~/.claude/skills/`, etc.) with atomic tracking.

| Role | Commands | Primary Output | npm Analogy |
|---|---|---|---|
| **Compiler** | `init`, `validate`, `build` | Project source + `dist/` artifacts | `npm init` / `tsc` |
| **Package Manager** | `add`, `list`, `remove`, `update` | Live harness paths (`.claude/skills/`, `~/.claude/skills/`) | `npm install` / `npm list` |

## Quickstart

### Initialize a project

```bash
skillprism init project my-skills
cd my-skills
```

### Add a skill

```bash
skillprism init skill my-agent
```

This generates `skills/my-agent/skill.yaml` and `skills/my-agent/SKILL.md` together — edit both; they're two halves of one skill.

### How `skill.yaml` and `SKILL.md` work together

`skill.yaml` holds metadata; `SKILL.md` is a [MiniJinja](https://docs.rs/minijinja) template, rendered once per harness configured in `skillprism.yaml`. It can also be named `SKILL.md.j2` if you'd rather the extension say "this is a template" explicitly — both are accepted, but not both at once in the same skill directory (skillprism errors rather than guessing which one you meant). Three kinds of values are available inside a template's `{{ }}`:

- **Built-ins**, always present: `skill_name`, `skill_description`, and `harness` (`harness.id`, `harness.name`, plus harness-specific macros — see `src/builtin_harnesses/*.yaml` for what each harness defines, e.g. `harness.subagent_guide`).
- **`skill.yaml` fields**, available under their own name — `license`, `version`, `compatibility`, `when_to_use`, `metadata.*`, `allowed_tools`, and more (full list: [skill-yaml reference](https://tuvren.github.io/skillprism/docs/skill-yaml/)). Both direct names (`model`, `paths`, `context`) and alias names (`model_override`, `activation_paths`, `context_fork`) are supported in template context.
- **`variables:`** — your own custom data. Anything under `variables:` in `skill.yaml` is available by name. Use this when a value is constant across every harness; if a variable or macro genuinely needs a *different* value depending on which harness is being built, set a top-level default and override it per harness:
  ```yaml
  variables:
    port: 5173
  overrides:
    claude:
      variables:
        port: 4173      # only claude's render sees this value
      macros:
        extra_note: "Claude-specific note, exposed as {{ harness.extra_note }}"
  ```

Minimal example:

```yaml
# skill.yaml
skillprism: '1'
name: my-agent
description: Helps with X
variables:
  greeting: Hello from my-agent
```

```jinja
# SKILL.md
# {{ skill_name }}

{{ skill_description }}

{{ greeting }}
```

### Build

```bash
skillprism build
```

`skillprism build` renders skills once per harness and writes compiled artifacts to `dist/<harness>/` (e.g. `dist/claude/skills/my-agent/SKILL.md`).

### Preview changes

```bash
skillprism build --diff
```

Shows a unified diff of what would be built without modifying any files.

### Validate

```bash
skillprism validate
```

Checks templates for syntax errors, undefined variables, and missing macros without writing output. Also enforces [Agent Skills spec](https://agentskills.io/specification) constraints.

## Distribution workflow

Install skills into live agent directories, manage their lifecycle, and keep them up to date.

### Install skills (`add`)

```bash
# Install from a GitHub shorthand
skillprism add owner/repo

# Install from a full Git URL (GitHub, GitLab, etc.)
skillprism add https://github.com/owner/repo.git

# Pin to a specific ref or filter to one skill
skillprism add owner/repo#v1.0.0
skillprism add owner/repo --skill my-skill

# Install from a local path into project scope
skillprism add ./path/to/skills --target project

# Install to specific harnesses
skillprism add owner/repo --target user -H claude,opencode
```

Each skill is either **skillprism-format** (has `skill.yaml` with `skillprism: '1'` — rendered through MiniJinja per harness) or **plain-format** (bare `SKILL.md` — copied as-is).

### List installed skills (`list` / `ls`)

```bash
skillprism list              # all installed
skillprism list --target user  # only user-scoped
skillprism list -H claude      # only claude harness
```

### Remove skills (`remove` / `rm`)

```bash
skillprism remove my-skill        # remove one skill
skillprism remove --all           # remove everything
skillprism remove --all --force   # skip confirmation
```

### Update skills (`update` / `up`)

```bash
skillprism update                 # check all installed skills for updates
skillprism update my-skill        # update a specific skill
skillprism update --diff          # preview changes without writing
skillprism update -H claude       # update only claude harness files
```

## Spec compliance

skillprism renders `SKILL.md` files with the YAML frontmatter the [Agent Skills specification](https://agentskills.io/specification) requires (`name` + `description` at minimum).

## Supported Harnesses

| Harness | Description | Project Path | User Path |
|---------|-------------|--------------|-----------|
| `claude` | Claude Code | `.claude/skills/` | `~/.claude/skills/` |
| `codex` | OpenAI Codex | `.agents/skills/` | `~/.codex/skills/` |
| `opencode` | OpenCode | `.opencode/skills/` | `~/.config/opencode/skills/` |
| `factory` | Factory | `.factory/skills/` | `~/.factory/skills/` |
| `pi` | Pi | `.pi/skills/` | `~/.pi/agent/skills/` |

## CLI Reference

```
skillprism build [-H <harnesses>] [--diff|--dry-run] [--force] [-v]
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

### Build flags

- `-H`, `--harness`: Target harness ID(s) (comma-separated or repeated). Alias: `--harnesses`
- `--diff` / `--dry-run`: Show a colored diff preview without writing files
- `--force`: Overwrite existing files without confirmation

### Init flags

- `init project <name>`: Scaffold a new project (`--out` for output dir, `-H`/`--harnesses` for comma-separated harness list; default: `claude,opencode`)
- `init skill <name>`: Scaffold a new skill into an existing project
- `init harness <name>`: Scaffold a custom harness definition in `harnesses/`

## Project Structure

```
my-skills/
├── skillprism.yaml          # Project config
├── skills/                  # Skill source directories
│   └── my-agent/
│       ├── skill.yaml       # Skill metadata (skillprism: '1')
│       ├── SKILL.md         # Template (MiniJinja; SKILL.md.j2 also accepted)
│       ├── references/      # Shared assets
│       └── scripts/
└── dist/                    # Compiled harness outputs (gitignored)
```

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Reproduction Script

```bash
./scripts/ci-check.sh   # build + test + clippy + fmt on pinned toolchain
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

# skillprism

[![CI](https://github.com/tuvren/skillprism/actions/workflows/ci.yml/badge.svg)](https://github.com/tuvren/skillprism/actions/workflows/ci.yml)

Distribution CLI with per-harness templating for agent skills. Install skills from remote sources, compile from local sources — all rendered once per harness, always [Agent Skills spec](https://agentskills.io/specification)-compliant.

**Documentation: [tuvren.github.io/skillprism](https://tuvren.github.io/skillprism/)**

## Installation

### Prerequisites

- **Rust 1.85+** (edition 2024)
- **`git`** must be on `PATH` for the `add` and `update` distribution commands (they clone remote sources via `git clone --depth 1` and check for updates via `git ls-remote`)

### From source

```bash
cargo install --path .
```

### With devenv

```bash
devenv shell
```

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

This generates `skills/my-agent/skill.yaml` and `skills/my-agent/SKILL.md` together
— edit both; they're two halves of one skill.

### How `skill.yaml` and `SKILL.md` work together

`skill.yaml` holds metadata; `SKILL.md` is a [MiniJinja](https://docs.rs/minijinja)
template, rendered once per harness configured in `skillprism.yaml`. It can also be
named `SKILL.md.j2` if you'd rather the extension say "this is a template" explicitly
— both are accepted, but not both at once in the same skill directory (skillprism
errors rather than guessing which one you meant). Three kinds of
values are available inside a template's `{{ }}`:

- **Built-ins**, always present: `skill_name`, `skill_description`, and `harness`
  (`harness.id`, `harness.name`, plus harness-specific macros — see
  `src/builtin_harnesses/*.yaml` for what each harness defines, e.g.
  `harness.subagent_guide`).
- **`skill.yaml` fields**, available under their own name — `license`, `version`, `compatibility`, `when_to_use`, `metadata.*`, `allowed_tools`, and more (full list: [skill-yaml reference](https://tuvren.github.io/skillprism/docs/skill-yaml/)). Three are exceptions and render under a different name than their `skill.yaml` key: `model:` is `{{ model_override }}`, `paths:` is `{{ activation_paths }}`, and `context:` is `{{ context_fork }}` (a derived boolean — `true` when `context: fork`, `false` otherwise — not the raw string). You don't have to reference every field in your template — some, like `compatibility` or `metadata.source`, are just packaging/attribution metadata that tooling can read without it appearing in the rendered body.
- **`variables:`** — your own custom data. Anything under `variables:` in `skill.yaml`
  is available by name. Use this when a value is constant across every harness; if a
  variable or macro genuinely needs a *different* value depending on which harness is
  being built, set a top-level default and override it per harness:
  ```yaml
  variables:
    port: 5173
  harnesses:
    claude:
      variables:
        port: 4173      # only claude's render sees this value
      macros:
        extra_note: "Claude-specific note, exposed as {{ harness.extra_note }}"
  ```
  Don't reach for `harnesses:` just because it exists — most skills render identically
  everywhere and don't need it at all. One caveat: validation checks every
  `{{ harness.* }}` reference statically, including both branches of `{% if %}` — so a
  macro override referenced anywhere in the template must be defined for *every*
  harness in `skillprism.yaml`'s `harnesses:` list, not just the one it's meaningfully
  different for, even if you only ever read it behind a guard.



Minimal example:

```yaml
# skill.yaml
name: my-agent
description: Helps with X
variables:
  greeting: Hello from my-agent
```

```
# SKILL.md
# {{ skill_name }}

{{ skill_description }}

{{ greeting }}
```

`skillprism build` renders this once per harness, writing each to that harness's
expected path (e.g. `.claude/skills/my-agent/SKILL.md`). For fuller worked examples,
see [`examples/`](examples/).

### Build

```bash
skillprism build
```

Output is written to project-level agent paths (e.g., `.claude/skills/`, `.opencode/skills/`).

### Preview changes

```bash
skillprism build --diff
```

Shows a unified diff of what would be written without modifying any files.

### Validate

```bash
skillprism validate
```

Checks templates for syntax errors, undefined variables, and missing macros without writing output. Also enforces [Agent Skills spec](https://agentskills.io/specification) constraints: skill name format (`^[a-z0-9]+(-[a-z0-9]+)*$`, must match the directory name), non-empty description, and per-harness length caps. Values over the spec's portable cap but within a harness's own cap (e.g. a 1200-char description for Claude) are reported as warnings, not errors.

## Distribution workflow

skillprism can install skills from remote Git repositories or local paths, manage their lifecycle, and keep them up to date — all while rendering each skill once per harness.

### Install skills

```bash
# From a GitHub shorthand
skillprism add owner/repo

# From a full Git URL (GitHub, GitLab, etc.)
skillprism add https://github.com/owner/repo.git

# Pin to a specific ref or filter to one skill from a multi-skill repo
skillprism add owner/repo#v1.0.0
skillprism add owner/repo --skill my-skill

# From a local path
skillprism add ./path/to/skills

# Filter which harnesses to install to
skillprism add owner/repo -H claude,opencode
```

Each skill is either **skillprism-format** (has `skill.yaml` with `skillprism: '1'` — rendered through MiniJinja per harness) or **plain-format** (bare `SKILL.md` — copied as-is). The format is auto-detected per skill.

### List installed skills

```bash
skillprism list              # all installed
skillprism list --target user  # only user-scoped
skillprism list -H claude      # only claude harness
```

### Remove skills

```bash
skillprism remove my-skill        # remove one skill
skillprism remove --all           # remove everything
skillprism remove --all --force   # skip confirmation
```

### Update skills

```bash
skillprism update                 # check all installed skills for updates
skillprism update my-skill        # update a specific skill
skillprism update --diff          # preview changes without writing
skillprism update -H claude       # only update claude harness files
```

Update performs a lightweight up-to-date check via `git ls-remote` before cloning, then re-renders only files whose content actually changed (SHA-256 comparison).

## Spec compliance

skillprism renders `SKILL.md` files with the YAML frontmatter the [Agent Skills specification](https://agentskills.io/specification) requires (`name` + `description` at minimum). Without this frontmatter, no compatible agent can discover a skill. The scaffold (`init skill`, `init project`) and `validate` both enforce spec constraints so a successful `skillprism build` produces skills that load in any spec-compatible client.

## Examples

[`examples/`](examples/) exists to show off skillprism's own mechanics, not to be a
ready-to-use skill library — see [`examples/README.md`](examples/README.md) for all
three:

- **`skills/quickstart/`** — a small, deliberately synthetic skill meant to be read
  end-to-end in one sitting: built-ins, custom variables, harness macros, and
  `skill.yaml`'s per-harness `harnesses:` override block, all exercised in a single
  short file. Start here.
- **`skills/mcp-builder/`** and **`skills/webapp-testing/`** — two real skills ported
  from Anthropic's public Agent Skills repository, compiled to `claude`, `opencode`,
  and `codex` from one shared source each. A writeup of three gaps between
  skillprism's documented schema and its implementation that surfaced — and were
  fixed — while porting them is in `examples/README.md`.

## Supported Harnesses

| Harness | Description |
|---------|-------------|
| `claude` | Claude Code (`.claude/skills/`) |
| `codex` | OpenAI Codex (`.agents/skills/`) |
| `opencode` | OpenCode (`.opencode/skills/`) |
| `factory` | Factory (`.factory/skills/`) |
| `pi` | Pi (`.pi/skills/`) |

## CLI Reference

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

### Global flags

- `-v`, `--verbose`: Enable verbose progress output with per-phase timing

### Build flags

- `--target`: Output scope — `project` (default), `user` (global), or `dist` (inspection)
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
│       ├── skill.yaml       # Skill metadata
│       ├── SKILL.md         # Template (MiniJinja; SKILL.md.j2 also accepted)
│       ├── references/      # Shared assets
│       └── scripts/
└── harnesses/               # User harness overrides (optional)
```

## Development

### Prerequisites

- Rust 1.85+ (edition 2024)
- [devenv](https://devenv.sh/) (optional, for reproducible environments)

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
cargo clippy -- -D warnings
cargo fmt --check
```

### Pre-commit Hooks

This project includes pre-commit hooks for `cargo fmt` and `cargo clippy`,
managed through devenv. Hooks install automatically when entering the
environment via `devenv shell` or `direnv`.

To run hooks manually:

```bash
devenv shell
pre-commit run --all-files
```

Or via devenv's CI integration:

```bash
devenv test
```

### Documentation

```bash
cargo doc --no-deps --document-private-items
```

## License

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

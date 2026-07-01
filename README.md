# skillprism

[![CI](https://github.com/tuvren/skillprism/actions/workflows/ci.yml/badge.svg)](https://github.com/tuvren/skillprism/actions/workflows/ci.yml)

Build-time compiler that transforms canonical skill sources into harness-specific agent files.

## Installation

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
- **`skill.yaml` fields**, available under their own name — `license`, `version`,
  `compatibility`, `when_to_use`, `metadata.*`, `allowed_tools`, and more (full list:
  `.constitution/tech-spec/contracts/skill-schema.json`). You don't have to reference
  every field in your template — some, like `compatibility` or `metadata.source`, are
  just packaging/attribution metadata that tooling can read without it appearing in the
  rendered body.
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

Checks templates for syntax errors, undefined variables, and missing macros without writing output.

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
skillprism build [--target project|user|dist] [--diff] [--force]
skillprism validate [path]
skillprism init project <name> [--out <dir>]
skillprism init skill <name> [--targets <harnesses>]
```

### Build flags

- `--target`: Output scope — `project` (default), `user` (global), or `dist` (inspection)
- `--diff`: Show colored diff preview without writing files
- `--force`: Overwrite existing user-scope files without warning

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
cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery
cargo fmt --check
```

### Pre-commit Hooks

This project includes pre-commit hooks for `cargo fmt` and `cargo clippy`,
managed through devenv. Hooks install automatically when entering the
environment via `devenv shell` or `direnv`.

To run hooks manually:

```bash
devenv shell
prek run --all-files
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

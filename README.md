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

Edit `skills/my-agent/SKILL.md.j2` to define your skill template.

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
│       ├── SKILL.md.j2      # Template (MiniJinja)
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

---
title: "Harnesses"
description: "The 5 built-in harnesses, custom harnesses, capability gating"
group: "Authoring"
weight: 60
---
A harness is an agent product that reads skills. skillprism ships with 5 built-in harness definitions and supports custom on-demand harness definitions.

## Built-in harnesses

| Harness | ID | Project path | User path | Manifest |
|---------|----|-------------|-----------|----------|
| Claude Code | `claude` | `.claude/skills/` | `~/.claude/skills/` | `.claude/plugin.json` |
| OpenAI Codex | `codex` | `.agents/skills/` | `~/.codex/skills/` | `.agents/marketplace.json` |
| OpenCode | `opencode` | `.agents/skills/` | `$XDG_CONFIG_HOME/opencode/skills/` | (none) |
| Factory | `factory` | `.agents/skills/` | `~/.factory/skills/` | (none) |
| Pi | `pi` | `.agents/skills/` | `~/.agents/skills/` | (none) |

OpenCode's user directory is `$XDG_CONFIG_HOME/opencode/skills`, or `~/.config/opencode/skills/` when `XDG_CONFIG_HOME` is unset. Codex, OpenCode, Factory, and Pi share the project directory `.agents/skills/`. `claude-code` selects Claude and `droid` selects Factory; the stored ids stay `claude` and `factory`.

## Custom harnesses (Advanced)

Custom harness definitions are advanced and created on demand in the project's `harnesses/` directory:

```bash
skillprism init harness my-agent
```

This scaffolds `harnesses/my-agent.yaml`. Custom harness YAML definitions placed under `harnesses/` are automatically loaded during `build`, `add`, and `update` commands.

```yaml
# yaml-language-server: $schema=https://tuvren.github.io/skillprism/schema/v1/harness.json
id: my-agent
name: my-agent
capabilities:
  supports_subagent: false
  requires_sidecar: false
  requires_manifest: false
  name_max_length: 64
  description_max_length: 1024
paths:
  project_scope_path: ".my-agent/skills"
  user_scope_path: ".my-agent/skills"
  skill_filename: SKILL.md
```

The first-line comment links the [harness definition schema](https://tuvren.github.io/skillprism/schema/v1/harness.json) for editor completion and field validation. Install the [VS Code YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml) to read the modeline; in [JetBrains IDEs](https://www.jetbrains.com/help/idea/yaml.html), select the schema URL through **JSON Schema Mappings**.

`/schema/v1/` follows the latest published docs within configuration format version 1, so the editor can accept fields added after your installed skillprism release before your binary supports them.

Edit the values to match your agent product's conventions, then add `my-agent` to `skillprism.yaml`'s `harnesses:` list.

## Capability matrix

| Capability | claude | codex | opencode | factory | pi |
|-----------|--------|-------|----------|---------|-----|
| `subagent` | ✓ | ✓ | ✓ | ✓ | ✗ |
| `allowed-tools` | ✓ | ✗ | ✗ | ✗ | ✗ |
| `disable-model-invocation` | ✓ | ✗ | ✗ | ✓ | ✓ |
| `user-invocable` | ✓ | ✗ | ✗ | ✓ | ✗ |
| `manifest` | ✓ | ✓ | ✗ | ✗ | ✗ |
| `sidecar` | ✗ | ✓ | ✗ | ✗ | ✓ |

## Length caps

| Harness | Name max | Description max |
|---------|----------|----------------|
| `claude` | 64 | 1536 |
| `codex` | 100 | 500 |
| `opencode` | 64 | 1024 |
| `factory` | 64 | 1024 |
| `pi` | 64 | 1024 |

The spec's portable caps are 64 (name) and 1024 (description). Values over the spec cap but within a harness's own cap are reported as **warnings**, not errors — the skill builds for that harness but may not be portable to stricter ones.

## required-capabilities

A skill can declare capabilities it needs:

```yaml
# skill.yaml
required-capabilities:
  - subagent
  - allowed-tools
```

If a harness doesn't support a required capability, that skill-harness pair is **skipped** with a `[resolve] skipped: ...` warning — not a build failure. Every other pair still builds.

This lets you write one skill that targets `claude` (which supports `allowed-tools`) while gracefully degrading for harnesses that don't — without maintaining separate source files.

### Harness macros

Custom harnesses can define macros — text snippets exposed as `{{ harness.<name> }}`:

```yaml
macros:
  subagent_guide:
    content: "## Subagent Instructions\n\nMy agent runs skills as isolated processes."
  setup_note:
    content: "## Setup\n\nInstall my-agent from npm."
```

### Manifests

If your harness needs a manifest file (a JSON index of all skills), define it:

```yaml
manifest:
  template: |
    {
      "name": {{ skill_name | tojson }},
      "description": {{ skill_description | tojson }}
    }
paths:
  manifest_scope_path: .my-agent
  manifest_filename: index.json
```

`build` aggregates all rendered skills for that harness into a single manifest file at the configured path.

Apply the `tojson` filter to each interpolated string value without surrounding quotes. It escapes quotes, backslashes, and newlines. Build rejects invalid manifest JSON before writing any output and reports the skill, harness, manifest path, and JSON error. If you migrate a hand-quoted template, remove the surrounding quotes and the unsupported `format: json` key.

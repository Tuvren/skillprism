---
title: "Harnesses"
description: "The 5 built-in harnesses, custom harnesses, capability gating"
group: "Authoring"
weight: 60
---
A harness is an agent product that reads skills. skillprism ships with 5 built-in harness definitions and supports custom ones.

## Built-in harnesses

| Harness | ID | Project path | User path | Manifest |
|---------|----|-------------|-----------|----------|
| Claude Code | `claude` | `.claude/skills/` | `~/.claude/skills/` | `.claude/plugin.json` |
| OpenAI Codex | `codex` | `.agents/skills/` | `~/.codex/skills/` | `.agents/marketplace.json` |
| OpenCode | `opencode` | `.opencode/skills/` | `~/.config/opencode/skills/` | (none) |
| Factory | `factory` | `.factory/skills/` | `~/.factory/skills/` | (none) |
| Pi | `pi` | `.pi/skills/` | `~/.pi/agent/skills/` | (none) |

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

## Custom harnesses

You can define custom harnesses in the `harnesses/` directory:

```bash
skillprism init harness my-agent
```

This scaffolds `harnesses/my-agent.yaml`:

```yaml
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

Edit the values to match your agent product's conventions, then add `my-agent` to `skillprism.yaml`'s `harnesses:` list.

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
  format: json
  template: |
    {
      "name": "{{ skill_name }}",
      "description": "{{ skill_description }}"
    }
paths:
  manifest_scope_path: .my-agent
  manifest_filename: index.json
```

skillprism aggregates all rendered skills for that harness into a single manifest file at the configured path.

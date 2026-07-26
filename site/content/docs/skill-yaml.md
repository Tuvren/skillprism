---
title: "skill.yaml reference"
description: "Every metadata field and format requirements"
group: "Authoring"
weight: 40
---
`skill.yaml` is the metadata specification for skillprism-format skills. Every field defined in `skill.yaml` is available in `SKILL.md` as a template variable under its own name (with three exceptions noted below).

## Skill Format Detection

skillprism supports two skill formats when building or installing skills:

1. **skillprism-format**: Contains `skill.yaml` (with `skillprism: '1'`) and `SKILL.md` (or `SKILL.md.j2`). Rendered through MiniJinja per harness.
2. **plain-format**: Bare `SKILL.md` file without `skill.yaml`. Installed directly as-is without templating.

> **Important:** `skill.yaml` requires `skillprism: '1'` at the top level to declare compliance with the skillprism spec.

```yaml
skillprism: '1'
name: dice-roller
description: Roll dice using a random number generator.
```

## Required fields

| Field | Type | Constraint | Template variable |
|-------|------|-----------|-------------------|
| `skillprism` | string | Must be `'1'` | (Format header) |
| `name` | string | 1-64 chars, `^[a-z0-9]+(-[a-z0-9]+)*$`, must match directory name | `{{ skill_name }}` |
| `description` | string | 1-1024 chars (spec); per-harness cap may be higher | `{{ skill_description }}` |

## Optional fields

| Field | Type | Constraint | Template variable |
|-------|------|-----------|-------------------|
| `version` | string | SemVer (`x.y.z`) | `{{ version }}` |
| `license` | string | License name or file reference | `{{ license }}` |
| `compatibility` | string | ≤500 chars; environment requirements | `{{ compatibility }}` |
| `metadata` | map<string,string> | Arbitrary key-value metadata | `{{ metadata.<key> }}` |
| `allowed-tools` | string | Pre-approved tools (experimental) | `{{ allowed_tools }}` |
| `when_to_use` | string | Trigger phrases (Claude Code) | `{{ when_to_use }}` |
| `argument-hint` | string | Autocomplete hint, e.g. `[issue-number]` | `{{ argument_hint }}` |
| `arguments` | list | Named positional arguments | `{{ arguments }}` |
| `disable-model-invocation` | bool | Prevent automatic loading | `{{ disable_model_invocation }}` |
| `user-invocable` | bool | Show/hide from `/` menu | `{{ user_invocable }}` |
| `disallowed-tools` | list | Tools removed while skill is active | `{{ disallowed_tools }}` |
| `model` | string | Model override, e.g. `claude-sonnet-4-20250514` | `{{ model_override }}` |
| `effort` | enum | `low`/`medium`/`high`/`xhigh`/`max` | `{{ effort }}` |
| `context` | enum | `fork` (runs in separate subagent context) | `{{ context_fork }}` (boolean) |
| `agent` | string | Subagent type when context is fork | `{{ agent }}` |
| `hooks` | map | Lifecycle hooks (Claude Code) | `{{ hooks }}` |
| `paths` | list | Glob patterns limiting activation | `{{ activation_paths }}` |
| `shell` | enum | `bash`/`powershell` | `{{ shell }}` |
| `required-capabilities` | list | Harness capabilities this skill needs | `{{ required_capabilities }}` |
| `variables` | map | Custom template values | `{{ <key> }}` (by name) |

## Template Context Aliases

Three fields are exposed under explicit alias names in MiniJinja template context alongside their direct YAML keys:

| `skill.yaml` key | Primary Alias | Direct Key | Description |
|------------------|---------------|------------|-------------|
| `model` | `{{ model_override }}` | `{{ model }}` | Model string (e.g. `claude-sonnet-4-20250514`) |
| `paths` | `{{ activation_paths }}` | `{{ paths }}` | Activation path list |
| `context` | `{{ context_fork }}` | `{{ context }}` | `context_fork` is a boolean (`true` when `context: fork`); `context` is the raw string (`"fork"` or `"inline"`) |

Both direct keys and alias names are supported in templates.

## Variables

The `variables:` map is your own custom data, available by name in `SKILL.md`:

```yaml
variables:
  port: 5173
  greeting: Hello from my-agent
```

```jinja
Port: {{ port }}
{{ greeting }}
```

Use `variables:` when a value is constant across every harness. If a variable genuinely needs a *different* value depending on which harness is being built, use the `overrides:` block (see [Templating](../templating)).

## overrides block

Per-harness overrides for variables and macros:

```yaml
variables:
  port: 5173
overrides:
  opencode:
    variables:
      port: 4173      # only opencode's render sees this value
    macros:
      extra_note: "OpenCode-specific note, exposed as {{ harness.extra_note }}"
```

See [Templating → Per-harness overrides](../templating/#per-harness-overrides) for details.

## Validation

`skillprism validate` checks:

- Presence of `skillprism: '1'` header
- Name format: `^[a-z0-9]+(-[a-z0-9]+)*$` (lowercase, digits, hyphens; no leading/trailing/consecutive hyphens)
- Name matches directory name (spec requirement)
- Description is non-empty
- Description within harness cap (hard error) and spec cap (warning if over)
- Compatibility ≤500 chars
- Template syntax, undefined variables, undefined macros

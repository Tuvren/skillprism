---
title: "skill.yaml reference"
description: "Every metadata field and what it does"
---

# skill.yaml reference

`skill.yaml` is the metadata half of a skill. Every field is available in `SKILL.md` as a template variable under its own name (with three exceptions noted below).

## Required fields

| Field | Type | Constraint | Template variable |
|-------|------|-----------|-------------------|
| `name` | string | 1-64 chars, `^[a-z0-9]+(-[a-z0-9]+)*$`, must match directory name | `{{ skill_name }}` |
| `description` | string | 1-1024 chars (spec); per-harness cap may be higher | `{{ skill_description }}` |

## Optional fields

| Field | Type | Constraint | Template variable |
|-------|------|-----------|-------------------|
| `version` | string | SemVer (`x.y.z`) | `{{ version }}` |
| `license` | string | License name or file reference | `{{ license }}` |
| `compatibility` | string | ≤500 chars; environment requirements | `{{ compatibility }}` |
| `metadata` | map<string,string> | Arbitrary key-value metadata | `{{ metadata.<key> }}` |
| `allowed-tools` | string or list | Pre-approved tools (experimental) | `{{ allowed_tools }}` |
| `when_to_use` | string | Trigger phrases (Claude Code) | `{{ when_to_use }}` |
| `argument-hint` | string | Autocomplete hint, e.g. `[issue-number]` | `{{ argument_hint }}` |
| `arguments` | string or list | Named positional arguments | `{{ arguments }}` |
| `disable-model-invocation` | bool | Prevent automatic loading | `{{ disable_model_invocation }}` |
| `user-invocable` | bool | Show/hide from `/` menu | `{{ user_invocable }}` |
| `disallowed-tools` | string or list | Tools removed while skill is active | `{{ disallowed_tools }}` |
| `model` | string | Model override, e.g. `claude-sonnet-4-20250514` | `{{ model_override }}` |
| `effort` | enum | `low`/`medium`/`high`/`xhigh`/`max` | `{{ effort }}` |
| `context` | enum | `fork` (runs in separate subagent context) | `{{ context_fork }}` (boolean) |
| `agent` | string | Subagent type when context is fork | `{{ agent }}` |
| `hooks` | map | Lifecycle hooks (Claude Code) | `{{ hooks }}` |
| `paths` | string or list | Glob patterns limiting activation | `{{ activation_paths }}` |
| `shell` | enum | `bash`/`powershell` | `{{ shell }}` |
| `required-capabilities` | list | Harness capabilities this skill needs | `{{ required_capabilities }}` |
| `variables` | map | Custom template values | `{{ <key> }}` (by name) |

## Renamed fields

Three fields render under a different name than their `skill.yaml` key:

| `skill.yaml` key | Template variable | Why |
|------------------|-------------------|-----|
| `model` | `{{ model_override }}` | Avoids collision with Jinja2's `model` namespace |
| `paths` | `{{ activation_paths }}` | Avoids collision with path-related builtins |
| `context` | `{{ context_fork }}` | Derived boolean (`true` when `context: fork`), not the raw string |

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

Use `variables:` when a value is constant across every harness. If a variable genuinely needs a *different* value depending on which harness is being built, use the `harnesses:` block (see [Templating](../templating)).

## harnesses block

Per-harness overrides for variables and macros:

```yaml
variables:
  port: 5173
harnesses:
  opencode:
    variables:
      port: 4173      # only opencode's render sees this value
    macros:
      extra_note: "OpenCode-specific note, exposed as {{ harness.extra_note }}"
```

See [Templating → Per-harness overrides](../templating/#per-harness-overrides) for details.

## Validation

`skillprism validate` checks:

- Name format: `^[a-z0-9]+(-[a-z0-9]+)*$` (lowercase, digits, hyphens; no leading/trailing/consecutive hyphens)
- Name matches directory name (spec requirement)
- Description is non-empty
- Description within harness cap (hard error) and spec cap (warning if over)
- Compatibility ≤500 chars
- Template syntax, undefined variables, undefined macros

## Spec mapping

The Agent Skills spec defines frontmatter fields (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`). skillprism maps `skill.yaml` fields to these in the rendered `SKILL.md` frontmatter — your template controls which fields appear in the output by referencing them.

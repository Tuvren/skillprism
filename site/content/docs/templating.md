---
title: "Templating"
description: "MiniJinja built-ins, variables, harness macros, per-harness overrides"
---

# Templating

`SKILL.md` is a [MiniJinja](https://docs.rs/minijinja) template — a Rust implementation of Jinja2 syntax. It's rendered once per configured harness, with the `skill.yaml` metadata and harness context available inside `{{ }}`.

## Built-in variables

Always present in every render:

| Variable | Source | Example |
|----------|--------|---------|
| `{{ skill_name }}` | `skill.yaml` `name` | `dice-roller` |
| `{{ skill_description }}` | `skill.yaml` `description` | `Roll dice using...` |
| `{{ harness.id }}` | Harness definition | `claude` |
| `{{ harness.name }}` | Harness definition | `Claude Code` |
| `{{ harness.<macro> }}` | Harness macros (see below) | `{{ harness.subagent_guide }}` |

## skill.yaml fields

Every `skill.yaml` field is available under its own name (with three [renamed exceptions](../skill-yaml/#renamed-fields)):

```jinja
---
name: {{ skill_name }}
description: {{ skill_description }}
license: {{ license }}
allowed-tools: {{ allowed_tools }}
---

# {{ skill_name }}

{{ skill_description }}

## When to use

{{ when_to_use }}
```

Unset optional fields render as empty (not the string `"none"`), so a missing `license` produces `license: ` in the frontmatter — not `license: none`.

## Custom variables

Anything under `variables:` in `skill.yaml` is available by name:

```yaml
# skill.yaml
variables:
  port: 5173
  framework: Vite
```

```jinja
# SKILL.md
Default port: {{ port }}
Framework: {{ framework }}
```

## Harness macros

Each harness defines macros — text snippets exposed as `{{ harness.<name> }}`. Every built-in harness defines at least `subagent_guide`:

```jinja
{{ harness.subagent_guide }}
```

This renders harness-specific subagent instructions without you writing per-harness conditionals. See `src/builtin_harnesses/*.yaml` in the repo for what each harness defines.

### Built-in harness fields

These are always available on the `harness` object:

| Field | Description |
|-------|-------------|
| `{{ harness.id }}` | Harness identifier (`claude`, `opencode`, etc.) |
| `{{ harness.name }}` | Display name (`Claude Code`, `OpenCode`, etc.) |
| `{{ harness.version }}` | Harness definition version |
| `{{ harness.skill_ref_pattern }}` | Skill reference pattern (`/{name}`) |

## Conditional content

Use `{% if %}` for harness-specific instructions when macros aren't enough:

```jinja
{% if harness.id == "claude" %}
This section only appears in Claude Code's output.
{% elif harness.id == "opencode" %}
This section only appears in OpenCode's output.
{% endif %}
```

Don't reach for conditionals just because they exist — most skills render identically everywhere. Use `{{ harness.subagent_guide }}` for the common case (harness-specific subagent text). Use `{% if %}` only for genuinely harness-specific instructions.

## Per-harness overrides

The `harnesses:` block in `skill.yaml` overrides variables and macros per harness:

```yaml
# skill.yaml
variables:
  greeting: Hello from skillprism
harnesses:
  opencode:
    variables:
      greeting: Hello from skillprism, rendered specifically for OpenCode
  codex:
    macros:
      subagent_guide: "## Subagent Instructions\n\nThis skill overrides Codex's own subagent_guide macro for itself only."
```

- **`harnesses.<id>.variables`** — merged with top-level `variables`, harness wins. Only that harness's render sees the override.
- **`harnesses.<id>.macros`** — overrides a harness's builtin macro for this skill only. Other skills built for the same harness still get the harness's unmodified builtin.

## Helper functions

skillprism registers one custom helper:

| Function | Description | Example |
|----------|-------------|---------|
| `skill_ref(name)` | Formats a skill reference for the current harness | `{{ skill_ref(skill_name) }}` → `/my-agent` |

## Gotchas

- **Validation checks all branches:** `{% if %}` references are checked statically, including both branches. A macro override referenced anywhere in the template must be defined for *every* harness in `skillprism.yaml`'s `harnesses:` list — not just the one it's meaningfully different for — even if you only ever read it behind a guard.
- **Variable name collisions:** A `variables:` entry with the same name as a built-in (`version`, `license`, etc.) silently overwrites it. `skillprism validate` catches this as a `ReservedVariableName` error.
- **Don't over-use `harnesses:` and `variables:`:** Most skills render identically everywhere and don't need per-harness overrides at all. The scaffold defaults to showing them as commented optional examples for this reason.

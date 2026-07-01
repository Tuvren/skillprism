---
name: {{ skill_name }}
description: {{ skill_description }}
---

# {{ skill_name }}

{{ skill_description }}

This skill doesn't do anything — it exists purely to show what a rendered `SKILL.md`
looks like once every skillprism mechanism has been exercised at least once, in one
short file you can read end-to-end.

## Built-ins

`{{ '{{ skill_name }}' }}` and `{{ '{{ skill_description }}' }}` come straight from
this skill's `skill.yaml`. `{{ '{{ harness.id }}' }}` and `{{ '{{ harness.name }}' }}`
come from whichever harness is currently being rendered for — right now, that's
**{{ harness.name }}** (`{{ harness.id }}`).

## Variables and per-harness overrides

`{{ greeting }}` is defined once under `skill.yaml`'s top-level `variables:`, with a
`harnesses.opencode.variables.greeting` override on top. Building this skill for
claude or codex renders the top-level default; building it for opencode renders the
override instead — same template, no conditional branching needed for values that
only differ by harness.

Current value: **{{ greeting }}**

## Harness macros and per-skill macro overrides

{% if harness.id == "codex" -%}
This skill also overrides Codex's own `subagent_guide` macro for itself only, on top
of the variable override above — every *other* skill built for Codex still gets
Codex's unmodified builtin text.
{% else -%}
The section below comes from {{ harness.name }}'s own builtin `subagent_guide` macro,
unmodified — only Codex has a per-skill override for it (see this skill's
`skill.yaml`, under `harnesses.codex.macros`).
{% endif %}
{{ harness.subagent_guide }}

## Assets

`references/note.md` is a real file in this skill's own directory, copied verbatim
into every harness's output alongside this rendered `SKILL.md` — skillprism copies
any subdirectory here regardless of what it's named.

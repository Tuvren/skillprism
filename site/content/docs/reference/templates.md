---
title: "Templates"
description: "Template context, aliases, macros, and filters."
group: "Reference"
weight: 44
aliases:
  - /docs/templating/
---

`SKILL.md` is the skill template. A skill directory has `SKILL.md` or `SKILL.md.j2`, not both. skillprism fills the template once for each harness. A missing value is empty, not the word `none`.

Template syntax is [Jinja](https://jinja.palletsprojects.com/en/stable/templates/). `skill_ref` is reserved, so a `variables` key of that name is rejected.

Field meanings are in [skill.yaml](skill-yaml/). Harness macros are in [Harness](harness/).

## Rendered frontmatter

`build`, `add`, and `update` reject a rendered header that is not a closed YAML mapping. A missing fence, an empty header, a list, a bare scalar, or a value that does not parse fails. An unquoted colon is the usual parse failure. Optional spec fields are written only when the template emits them.

`name` and `description` must render as YAML strings. An unquoted `123`, `true`, `false`, `null`, or `~` is a number, a boolean, or null. `yes` stays a string. skillprism does not coerce those values into text. The check runs again after YAML merges.

## Context variables

These names are present for every render.

| Name | Value |
|------|-------|
| `skill_name` | Skill name. |
| `skill_description` | Skill description. |
| `version` | `version`, or undefined. |
| `license` | `license`, or undefined. |
| `compatibility` | `compatibility`, or undefined. |
| `metadata` | `metadata` map. An omitted field is an empty map. |
| `allowed_tools` | `allowed-tools`, or undefined. |
| `when_to_use` | `when_to_use`, or undefined. |
| `argument_hint` | `argument-hint`, or undefined. |
| `arguments` | `arguments`, or undefined. |
| `disable_model_invocation` | `disable-model-invocation`, or undefined. |
| `user_invocable` | `user-invocable`, or undefined. |
| `disallowed_tools` | `disallowed-tools`, or undefined. |
| `model` | `model`, or undefined. |
| `model_override` | Same value as `model`. |
| `effort` | `effort`, or undefined. |
| `context` | `fork` when `context` is `fork`; otherwise `inline`. |
| `context_fork` | Boolean. True only when `context` is `fork`. |
| `agent` | `agent`, or undefined. |
| `hooks` | `hooks`, or undefined. |
| `paths` | `paths`, or undefined. |
| `activation_paths` | Same value as `paths`. |
| `shell` | `shell`, or undefined. |
| `required_capabilities` | `required-capabilities`. An omitted field is an empty list. |
| `harness` | Object below. |
| each `variables` key | That key's value after the harness override merge. |

Reserved names, which `variables` must not use: `skill_name`, `skill_description`, `skill_ref`, and every name in the table except `harness` and the variable keys themselves (`version` through `required_capabilities`, including `model_override`, `context_fork`, and `activation_paths`).

A name in the template must be one of these variables, `skill_ref`, `harness`, or a Jinja built-in. `validate` checks both sides of an `{% if %}`.

## Aliases

Three fields are inserted under two names.

| `skill.yaml` key | Names | Values |
|------------------|-------|--------|
| `model` | `model`, `model_override` | The same string. |
| `paths` | `paths`, `activation_paths` | The same list. |
| `context` | `context`, `context_fork` | `context` is `fork` or `inline`. `context_fork` is a boolean. |

## Harness object

| Name | Value |
|------|-------|
| `harness.id` | Harness id. |
| `harness.name` | Display name. |
| `harness.version` | Definition version, when set. |
| `harness.skill_ref_pattern` | Pattern string, when set. |
| `harness.<macro>` | Macro text. A skill `overrides.<id>.macros` value replaces the harness macro of the same name for that skill. |

## `skill_ref`

`skill_ref(name)` replaces `{name}` in the current harness's `skill_ref_pattern`. With no pattern, the pattern is `/{name}`. The name `skill_ref` is reserved.

## Filters

### `yaml_str`

Use `yaml_str` on every string you place in the YAML frontmatter of `SKILL.md`. It quotes the value, so a description that contains a colon or a quote stays text.

An empty value becomes `null`. A list or a map is an error. Put those outside the frontmatter.

```jinja
description: {{ skill_description | yaml_str }}
```

### `tojson`

`tojson` writes a value as JSON. In a manifest template, apply it to each string and do not wrap the expression in quotes. See [Harness](harness/#manifests).

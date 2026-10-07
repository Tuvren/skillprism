---
title: "skill.yaml"
description: "Fields of a skillprism-format skill manifest."
group: "Reference"
weight: 42
aliases:
  - /docs/skill-yaml/
---

`skill.yaml` describes one skill. Editors can use the schema at <https://tuvren.github.io/skillprism/schema/v1/skill.json>. Unknown fields are rejected.

`skillprism: '1'` marks a skill that skillprism compiles. The value is the string `1` or the number `1`. A directory with `SKILL.md` and no `skill.yaml` is copied as-is: `add` does not fill a template.

A skill directory under `skills_dir` with no `skill.yaml` still compiles. Its name is the directory name and its description is empty, so `validate` reports that empty description. A directory that contains both `SKILL.md` and `SKILL.md.j2` is an error.

A directory that has `skill.yaml` and no template, and that contains child skill directories, is a group. Its `variables` merge into each child. The child's value replaces the parent's value for the same key. A group file still requires `skillprism`.

`name` and `description` are optional at load time. A group file may omit both. A leaf that omits `name` uses the directory name. A leaf that omits `description` gets an empty description.

Template names are the names in [Templates](templates/). Hyphenated keys are exposed with underscores.

## `skillprism`

| | |
|---|---|
| Type | string `"1"` or integer `1` |
| Required | yes |

Format version. The only accepted value is 1.

## `name`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `skill_name` |

Skill name. `validate` and `build` require lowercase letters, digits, and single hyphens (`^[a-z0-9]+(-[a-z0-9]+)*$`), and the name must match the directory name. A name longer than that harness allows is an error. A name longer than 64 characters is a warning when the harness allows more. Length counts characters, not bytes.

## `description`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `skill_description` |

Description. An empty description is a validation error. Length is checked against the harness `description_max_length` (error) and the spec cap of 1024 characters (warning when the harness cap is higher).

## `version`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `version` |

Version string. Any string.

## `license`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `license` |

License name or a reference to a bundled license file.

## `compatibility`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `compatibility` |

Environment requirements. When the field is present, validation requires 1–500 characters. There is no per-harness cap.

## `required-capabilities`

| | |
|---|---|
| Type | array of strings, or null |
| Required | no |
| Template | `required_capabilities` |

Capability names the harness must provide. A missing capability skips that skill-harness pair. The names and the skip rule are in [Harness](harness/). Use a list of strings. A single string is invalid.

## `metadata`

| | |
|---|---|
| Type | object of string to string, or null |
| Required | no |
| Template | `metadata` |

Key-value metadata. Omitted means an empty map in the template context.

## `allowed-tools`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `allowed_tools` |

Pre-approved tools. Use a string. A list is invalid.

## `variables`

| | |
|---|---|
| Type | object, or null |
| Required | no |
| Template | each key, by that key |

Values inserted into the template context under their own names. A harness entry under `overrides` replaces the same key for that harness only. A key equal to a built-in context name or to `skill_ref` is a validation error.

## `when_to_use`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `when_to_use` |

Trigger phrases.

## `argument-hint`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `argument_hint` |

Autocomplete hint, such as `[issue-number]`.

## `arguments`

| | |
|---|---|
| Type | array of strings, or null |
| Required | no |
| Template | `arguments` |

Named positional arguments. Use a list of strings. A single string is invalid.

## `disable-model-invocation`

| | |
|---|---|
| Type | boolean or null |
| Required | no |
| Template | `disable_model_invocation` |

When true, the skill is not loaded automatically. Omission leaves the value unset. No default is filled in.

## `user-invocable`

| | |
|---|---|
| Type | boolean or null |
| Required | no |
| Template | `user_invocable` |

When false, the skill is hidden from the `/` menu. Omission leaves the value unset.

## `disallowed-tools`

| | |
|---|---|
| Type | array of strings, or null |
| Required | no |
| Template | `disallowed_tools` |

Tools removed while the skill is active. Use a list of strings. A single string is invalid.

## `model`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `model` and `model_override` |

Model override. Both template names hold the same string.

## `effort`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `effort` |

Effort hint. Any string.

## `context`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `context` and `context_fork` |

`context_fork` is true only when the value is `fork`. Any other string, including omission, leaves `context_fork` false. The template value `context` is the string `fork` when forking is set and the string `inline` otherwise. It is not the raw YAML text of any other value.

## `agent`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `agent` |

Subagent type used when `context` is `fork`.

## `hooks`

| | |
|---|---|
| Type | object, or null |
| Required | no |
| Template | `hooks` |

Lifecycle hooks. Values may be any YAML value.

## `paths`

| | |
|---|---|
| Type | array of strings, or null |
| Required | no |
| Template | `paths` and `activation_paths` |

Glob patterns that limit activation. Both template names hold the same list. Use a list of strings. A single string is invalid.

## `shell`

| | |
|---|---|
| Type | string or null |
| Required | no |
| Template | `shell` |

Shell for command injection. Any string.

## `overrides`

| | |
|---|---|
| Type | object, or null |
| Required | no |

Per-harness block. Each key is a harness id. Unknown keys inside an entry are rejected. Rendering a different value per harness is covered in [Vary output per harness](../how-to/vary-output-per-harness/).

### `overrides.<id>.variables`

| | |
|---|---|
| Type | object |
| Required | no |

Merged over the top-level `variables` map for that harness. The harness value wins.

### `overrides.<id>.macros`

| | |
|---|---|
| Type | object of string to string |
| Required | no |

Replaces the harness macro of the same name for this skill only. Other skills keep the harness definition.

## Legacy `harnesses` key

A top-level `harnesses` key in `skill.yaml` is rejected. Per-harness values belong under `overrides`. The same key inside `overrides` is also rejected.

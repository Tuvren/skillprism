---
title: "Harness"
description: "Harness definition fields and the five built-in harnesses."
group: "Reference"
weight: 43
aliases:
  - /docs/harnesses/
---

A harness is one agent: its id, the directories it reads, and any extra files it needs. Editors can use the schema at <https://tuvren.github.io/skillprism/schema/v1/harness.json>. Unknown fields are rejected.

Built-in ids are `claude`, `codex`, `opencode`, `factory`, and `pi`. Files `harnesses/*.yaml` and `harnesses/*.yml` load as user harnesses and replace a built-in with the same `id`. `init harness` writes that file. The steps are in [Add a custom harness](../how-to/add-custom-harness/).

`claude-code` selects `claude` and `droid` selects `factory`. An exact registered id wins: a user harness whose id is `claude-code` or `droid` is that harness, and the built-in alias is not applied.

Output directories for the built-ins are in [Directories and state](directories/).

## `id`

| | |
|---|---|
| Type | string |
| Required | yes |

Harness identifier. `build` uses it as the `dist/<id>/` directory name.

## `name`

| | |
|---|---|
| Type | string |
| Required | yes |

Display name. The template sees it as `harness.name`.

## `version`

| | |
|---|---|
| Type | string or null |
| Required | no |

Definition version. Any string. When set, the template sees it as `harness.version`.

## `capabilities`

| | |
|---|---|
| Type | object |
| Required | yes |

`supports_subagent` is required. The other fields are optional.

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `supports_subagent` | boolean | — | Subagent support. Capability name: `subagent`. |
| `requires_sidecar` | boolean | `false` | Sidecar support flag. Capability name: `sidecar`. |
| `requires_manifest` | boolean | `false` | Manifest support flag. Capability name: `manifest`. |
| `name_max_length` | integer ≥ 0 | `64` | Maximum skill-name length for this harness. |
| `description_max_length` | integer ≥ 0 | `1024` | Maximum description length for this harness. |
| `supports_allowed_tools` | boolean | `false` | `allowed-tools`. Capability names: `allowed-tools`, `allowed_tools`. |
| `supports_disable_model_invocation` | boolean | `false` | `disable-model-invocation`. Capability names: `disable-model-invocation`, `disable_model_invocation`. |
| `supports_user_invocable_flag` | boolean | `false` | `user-invocable`. Capability names: `user-invocable`, `user_invocable`. |

A `required-capabilities` entry that matches none of those names is unsatisfied. The pair is skipped and the rest of the project still resolves. An unknown harness id in `skillprism.yaml` aborts resolution.

Length checks use Unicode scalar values. A name or description over the spec caps (64 and 1024) and within the harness cap is a warning. Over the harness cap is an error.

Built-in values:

| Id | Name | Subagent | Sidecar | Manifest | Allowed tools | Disable model invocation | User-invocable | Name max | Description max |
|----|------|----------|---------|----------|---------------|--------------------------|----------------|----------|-----------------|
| `claude` | Claude Code | true | false | true | true | true | true | 64 | 1536 |
| `codex` | Codex CLI | true | true | true | false | false | false | 100 | 500 |
| `opencode` | OpenCode | true | false | false | false | false | false | 64 | 1024 |
| `factory` | Factory | true | false | false | false | true | true | 64 | 1024 |
| `pi` | Pi | false | true | false | false | true | false | 64 | 1024 |

## `paths`

| | |
|---|---|
| Type | object |
| Required | yes |

| Field | Type | Required | Meaning |
|-------|------|----------|---------|
| `project_scope_path` | string | yes | Project skill directory, relative to the project root. |
| `user_scope_path` | string | yes | User skill directory. Joined to `$HOME`, except `opencode`, which is joined to `$XDG_CONFIG_HOME` or `$HOME/.config`. |
| `skill_filename` | string | yes | Rendered skill filename. Built-ins use `SKILL.md`. |
| `manifest_scope_path` | string or null | no | Manifest directory. Omitted when the harness has no manifest. |
| `manifest_filename` | string or null | no | Manifest filename. |

`build` writes the skill file to `dist/<id>/<skill-name>/<skill_filename>`. It does not insert `project_scope_path` into that path. A manifest is `dist/<id>/<manifest_scope_path>/<manifest_filename>`. Live installs use `project_scope_path` and `user_scope_path`. A scope path, sidecar filename, or sidecar `output_dir` that is absolute or contains `..` is rejected.

Built-in skill filenames are `SKILL.md`. Manifest paths:

| Id | `manifest_scope_path` | `manifest_filename` |
|----|------------------------|---------------------|
| `claude` | `.claude` | `plugin.json` |
| `codex` | `.agents` | `marketplace.json` |
| `opencode` | — | — |
| `factory` | — | — |
| `pi` | — | — |

## `macros`

| | |
|---|---|
| Type | object |
| Required | no |
| Default | `{}` |

Each key is available as `harness.<key>`. The value is a string, or an object whose required field is `content` (a string). Other keys on that object are ignored.

A skill `overrides.<id>.macros` entry replaces the same name for that skill only. See [Templates](templates/).

Every built-in defines `subagent_guide`. `claude` also defines `hints`.

## `sidecars`

| | |
|---|---|
| Type | array |
| Required | no |
| Default | `[]` |

Each item is rendered next to the skill file.

| Field | Type | Required | Meaning |
|-------|------|----------|---------|
| `filename` | string | yes | Output filename. |
| `template` | string | yes | Template for that sidecar file. |
| `output_dir` | string or null | no | Subdirectory of the skill output directory. |

The built-in definitions ship no sidecar templates. `codex` and `pi` set `requires_sidecar` to true, so a skill that requires `sidecar` resolves for those two ids.

## `manifest`

| | |
|---|---|
| Type | object |
| Required | no |

Top-level field. It is not an item of `sidecars`. It has one field, `template` (string, required when `manifest` is set). `build` fills that template once per skill, checks that each result is JSON, and writes one JSON array of those objects. Invalid JSON is an error and no output is written.

`claude` and `codex` set `requires_manifest` to true and define a `manifest.template`. Their templates also emit `skill_ref`.

### Manifests

The template below has no `format` key.

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

## `skill_ref_pattern`

| | |
|---|---|
| Type | string or null |
| Required | no |

Pattern passed to `skill_ref`. `{name}` is replaced with the argument. Every built-in sets `/{name}`. When the field is omitted, `skill_ref` uses `/{name}`. The template sees the raw pattern as `harness.skill_ref_pattern` when the field is set.

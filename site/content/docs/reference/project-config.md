---
title: "Project configuration"
description: "Fields of skillprism.yaml."
group: "Reference"
weight: 41
---

`skillprism.yaml` is the project file. Commands look in the current directory and then in parent directories until they find one. Editors can use the schema at <https://tuvren.github.io/skillprism/schema/v1/skillprism.json>.

The file has no `name` field. Unknown fields are rejected. An empty file is valid: `harnesses` is an empty list and `skills_dir` is `skills`.

`init project` writes this file. The command is [init project](cli/#init-project).

## `harnesses`

| | |
|---|---|
| Type | array of strings |
| Required | no |
| Default | `[]` |

Harness ids this project builds and, when non-empty, the default install targets for `add`. Built-in ids are `claude`, `codex`, `opencode`, `factory`, and `pi`. See [Harness](harness/).

An id that is not a built-in and not a file under `harnesses/` makes `build` and `validate` fail. The list may be empty. `harnesses: claude` is invalid because the value must be a list.

## `skills_dir`

| | |
|---|---|
| Type | string |
| Required | no |
| Default | `skills` |

Directory of skills, relative to the project root. `init skill` rejects an absolute path and a path that contains `..`.

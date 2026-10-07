---
title: "How to add a custom harness"
description: "Add a harness definition the five built-in harnesses do not cover."
group: "How-to guides"
weight: 24
---

This guide shows you how to add a harness the five built-ins do not cover. Run the commands from a directory that has `skillprism.yaml`, or from a directory under it. The definition fields are in the [harness reference](../reference/harness/).

Pick an id that is not `claude`, `codex`, `opencode`, `factory`, or `pi`. A file that reuses a built-in id replaces that built-in. Do not use `claude-code` or `droid` unless you mean to replace that alias: an exact id wins over the alias.

```bash
skillprism init harness my-agent
```

This writes `harnesses/my-agent.yaml`. `build`, `add`, and `update` load every `.yaml` and `.yml` file in that directory.

Edit `project_scope_path` and `user_scope_path` so they match the directories that agent reads. Leave `skill_filename` as `SKILL.md` unless the agent expects another name.

Add the id to `skillprism.yaml`, or a plain `build` will not include it:

```yaml
harnesses:
  - claude
  - my-agent
```

Compile it:

```bash
skillprism build -H my-agent
```

The skill file is `dist/my-agent/<skill>/SKILL.md`.

To give that harness a JSON index, add the path fields to the existing `paths` map:

```yaml
manifest_scope_path: .my-agent
manifest_filename: index.json
```

Add this map. Do not add a `format` key:

```yaml
manifest:
  template: |
    {
      "name": {{ skill_name | tojson }},
      "description": {{ skill_description | tojson }}
    }
```

Apply `tojson` to each interpolated string and leave the filter unquoted. `build` renders one entry per skill, checks that the aggregated file is JSON, and writes `dist/my-agent/.my-agent/index.json`. A broken template fails the build before any output is written.

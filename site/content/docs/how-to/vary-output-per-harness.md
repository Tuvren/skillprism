---
title: "How to vary output per harness"
description: "Render different text from one skill for each harness."
group: "How-to guides"
weight: 23
---

This guide shows you how to make one skill render different text for different harnesses. Field names are in [skill.yaml](../reference/skill-yaml/). Template syntax is in [templates](../reference/templates/).

Create `skills/status-note/skill.yaml`:

```yaml
skillprism: '1'
name: status-note
description: Report status in the wording each harness expects.
variables:
  status: ready
overrides:
  opencode:
    variables:
      status: ready for OpenCode
```

`overrides.<harness>.variables` replaces the top-level value for that harness only. Other harnesses keep `ready`.

Create `skills/status-note/SKILL.md`:

```jinja
---
name: {{ skill_name | yaml_str }}
description: {{ skill_description | yaml_str }}
---

# {{ skill_name }}

Status: {{ status }}.

{% if harness.id == "claude" %}
Run this from the Claude Code menu.
{% elif harness.id == "opencode" %}
Ask OpenCode to run this skill by name.
{% else %}
Run this skill the way this harness usually runs a skill.
{% endif %}
```

On every line between the two `---` fences, pass string interpolations through `yaml_str`. Do not wrap that filter in quotes. Leave the conditional body outside the frontmatter.

The harness ids in the conditional must be ids your project actually builds. Compile two of them and compare the files:

```bash
skillprism build -H claude,opencode
```

`dist/claude/status-note/SKILL.md` contains `Status: ready.` and the Claude Code sentence. `dist/opencode/status-note/SKILL.md` contains `Status: ready for OpenCode.` and the OpenCode sentence. A harness omitted from the conditional gets the `else` text and the top-level `status` value.

To change a harness macro for this skill only, add `overrides.<harness>.macros` in the same `skill.yaml`. That override does not change the macro for other skills.

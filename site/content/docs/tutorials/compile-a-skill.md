---
title: "Create a skill and compile it"
description: "Create one project, author one skill, and compile it into dist/."
group: "Tutorials"
weight: 10
aliases:
  - /docs/quickstart/
---

In this tutorial, we will create one project, author one skill, validate it, and compile it so a rendered SKILL.md appears under dist/.

This tutorial uses skillprism 0.4.0 on your PATH. If that command is missing, follow [How to install skillprism](../how-to/install-skillprism/).

First, create a project named `my-skills` for Claude. We pass `-H claude` so the command writes that harness and does not prompt.

```bash
skillprism init project my-skills -H claude
```

The output should look something like this:

```text
Created project `my-skills` in `my-skills`
```

Move into the project:

```bash
cd my-skills
```

The command prints nothing. The project files are:

```text
my-skills/
├── skillprism.yaml
├── .gitignore
├── README.md
└── skills/
    └── sample/
        ├── skill.yaml
        ├── SKILL.md
        ├── assets/
        │   └── example.txt
        ├── references/
        │   └── example.md
        └── scripts/
            └── example.sh
```

`.gitignore` ignores `dist/`. Open `skillprism.yaml`. It is the whole project configuration:

```yaml
# yaml-language-server: $schema=https://tuvren.github.io/skillprism/schema/v1/skillprism.json
harnesses:
  - claude
skills_dir: skills
```

`harnesses` lists Claude, and `skills_dir` is the directory of skill source. See [project configuration](../reference/project-config/).

The project has one skill, `skills/sample`. The directory name is `sample`, so the `name` field has to be `sample`. See [skill.yaml](../reference/skill-yaml/).

Init wrote a placeholder description. Replace `skills/sample/skill.yaml` with this file:

```yaml
# yaml-language-server: $schema=https://tuvren.github.io/skillprism/schema/v1/skill.json
skillprism: '1'
name: sample
description: Summarize the file the user names. Use when asked to summarize a file, a diff, or a block of text.
```

`SKILL.md` is the template for that skill. Replace `skills/sample/SKILL.md` with:

```jinja
---
name: {{ skill_name | yaml_str }}
description: {{ skill_description | yaml_str }}
---

# {{ skill_name }}

{{ skill_description }}

Read the file the user names, then write a short summary of what it contains.
```

The YAML frontmatter is filled from `skill_name` and `skill_description` in [skill.yaml](../reference/skill-yaml/). The `yaml_str` filter keeps those strings valid in the compiled file. See [templates](../reference/templates/).

Check the skill before writing any output:

```bash
skillprism validate
```

The output should look something like this:

```text
  ok: sample → claude
Validation passed (1 skill(s))
```

`validate` writes no files. Compile the skill:

```bash
skillprism build
```

The command prints nothing. It writes the compiled tree under `dist/`:

```text
dist/
└── claude/
    ├── .claude/
    │   └── plugin.json
    └── sample/
        ├── SKILL.md
        ├── assets/
        │   └── example.txt
        ├── references/
        │   └── example.md
        └── scripts/
            └── example.sh
```

Open `dist/claude/sample/SKILL.md`. It should look like this:

```markdown
---
name: "sample"
description: "Summarize the file the user names. Use when asked to summarize a file, a diff, or a block of text."
---

# sample

Summarize the file the user names. Use when asked to summarize a file, a diff, or a block of text.

Read the file the user names, then write a short summary of what it contains.
```

Notice that `name` and `description` are quoted, and that the body is the text from the template. The placeholder files under `assets/`, `references/`, and `scripts/` were copied beside that `SKILL.md`.

Notice that build also wrote `dist/claude/.claude/plugin.json`:

```json
[
  {
    "name": "sample",
    "description": "Summarize the file the user names. Use when asked to summarize a file, a diff, or a block of text.",
    "skill_ref": "/sample"
  }
]
```

That file is inside `dist/`. It is not an agent directory.

We created the `my-skills` project, authored the `sample` skill, and compiled it to `dist/claude/sample/SKILL.md`. [Why source, dist, and installed are different](../explanation/source-dist-and-installed/).

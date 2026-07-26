---
title: "Quickstart"
description: "Create your first skill, build it, and install it end-to-end"
aliases:
  - /docs/
group: "Get started"
weight: 20
---
This walkthrough creates a project, authors a skill, compiles it to `dist/`, and installs it into live agent directories. It takes about 2 minutes.

## 1. Initialize a project

```bash
skillprism init project my-skills
cd my-skills
```

`skillprism init project` interactively prompts for target harnesses (default: `claude`, `opencode`) and creates:

```
my-skills/
├── skillprism.yaml      # Project config (configured harnesses)
├── .gitignore           # Ignores dist/ output dir
├── README.md            # Project README
└── skills/
    └── sample/          # A sample skill to get started
        ├── skill.yaml   # Skill metadata
        └── SKILL.md     # Template (MiniJinja)
```

The generated `skillprism.yaml` configures the target harnesses:

```yaml
name: my-skills
harnesses:
  - claude
  - opencode
skills_dir: skills
```

## 2. Add a real skill

```bash
skillprism init skill dice-roller
```

This scaffolds `skills/dice-roller/` with a spec-compliant template. Edit the two files:

**`skills/dice-roller/skill.yaml`:**

```yaml
skillprism: '1'
name: dice-roller
description: >-
  Roll dice using a random number generator. Use when asked to roll a die (d6, d20, etc.), roll dice, or generate a random dice roll.
```

**`skills/dice-roller/SKILL.md`:**

```jinja
---
name: {{ skill_name }}
description: {{ skill_description }}
---

# {{ skill_name }}

{{ skill_description }}

To roll a die, use the following command that generates a random number from 1 to the given number of sides:

```bash
echo $((RANDOM % <sides> + 1))
```

Replace `<sides>` with the number of sides on the die (e.g. 6 for a standard die, 20 for a d20).
```

The YAML frontmatter at the top of `SKILL.md` is what the Agent Skills spec requires for discovery — skillprism renders `{{ skill_name }}` and `{{ skill_description }}` from `skill.yaml` into it, once per harness.

## 3. Validate

```bash
skillprism validate
```

This checks your templates for syntax errors, undefined variables, missing macros, and spec compliance — without writing any files. You should see both skills listed as `ok`.

## 4. Build

```bash
skillprism build
```

`skillprism build` compiles each skill into `dist/<harness>/`:

```
dist/claude/skills/dice-roller/SKILL.md
dist/claude/skills/sample/SKILL.md
dist/opencode/skills/dice-roller/SKILL.md
dist/opencode/skills/sample/SKILL.md
```

Each rendered `SKILL.md` has the frontmatter filled in with the real values from `skill.yaml`. You can inspect `dist/` to verify compiler output.

## 5. Install live skills

When you want to activate skills in live agent directories (e.g. `.claude/skills/`), use `skillprism add`:

```bash
skillprism add ./ --target project
```

This installs the compiled skills into your live harness paths (`.claude/skills/`, `.opencode/skills/`) and registers them in `.skillprism/state.json`.

## 6. Preview build diffs

```bash
skillprism build --diff
```

Shows a unified diff of what *would* be built, without modifying any files. Useful before committing to see exactly what changed.

## Next steps

- [Concepts](../concepts) — Source vs dist vs installed states
- [CLI Reference](../cli) — Complete flags and commands
- [skill.yaml reference](../skill-yaml) — Every metadata field and what it does
- [Templating](../templating) — Variables, harness macros, per-harness overrides

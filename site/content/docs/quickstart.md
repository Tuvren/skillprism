---
title: "Quickstart"
description: "Create your first skill and build it end-to-end"
group: "Get started"
weight: 20
---

# Quickstart

This walkthrough creates a project, adds a skill, and builds it to multiple harnesses. It takes about 2 minutes.

## 1. Initialize a project

```bash
skillprism init project my-skills
cd my-skills
```

This creates:

```
my-skills/
├── skillprism.yaml      # Project config (harnesses list)
├── .gitignore           # Ignores harness output dirs
├── README.md            # Project README
├── harnesses/           # Custom harness overrides (empty)
└── skills/
    └── sample/          # A sample skill to get started
        ├── skill.yaml   # Skill metadata
        └── SKILL.md     # Template (MiniJinja)
```

The default `skillprism.yaml` targets `claude` and `opencode`:

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

skillprism renders each skill once per configured harness and writes to each harness's expected path:

```
.claude/skills/dice-roller/SKILL.md
.claude/skills/sample/SKILL.md
.opencode/skills/dice-roller/SKILL.md
.opencode/skills/sample/SKILL.md
```

Each rendered `SKILL.md` has the frontmatter filled in with the real values from `skill.yaml`. The skills are now discoverable by Claude Code and OpenCode.

## 5. Preview changes

```bash
skillprism build --diff
```

Shows a unified diff of what *would* be written, without modifying any files. Useful before committing to see exactly what changed.

## 6. Add more harnesses

Edit `skillprism.yaml` to target more harnesses:

```yaml
name: my-skills
harnesses:
  - claude
  - opencode
  - codex
  - factory
  - pi
skills_dir: skills
```

Re-run `skillprism build` — your skills are now compiled to all five harnesses from the same source.

## Next steps

- [skill.yaml reference](../skill-yaml) — Every metadata field and what it does
- [Templating](../templating) — Variables, harness macros, per-harness overrides
- [Spec compliance](../spec-compliance) — How skillprism maps to the Agent Skills spec

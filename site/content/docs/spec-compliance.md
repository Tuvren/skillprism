---
title: "Spec compliance"
description: "How skillprism maps to the Agent Skills spec"
---

# Spec compliance

skillprism renders output that complies with the [Agent Skills specification](https://agentskills.io/specification). This page explains the mapping.

## Frontmatter

The spec requires `SKILL.md` to start with YAML frontmatter containing at minimum `name` and `description`:

```markdown
---
name: my-skill
description: Does X when the user asks for Y.
---

# My Skill

Instructions here.
```

skillprism's scaffold emits this frontmatter by default, and `skillprism validate` enforces it. Your `SKILL.md` template should include:

```jinja
---
name: {{ skill_name }}
description: {{ skill_description }}
---
```

skillprism renders `{{ skill_name }}` and `{{ skill_description }}` from `skill.yaml` into the frontmatter, once per harness.

### Optional frontmatter fields

The spec defines optional frontmatter fields (`license`, `compatibility`, `metadata`, `allowed-tools`). Your template controls which appear in the output by referencing them:

```jinja
---
name: {{ skill_name }}
description: {{ skill_description }}
license: {{ license }}
allowed-tools: {{ allowed_tools }}
---
```

Unset fields render as empty — `license: ` — not as `none`.

## Name constraints

The spec requires:

- 1-64 characters
- Lowercase letters, digits, and hyphens only (`^[a-z0-9]+(-[a-z0-9]+)*$`)
- No leading, trailing, or consecutive hyphens
- Must match the parent directory name

`skillprism validate` enforces all of these. A skill named `My_Skill` or `--agent` will fail validation with a clear error message.

### Harness-specific name length

Some harnesses allow longer names (e.g. Codex allows 100 characters). skillprism:
- **Errors** if the name exceeds the target harness's `name_max_length`
- **Warns** if the name is over 64 (spec cap) but within the harness's cap

## Description constraints

The spec requires:

- 1-1024 characters
- Non-empty
- Describes what the skill does **and** when to use it

`skillprism validate` enforces non-empty and length caps.

### Harness-specific description length

Claude Code allows descriptions up to 1536 characters (vs. the spec's 1024). skillprism:
- **Errors** if the description exceeds the target harness's `description_max_length`
- **Warns** if the description is over 1024 (spec cap) but within the harness's cap

This means a 1200-character description builds successfully for Claude (with a portability warning) but would error for Codex (cap: 500) or OpenCode (cap: 1024).

### Character count, not byte count

Length checks use Unicode character count, not byte count. A 1024-character description with multi-byte characters (e.g. emojis, accented letters) passes the 1024-char cap even though it's more than 1024 bytes.

## Compatibility constraint

The spec caps the `compatibility` field at 500 characters. `skillprism validate` enforces this universally (no harness-specific override exists).

## Progressive disclosure

The spec recommends:

- `SKILL.md` under 500 lines and <5000 tokens
- Detailed reference material in separate files (`references/`, `scripts/`, `assets/`)
- File references one level deep from `SKILL.md`

skillprism supports this by copying every subdirectory of a skill verbatim alongside the rendered `SKILL.md`. You structure your skill for progressive disclosure; skillprism preserves that structure in every harness's output.

## Asset directories

The spec defines optional directories: `scripts/`, `references/`, `assets/`. skillprism copies **every** direct subdirectory of a skill's directory verbatim, regardless of name — so `reference/` (singular), `examples/`, or any other name works. Dot-directories (`.venv/`, `.git/`) are excluded as tooling artifacts.

## What skillprism adds beyond the spec

skillprism is a **build tool**, not a replacement for the spec. It adds:

- **Multi-harness compilation** — one source, multiple outputs
- **Template rendering** — MiniJinja variables and harness macros
- **Per-harness overrides** — variables and macros that differ by harness
- **Capability gating** — `required-capabilities` to skip incompatible harnesses
- **Manifest generation** — aggregated JSON manifests for harnesses that need them
- **Validation** — spec compliance + template correctness checks

The output is always plain spec-compliant `SKILL.md` files — no skillprism-specific runtime required. Agents that support the Agent Skills spec can read skillprism output directly.

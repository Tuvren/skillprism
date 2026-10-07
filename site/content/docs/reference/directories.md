---
title: "Directories and state"
description: "Compile output, live skill directories, and the state file."
group: "Reference"
weight: 45
---

Live installs write under the harness paths below. `build` does not. Commands that read or write these paths are in [CLI](cli/). The path fields are in [Harness](harness/).

## Compile output

`build` writes each skill to `dist/<harness-id>/<skill-name>/`. The file name is the harness `skill_filename` (`SKILL.md` for every built-in). `project_scope_path` is not part of that path.

Every direct subdirectory of the skill is copied next to that file. The directory can have any name. Dot-directories are not copied. The Agent Skills specification recommends a `SKILL.md` under 500 lines and under 5000 tokens, with longer material in separate files. skillprism does not enforce that size.

A manifest is `dist/<harness-id>/<manifest_scope_path>/<manifest_filename>`. For the built-ins that define one, that is `dist/claude/.claude/plugin.json` and `dist/codex/.agents/marketplace.json`.

## Live directories

`codex`, `opencode`, `factory`, and `pi` share the project directory `.agents/skills/`. Two of those harnesses that render different bytes to the same file are a path collision.

OpenCode's user directory is `$XDG_CONFIG_HOME/opencode/skills/` when `XDG_CONFIG_HOME` is set and non-empty. When it is unset or empty, the directory is `~/.config/opencode/skills/`. Every other user path is relative to `$HOME`.

| Id | Project directory | User directory |
|----|-------------------|----------------|
| `claude` | `.claude/skills/` | `~/.claude/skills/` |
| `codex` | `.agents/skills/` | `~/.codex/skills/` |
| `opencode` | `.agents/skills/` | `$XDG_CONFIG_HOME/opencode/skills/` (default `~/.config/opencode/skills/` when `XDG_CONFIG_HOME` is unset) |
| `factory` | `.agents/skills/` | `~/.factory/skills/` |
| `pi` | `.agents/skills/` | `~/.agents/skills/` |

`remove` also deletes a recorded file under a legacy directory from an earlier install: `.opencode/skills/`, `.factory/skills/`, `.pi/skills/`, `~/.pi/agent/skills/`, and `~/.config/opencode/skills/` when that path was recorded for an OpenCode user install. An unrecorded file there stays. Moving those copies is covered in [Move skills installed before 0.4.0](../how-to/move-pre-0-4-installs/).

## Install record

`add`, `list`, `remove`, and `update` keep one record of installed skills at `~/.config/skillprism/installed.yaml`. When `XDG_CONFIG_HOME` is set, the file is `$XDG_CONFIG_HOME/skillprism/installed.yaml`. Project and user installs are both in that file. `list`, `remove`, and `update` show only the project you are in, unless you ask for user scope. Do not edit the file. Use the commands.

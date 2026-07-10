---
title: "skillprism vs Vercel Skills CLI"
description: "An honest comparison of two approaches to agent skill management"
group: "Guides"
weight: 110
---
[Vercel's Skills CLI](https://github.com/vercel-labs/skills) (`npx vercel-skills`) is another tool for working with agent skills. Both tools install skills and manage their lifecycle, but they take fundamentally different approaches.

## Philosophy

| | skillprism | Vercel Skills CLI |
|---|---|---|
| **Primary role** | Distribution CLI with per-harness templating | Skill installer with Git workflow helpers |
| **Format support** | Builds on the [Agent Skills specification](https://agentskills.io/specification) — one source, multi-harness compilation | Installs skills as-is; expects per-harness sources to already exist |
| **Rendering** | MiniJinja template engine per harness | No rendering; installs raw Markdown files |
| **Target audience** | Skill authors who maintain one source across multiple agent platforms | Developers who want to quickly install community skills |

## Feature comparison

| Feature | skillprism | Vercel Skills CLI |
|---|---|---|
| **Install from GitHub** | `add owner/repo` | `install owner/repo` |
| **Git ref pinning** | `owner/repo#v1.0.0` | `owner/repo#v1.0.0` |
| **Local installs** | Yes | No |
| **Multi-harness rendering** | Built-in (MiniJinja per harness) | No |
| **Format auto-detection** | skillprism-format vs plain-format per skill | Single format |
| **State tracking** | `~/.config/skillprism/installed.yaml` | No persistent state |
| **List installed** | `list` | No |
| **Remove installed** | `remove` | No |
| **Update installed** | `update` with `git ls-remote` up-to-date checks | No |
| **Diff preview** | `--diff` / `--dry-run` on build and update | No |
| **Per-file hash comparison** | SHA-256, write-only-on-change | N/A |
| **Git transport** | Direct `git` shell-out | `simple-git` library + `gh` CLI + SSH fallback |
| **Local compilation** | `build` for project-local skills | Not applicable |
| **Validation** | `validate` — spec compliance, syntax, variables, macros | No |
| **Scaffolding** | `init project`, `init skill`, `init harness` | `init` (basic template) |

## Git transport differences

Both tools need to fetch Git repositories. The difference is in how:

- **Vercel Skills CLI** uses `simple-git` (Node.js library) as primary transport, with `gh` CLI and SSH as fallbacks. This provides flexibility in authenticated environments but adds dependency weight.
- **skillprism** shells out directly to `git`. This means `git` must be on `PATH` and authenticated for private repos, but keeps the binary small and avoids intermediate library abstractions.

## When to use which

**Choose skillprism when:**
- You maintain a multi-harness skill and want one source of truth
- You need to install, list, remove, and update skills as part of a workflow
- You want spec-compliance validation and deterministic builds
- You prefer a single compiled binary over an npm/npx invocation

**Choose Vercel Skills CLI when:**
- You want to quickly install a skill from GitHub with no setup
- You work exclusively with plain Markdown skills (no templating)
- You prefer an npm-based workflow
- You need `gh` and SSH integration for authenticated private repos

Both tools can coexist in the same project — use Vercel Skills CLI for quick installs and skillprism for multi-harness compilation and lifecycle management.

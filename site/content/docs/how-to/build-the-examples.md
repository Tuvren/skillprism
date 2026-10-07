---
title: "How to build the examples"
description: "Build the example skills shipped in the repository and check the output."
group: "How-to guides"
weight: 26
aliases:
  - /docs/examples/
---

This guide shows you how to build the examples shipped in the repository. They are a `skillprism build` target, not a skill library. Do not copy them verbatim into a project.

From a clone of the repository:

```bash
cd examples
skillprism build
find dist -type f | sort
```

Do not pass `--target`. `build` does not accept it. The command writes only `dist/`, which the repository gitignore ignores, and does not write live `.claude/` or `.agents/` directories into `examples/`.

The project harnesses are `claude`, `opencode`, and `codex`. Expect a `[resolve] skipped` warning on stderr for `mcp-builder` with `opencode` and for `mcp-builder` with `codex`. The build still succeeds. `mcp-builder` requires `subagent` and `allowed-tools`, and only `claude` supports `allowed-tools` among those three harnesses.

A finished build has seven `SKILL.md` files:

- `dist/claude/mcp-builder/SKILL.md`
- `dist/claude/webapp-testing/SKILL.md`, `dist/opencode/webapp-testing/SKILL.md`, and `dist/codex/webapp-testing/SKILL.md`
- `dist/claude/quickstart/SKILL.md`, `dist/opencode/quickstart/SKILL.md`, and `dist/codex/quickstart/SKILL.md`

`mcp-builder` keeps its upstream `reference/` directory, including `reference/mcp_best_practices.md`. `webapp-testing` keeps its upstream `examples/` directory.

Two manifests are written:

- `dist/claude/.claude/plugin.json` references `mcp-builder`, `webapp-testing`, and `quickstart`.
- `dist/codex/.agents/marketplace.json` references `webapp-testing` and `quickstart`, not the skipped `mcp-builder`.

OpenCode has no manifest.

Where each example skill comes from is recorded in `examples/README.md` in the repository.

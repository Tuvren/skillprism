---
title: "Examples"
description: "Real skills compiled across harnesses"
---

# Examples

The [`examples/`](https://github.com/tuvren/skillprism/tree/master/examples) directory in the repo contains three skills that exercise skillprism's mechanics:

## quickstart

A small, deliberately synthetic skill meant to be read end-to-end in one sitting. It doesn't do anything useful — it exists purely to exercise every skillprism mechanism at least once in one short file: built-ins (`{{ skill_name }}`, `{{ harness.id }}`), a custom `variables:` entry, a builtin harness macro (`{{ harness.subagent_guide }}`), and `skill.yaml`'s per-harness `harnesses:` block overriding both a variable (for `opencode` only) and a macro (for `codex` only).

**Start here** if you're new to skillprism.

## mcp-builder

A guide for building MCP (Model Context Protocol) servers, ported from Anthropic's public `mcp-builder` Agent Skill. It genuinely needs `required-capabilities: [subagent, allowed-tools]` — the four-phase build is long enough to warrant running forked, and it only needs read/write/bash/fetch access. Since only `claude` supports `allowed-tools` among the example project's three harnesses, this skill resolves for `claude` only — `opencode`/`codex` are skipped with a warning, not a build failure.

## webapp-testing

A Playwright-based toolkit for testing local web apps, ported from Anthropic's public `webapp-testing` Agent Skill. No `required-capabilities`, so it resolves for all three harnesses. Its asset folder (`examples/`) is kept exactly as-is from upstream — demonstrating that skillprism copies any subdirectory name, not just `references/` or `scripts/`.

## Building the examples

```bash
cd examples
skillprism build --target dist
find dist -type f | sort
```

Use `--target dist` here, not plain `skillprism build` — the default `--target project` writes live `.claude/`, `.opencode/`, and `.agents/` directories into `examples/`, which would leave generated output as untracked files in your working tree.

Expect a `[resolve] skipped: ...` warning on stderr for each of the two `mcp-builder`/`opencode` and `mcp-builder`/`codex` pairs, then a successful build with 7 rendered `SKILL.md` files and two aggregated manifests.

## Attribution

`quickstart` is original content written for this repository. `mcp-builder` and `webapp-testing` are adapted from [Anthropic's public Agent Skills repository](https://github.com/anthropics/skills), pinned at commit `35414756ca55738e050562e272a6bbc6273aa926`. Both source skills are licensed under the Apache License, Version 2.0.

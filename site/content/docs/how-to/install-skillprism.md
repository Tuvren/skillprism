---
title: "How to install skillprism"
description: "Install the skillprism command with npm or bun."
group: "How-to guides"
weight: 20
aliases:
  - /docs/install/
---

This guide shows you how to install the `skillprism` command. Use npm or bun. `add` and `update` need `git` on `PATH` as well, because they clone a skill repository.

## With npm

```bash
npm install -g skillprism
```

The package downloads the skillprism binary for Linux x86_64, macOS x86_64, or macOS ARM. If `skillprism` is not found, add the directory from `npm prefix -g` plus `/bin` to `PATH`.

## With bun

```bash
bun add -g skillprism
```

The same platforms apply. If `skillprism` is not found, add bun's global bin directory to `PATH`.

Check the result with:

```bash
skillprism --version
```

The command prints `skillprism` and the installed version.

The command list is in the [CLI reference](../reference/cli/). To compile a first skill, follow [Create a skill and compile it](../tutorials/compile-a-skill/).

---
title: "How to compile selected harnesses"
description: "Compile the current project into dist/ for the harnesses you choose."
group: "How-to guides"
weight: 21
---

Compile the current project into `dist/` for the harnesses you choose. `skillprism build` only writes that directory. If you have never compiled a skill, follow [Create a skill and compile it](../tutorials/compile-a-skill/) first.

Run the command from the project directory, or from any directory under it.

To compile every harness named in that file:

```bash
skillprism build
```

To compile only some of them, pass `-H` with a comma-separated list, or repeat the flag:

```bash
skillprism build -H claude,opencode
```

Each id must be a built-in harness or a harness file under `harnesses/`. The flag replaces the list in `skillprism.yaml` for that run.

To see a unified diff and write nothing:

```bash
skillprism build -H claude --diff
```

If a file under `dist/` already exists, `build` asks before replacing it. Flags are in the [CLI reference](../reference/cli/). Where files land is in [Directories and state](../reference/directories/). To copy a skill into a live directory, use [How to install, update, and remove a skill](install-update-remove/).

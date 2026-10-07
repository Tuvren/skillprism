---
title: "How to install, update, and remove a skill"
description: "Install a skill into a live agent directory, then list, update, or remove it."
group: "How-to guides"
weight: 22
aliases:
  - /docs/distribution/
---

Install a skill into a live agent directory, see what is installed, update it, or remove it. `add` and `update` need `git` on `PATH`. These commands do not write `dist/`. Directories and the install record are in [Directories and state](../reference/directories/). Sources and flags are in the [CLI reference](../reference/cli/).

## Install it

To install into the current project:

```bash
skillprism add owner/repo --target project
```

To install into your user directories:

```bash
skillprism add owner/repo -g
```

If you omit the scope, `add` asks. Outside a skillprism project the only choice is user scope.

Open the agent after the files are in place. It reads the directory this command wrote.

## See what is installed

```bash
skillprism list
```

## Update it

```bash
skillprism update
skillprism update my-skill
```

With no scope flag, this updates user installs and the project installs for the current project.

## Remove it

```bash
skillprism remove my-skill --target project
```

With no scope flag, `remove` affects the project scope only. Pass `-g` for the user install.

```bash
skillprism remove --all --target project
```

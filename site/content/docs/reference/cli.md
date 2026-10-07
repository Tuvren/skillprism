---
title: "CLI"
description: "Commands, flags, and exit codes of the skillprism binary."
group: "Reference"
weight: 40
aliases:
  - /docs/cli/
---

`skillprism` is one binary. `-v` / `--verbose` is global. Clap rejects an unknown flag or an invalid value and exits 2.

Live directories are the project and user skill paths in [Directories and state](directories/). `dist/<harness-id>/<skill-name>/` is compile output, not a live directory.

| Exit | When |
|------|------|
| 0 | The command finishes without an error. |
| 1 | A runtime or validation error, including a non-interactive overwrite (`NonInteractiveOverwrite`). |
| 2 | A usage error: Clap's parse error, `init` usage errors, and `add` / `remove` usage errors. |
| 130 | `build` receives SIGINT. |
| 143 | `build` receives SIGTERM (Unix). |

A non-interactive overwrite exits 1. `build` returns that error when stdin is not a terminal, the destination exists, and `--force` is absent. `add` returns it when `--yes` is set without `--force` and the file existed before the command. `update` treats `-y` as `--force`, so `-y` overwrites; with neither flag, a non-terminal stdin exits 1 when a destination exists. `remove` exits 2 when it must confirm and stdin is not a terminal, unless `-y`, `--force`, or `--all` is set.

`build`, `init skill`, and `init harness` walk upward from the current directory for `skillprism.yaml`. `validate` walks upward from its path argument.

## build

`build` renders each skill for each selected harness and writes `dist/<harness-id>/<skill-name>/`. It does not write live directories. `--target` is rejected.

With no `-H`, the harness list is `harnesses` from [skillprism.yaml](project-config/). An unknown id in that file aborts the command. A skill whose `required-capabilities` the harness does not provide is skipped; other pairs still render. See [Harness](harness/).

`--diff` prints a unified diff and writes nothing. `--force` overwrites an existing destination without a prompt. Otherwise an existing file prompts on stderr: `y` / `yes`, `n` / `no`, `o` / `overwrite` / `all` (overwrite the rest), `s` / `skip` / `skip-all` (skip the rest), `a` / `abort`.

An empty skill list prints `No skills to build.` and exits 0. Path collisions abort before writing. Invalid manifest JSON aborts before writing. Selecting harnesses is covered in [Compile for selected harnesses](../how-to/compile-selected-harnesses/).

| Flag | Description |
|------|-------------|
| `-H`, `--harness` | Harness ids, comma-separated or repeated. Visible alias: `--harnesses`. `claude-code` and `droid` follow the alias rule in [Harness](harness/). |
| `--diff` | Print the diff and write nothing. Visible alias: `--dry-run`. |
| `--force` | Overwrite existing files without a prompt. |
| `-v`, `--verbose` | Print the project root, phase timings, and resolved variables. |

## validate

`validate` loads the project, resolves pairs, and checks templates. It writes no files.

Each valid pair is printed as `ok: <skill> → <harness>`. Warnings go to stderr. Any error exits 1. A missing path exits 1.

| Argument | Description |
|----------|-------------|
| `path` | Start directory or file. Default: `.`. The project root is the nearest ancestor directory that contains `skillprism.yaml`. |

## init

`init` writes source files. It does not write live directories or `dist/`.

### init project

`init project <name>` creates `<name>/` (or `--out`) with `skillprism.yaml`, `skills/sample/`, `.gitignore`, and `README.md`. `<name>` is the directory argument and the README title. It is not a field in `skillprism.yaml`.

The harness prompt appears only when stdin and stdout are both terminals and `-H` is omitted. Otherwise the harness list is `claude`, `opencode`, and stderr says `Using default harnesses: claude, opencode.` An empty list exits 2. An existing `skillprism.yaml` in the target directory exits 2 and leaves that directory unchanged.

| Flag | Description |
|------|-------------|
| `-o`, `--out` | Output directory. Default: `./<name>`. |
| `-H`, `--harnesses` | Comma-separated harness ids. |

### init skill

`init skill <name>` writes `<skills_dir>/<name>/` in the current project: `skill.yaml` (`skillprism: '1'`), `SKILL.md`, and `references/`, `scripts/`, and `assets/`. `<name>` must not contain `/`, `\`, or `..`. No `skillprism.yaml` on the walk upward exits 2. `skills_dir` must be relative and must not contain `..`.

### init harness

`init harness <name>` writes `harnesses/<name>.yaml`. The same name and project-root rules as `init skill` apply. Filling that file in is covered in [Add a custom harness](../how-to/add-custom-harness/).

## completions

`completions` writes a completion script to stdout and writes no files.

| Argument | Description |
|----------|-------------|
| `shell` | `bash`, `fish`, or `zsh`. |

## add

`add` installs skills into live directories and records them in the state file from [Directories and state](directories/). Remote sources use `git`. A local path does not. Installing, updating, and removing are covered in [Install, update, and remove](../how-to/install-update-remove/).

A directory with `skill.yaml` and `skillprism` equal to `1` or `'1'` is skillprism format and is rendered. A directory with `SKILL.md` and no `skill.yaml` is plain format and is copied. A `skill.yaml` without a supported `skillprism` value is an error.

With `--all`, harness selection is the `-H` / `-a` list when either is set, otherwise the project's `harnesses` list when that list is non-empty, otherwise every built-in harness.

With no scope flag, `add` prompts for `project` or `user` when a terminal is available. `--yes`, `--force`, or `--all` selects `project` when `skillprism.yaml` is found and `user` otherwise. Project scope with no `skillprism.yaml` exits 2.

| Argument | Description |
|----------|-------------|
| `source` | Install source. Empty or whitespace-only input exits 2. |
| `--target` | `project` or `user`. `dist` is rejected (exit 2). Conflicts with `-g`. |
| `-g`, `--global` | User scope. |
| `-s`, `--skill` | One skill name from a multi-skill source. Ignored when `--all` is set. |
| `-a`, `--agent` | Harness id, repeatable, comma-separated. Accepts `claude-code` and `droid`. |
| `-H`, `--harnesses` | Comma-separated harness ids. Combined with `-a`. |
| `-y`, `--yes` | Skip prompts. Does not overwrite pre-existing files. |
| `--list` | Print skill names and write nothing. |
| `--all` | Install every discovered skill and skip prompts. |
| `--force` | Overwrite pre-existing files and skip prompts. |

Source forms:

| Form | Example |
|------|---------|
| GitHub shorthand | `owner/repo` |
| GitHub prefix | `github:owner/repo` |
| GitLab prefix | `gitlab:owner/repo` |
| HTTPS or SSH Git URL | `https://github.com/owner/repo.git`, `git@github.com:owner/repo.git` |
| Ref | `owner/repo#v1.0.0` |
| Subpath | `owner/repo/skills/my-skill` |
| Skill filter | `owner/repo@my-skill` |
| Local path | `./path` or an absolute path |
| Alias | `coinbase/agentWallet` resolves to `coinbase/agentic-wallet-skills` |

A local source is also `.`, `../dir`, `~`, or `~/dir`. A skill directory contains `SKILL.md` or `SKILL.md.j2`.

`#ref` and `@skill` combine, as in `owner/repo#v1.0.0@my-skill`. The same suffixes work on `github:` and `gitlab:` forms, including GitLab subgroups. A tree URL is a remote, for example `https://github.com/owner/repo/tree/v1.0.0/skills/my-skill`. A host that ends in `.gitlab.com`, or starts with `gitlab.`, is GitLab. A `#ref` fragment on an HTTPS or SSH Git URL pins the checkout.

A subpath must not contain `..`. An `http://` or `https://` URL that is not a GitHub, GitLab, or `.git` remote is rejected. A `.well-known/agent-skills/index.json` URL parses and then fails: that source is not installed.

## list

`list` reads the state file and writes a tab-separated table to stdout. It does not modify skill directories. Opening the state file creates it when it is absent. The alias is `ls`.

Columns: name, source, ref, format (`skillprism` or `plain`), scope (`project` or `user`), harness ids. A commit SHA longer than 7 characters is printed as its first 7 characters. A missing ref is `-`. Project records are limited to the current project root. No matching rows prints `No installed skills` on stderr and exits 0.

| Flag | Description |
|------|-------------|
| `--target` | `project` or `user`. Conflicts with `-g`. |
| `-g`, `--global` | User scope only. |
| `-a`, `--agent` | Harness id, repeatable, comma-separated. Same alias rule as `add`. |
| `-H`, `--harnesses` | Comma-separated harness ids. Combined with `-a`. |

## remove

`remove` deletes installed files from live directories and updates the state file. The alias is `rm`. It also deletes a recorded file under a legacy directory listed in [Directories and state](directories/). An unrecorded file in those directories is left in place.

The default scope is `project`. `--all-scopes` selects both scopes. Names come from the positional arguments and from repeatable `-s` (comma-separated); duplicates are dropped. A missing selection exits 1. `--all` with named skills exits 2. `--all-scopes` without `--all` or named skills exits 2.

Confirmation is a `y` / `yes` prompt. `-y`, `--force`, and `--all` skip it. Without those flags, a non-terminal stdin exits 2. Declining the prompt exits 1.

| Argument | Description |
|----------|-------------|
| `skills` | Skill names. Optional. |
| `-s`, `--skill` | Skill name, repeatable, comma-separated. |
| `--target` | `project` or `user`. Conflicts with `-g`. |
| `-g`, `--global` | User scope. |
| `-a`, `--agent` | Harness id, repeatable, comma-separated. |
| `-H`, `--harnesses` | Comma-separated harness ids. Combined with `-a`. |
| `--all` | Every installed skill in the selected scope. Skips confirmation. |
| `--all-scopes` | Project and user scopes. |
| `-y`, `--yes` | Skip confirmation. |
| `--force` | Skip confirmation. |

## update

`update` re-reads each selected install from its source and writes changed files in live directories. The alias is `up`. It has no `--agent` flag. With no scope flag, user installs and the current project's installs are selected. It writes the current harness directories and leaves a previous copy in another directory on disk. `--diff` / `--dry-run` prints a diff and writes nothing, including the state file.

A local source, an install with no git ref, and an install pinned to a commit SHA are skipped. An unknown skill name is skipped. Other errors exit 1. `-y` and `--force` both skip the overwrite prompt and replace an existing file. `update` checks whether the remote changed, then clones the skill again when it has.

| Argument | Description |
|----------|-------------|
| `skills` | Skill names. Omitted means every installed skill that matches the scope and harness filters. |
| `--target` | `project` or `user`. Conflicts with `-g` and `-p`. |
| `-g`, `--global` | User scope. |
| `-p`, `--project` | Project scope. |
| `-H`, `--harnesses` | Comma-separated harness ids. Alias rule applies. |
| `--diff` | Print the diff and write nothing. Visible alias: `--dry-run`. |
| `-y`, `--yes` | Same effect as `--force` on this command. |
| `--force` | Overwrite existing files and skip the prompt. |

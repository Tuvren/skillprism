# skillprism

[![standard-readme compliant](https://img.shields.io/badge/standard--readme-compliant-green.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![npm version](https://img.shields.io/npm/v/skillprism.svg?style=flat-square)](https://www.npmjs.com/package/skillprism)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=flat-square)](LICENSE)

> Distribution CLI with per-harness templating for AI agent skills.

`skillprism` compiles skill templates (`SKILL.md`) for multiple AI agent runtimes (like Claude Code, Codex, Opencode) based on a unified configuration, validates them against schema requirements, and manages remote skill installations.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
  - [CLI Commands](#cli-commands)
  - [Environment Variables](#environment-variables)
- [Maintainers](#maintainers)
- [Contributing](#contributing)
- [License](#license)

## Background

AI agents (like Claude Code, Codex, and others) use specialized "skills" to perform complex workspace actions. However, different agent environments require slightly different markdown formats, trigger keywords, and system guidelines.

`skillprism` bridges this gap. It implements the [Agent Skills specification](https://agentskills.io/specification) to let you write skills once using a templating engine (MiniJinja) and compile them dynamically for all target environments. It also manages the discovery, lifecycle, and updates of third-party skills cloned from remote Git repositories.

This npm package acts as a thin launcher wrapper. When run, it automatically detects your operating system and architecture, downloads the verified native binary from the official release, caches it locally, and forwards all arguments directly.

## Install

No pre-requisites are required other than Node.js (version 18 or higher) and system `tar` (for unpacking).

### Global Installation

To install globally on your system:

```sh
npm install -g skillprism
```

### Local/Temporary Run (npx)

You can run it directly without global installation:

```sh
npx skillprism <command>
```

## Usage

Initialize a project, scaffold a skill, and build your compiled outputs.

### CLI Commands

#### 1. Initialize a new project
```sh
skillprism init project my-skills
cd my-skills
```

#### 2. Scaffold a new skill
```sh
skillprism init skill my-new-agent
```
This generates:
- `skills/my-new-agent/skill.yaml` (metadata & custom variables)
- `skills/my-new-agent/SKILL.md` (MiniJinja template)

#### 3. Compile and Build
```sh
skillprism build
```
This renders your skill templates and outputs the harness-compliant files into their respective folders (e.g. `.claude/skills/`, `.opencode/skills/`).

To preview changes without writing files:
```sh
skillprism build --diff
```

#### 4. Validate Skills
Validate the templates, variables, and spec conformance without writing files:
```sh
skillprism validate
```

#### 5. Install Remote Skills
Add skills directly from Git repositories or GitHub/GitLab shorthands:
```sh
skillprism add owner/repo
```

#### 6. List and Remove Skills
```sh
skillprism list
skillprism remove <skill-name>
```

### Environment Variables

- `SKILLPRISM_VERSION`: Pin a specific version of the native binary (e.g. `0.1.1`). Defaults to the latest release.
- `SKILLPRISM_SKIP_CHECKSUM`: Set to `1` to bypass tarball checksum validation (intended only for local testing).

## Maintainers

- Oscar Yáñez Cisterna ([@SkrOYC](https://github.com/SkrOYC))

## Contributing

PRs accepted. Please refer to the workspace `AGENTS.md` file for environment setup and commands.

Small note: If editing the Rust code, make sure to format with `cargo fmt` and run checks:
```sh
cargo clippy -- -D warnings
cargo test
```

## License

[Apache License 2.0](LICENSE) © Oscar Yáñez Cisterna

# skillprism

[![standard-readme compliant](https://img.shields.io/badge/standard--readme-compliant-green.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
[![npm version](https://img.shields.io/npm/v/skillprism.svg?style=flat-square)](https://www.npmjs.com/package/skillprism)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=flat-square)](LICENSE)

> Distribution CLI with per-harness templating for AI agent skills.

## Table of Contents

- [Background](#background)
- [Install](#install)
- [Usage](#usage)
- [Maintainers](#maintainers)
- [Contributing](#contributing)
- [License](#license)

## Background

This npm package is a thin launcher. When you run it, it detects your operating system and architecture, downloads the native binary from the official GitHub release, checks the checksum, caches the binary locally, and forwards every argument to that binary.

What the commands do is documented at [tuvren.github.io/skillprism/docs](https://tuvren.github.io/skillprism/docs/). The command reference is the [CLI reference](https://tuvren.github.io/skillprism/docs/reference/cli/).

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
npx skillprism --help
```

## Usage

```sh
skillprism --help
```

`npx skillprism` works the same way without a global install. Replace `--help` with any skillprism command.

To compile a skill for the first time, follow [Create a skill and compile it](https://tuvren.github.io/skillprism/docs/tutorials/compile-a-skill/). Flags and subcommands are in the [CLI reference](https://tuvren.github.io/skillprism/docs/reference/cli/).

### Environment Variables

- `SKILLPRISM_VERSION`: Pin a specific version of the native binary, for example `0.5.0`. When unset, the launcher downloads the newest GitHub release.
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

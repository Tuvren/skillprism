# skillprism

[![CI](https://github.com/tuvren/skillprism/actions/workflows/ci.yml/badge.svg)](https://github.com/tuvren/skillprism/actions/workflows/ci.yml)

skillprism is a distribution CLI and per-harness compiler for agent skills. Write a skill once, compile it for the harnesses configured in the project (`dist/`), and install, list, update, or remove skills in live agent directories. Rendered skills follow the [Agent Skills specification](https://agentskills.io/specification).

**Documentation:** [tuvren.github.io/skillprism/docs](https://tuvren.github.io/skillprism/docs/)

To compile a skill for the first time, follow [Create a skill and compile it](https://tuvren.github.io/skillprism/docs/tutorials/compile-a-skill/). For a flag, field, path, or template fact, use the reference below.

## Install

```bash
npm install -g skillprism
```

```bash
bun add -g skillprism
```

`add` and `update` clone a skill repository, so `git` has to be on `PATH`.

The package installs a binary for Linux x86_64, macOS x86_64, or macOS ARM. Check with `skillprism --version`.

## Documentation

### Tutorials

- [Create a skill and compile it](https://tuvren.github.io/skillprism/docs/tutorials/compile-a-skill/)

### How-to guides

- [Install skillprism](https://tuvren.github.io/skillprism/docs/how-to/install-skillprism/)
- [Compile selected harnesses](https://tuvren.github.io/skillprism/docs/how-to/compile-selected-harnesses/)
- [Install, update, and remove skills](https://tuvren.github.io/skillprism/docs/how-to/install-update-remove/)
- [Vary output per harness](https://tuvren.github.io/skillprism/docs/how-to/vary-output-per-harness/)
- [Add a custom harness](https://tuvren.github.io/skillprism/docs/how-to/add-custom-harness/)
- [Move pre-0.4 installs](https://tuvren.github.io/skillprism/docs/how-to/move-pre-0-4-installs/)
- [Build the examples](https://tuvren.github.io/skillprism/docs/how-to/build-the-examples/)

### Reference

- [CLI](https://tuvren.github.io/skillprism/docs/reference/cli/)
- [Project configuration](https://tuvren.github.io/skillprism/docs/reference/project-config/)
- [skill.yaml](https://tuvren.github.io/skillprism/docs/reference/skill-yaml/)
- [Harness definition](https://tuvren.github.io/skillprism/docs/reference/harness/)
- [Templates](https://tuvren.github.io/skillprism/docs/reference/templates/)
- [Directories](https://tuvren.github.io/skillprism/docs/reference/directories/)

Project and user install paths for each harness are in [Directories](https://tuvren.github.io/skillprism/docs/reference/directories/).

### Explanation

- [Source, dist, and installed copies](https://tuvren.github.io/skillprism/docs/explanation/source-dist-and-installed/)
- [Capabilities](https://tuvren.github.io/skillprism/docs/explanation/capabilities/)
- [Agent Skills specification](https://tuvren.github.io/skillprism/docs/explanation/agent-skills-spec/)
- [skills CLI](https://tuvren.github.io/skillprism/docs/explanation/skills-cli/)

## Develop skillprism

The repository pins Rust 1.85 (edition 2024) in `rust-toolchain.toml`.

```bash
cargo install --path .
devenv shell
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

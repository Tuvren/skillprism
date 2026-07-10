---
title: "Install"
description: "Get skillprism running on your machine"
group: "Get started"
weight: 10
---

# Install

## From source

```bash
git clone https://github.com/tuvren/skillprism.git
cd skillprism
cargo install --path .
```

This installs the `skillprism` binary to your Cargo bin directory (usually `~/.cargo/bin/`).

## With devenv

If you use [devenv](https://devenv.sh/) for reproducible environments:

```bash
git clone https://github.com/tuvren/skillprism.git
cd skillprism
devenv shell
```

This drops you into a shell with `skillprism`, `cargo`, and all dependencies ready.

## Prerequisites

- Rust 1.85+ (edition 2024) — for building from source
- No runtime dependencies — skillprism is a single static binary

## Verify

```bash
skillprism --help
```

You should see the four subcommands: `build`, `validate`, `init`, and `completions`.

## Shell completions

skillprism can generate completions for Bash, Fish, and Zsh:

```bash
# Bash (add to ~/.bashrc)
skillprism completions bash >> ~/.bashrc

# Fish
skillprism completions fish > ~/.config/fish/completions/skillprism.fish

# Zsh (add to ~/.zshrc or a fpath dir)
skillprism completions zsh >> ~/.zshrc
```

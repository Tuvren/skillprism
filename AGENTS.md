AGENTS.md

This file provides guidance to AI Agents when working with code in this repository.
These instructions guide you to focus on project-specific architecture and commands rather than generic development advice, and to base the content on actual analysis of the codebase rather than assumptions.

## Commands

```bash
cargo build
cargo build --release --locked
cargo test                                        # all tests
cargo test <name_filter> -- --test-threads=1       # single test (add --nocapture for stdout)
cargo clippy -- -D warnings
cargo fmt --check
cargo doc --no-deps --document-private-items
cargo publish --dry-run
devenv shell                                      # enter dev environment
devenv test                                       # run pre-commit hooks via CI
pre-commit run --all-files                        # manually run hooks (fmt + clippy)
hugo server -s site                               # run the website locally (http://localhost:1313)
hugo --gc --minify -s site                        # build the website to site/public/
```

Clippy is deny-level (`#![deny(clippy::all, clippy::pedantic, clippy::nursery)]` in `main.rs`).
Rust toolchain is pinned to 1.85 (edition 2024) in `rust-toolchain.toml`.

## License Header

Every source file must start with the Apache 2.0 header:

```rust
// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
```

## Publishing

Two separate mechanisms, both triggered from the `master` branch:

1. **GitHub Release (binaries):** Tag a commit with a `v*` tag. The `release.yml` workflow builds Linux x86_64, macOS x86_64, and macOS ARM binaries, packages them with README/LICENSE/NOTICE/man page, and creates a GitHub Release.
   ```bash
   git tag -a v0.1.0 -m "v0.1.0"
   git push origin v0.1.0
   ```

2. **crates.io (library):** Requires `cargo login` first (token from https://crates.io/settings/tokens). Then:
   ```bash
   cargo publish
   ```

Both are manual steps; there is no automated cargo publish in CI.

## Pipeline Architecture

The `build` subcommand is the core pipeline. Subcommands (`validate`, `init`, `completions`) each run only a slice of it. The `__generate_man` sentinel in `main.rs` bypasses clap entirely.

```
CLI parse (cli.rs)
  │
  ├─ build ──────────────────────────────────────────────────────────
  │   load_project()         — reads skillprism.yaml, discovers skills
  │   HarnessResolver        — pairs each skill with each configured harness
  │   Validator::validate    — syntax, variables, macros checks
  │   Router::collisions     — detect path collisions before writing
  │   Engine::render         — MiniJinja rendering with context/harness macros
  │   Router::write          — atomic write with overwrite prompts, asset copies
  │   Router::manifests      — aggregate per-harness JSON manifests
  │
  ├─ validate ─ load + resolve + validate (no render/write)
  ├─ init ─ scaffold::project / scaffold::skill / scaffold::harness
  └─ completions ─ clap_complete::generate to stdout
```

### Module roles

| Module | Responsibility |
|--------|---------------|
| `cli.rs` | Clap derive definitions, dispatch, pipeline orchestration, completions, man page |
| `types/` | `ProjectModel`, `SkillModel`, `ProjectConfig`, `ProjectError`, re-exports `HarnessDefinition` |
| `registry/` | `HarnessRegistry` — 5 built-in harnesses (claude, codex, opencode, factory, pi) embedded via `include_str!`, user YAML overrides, `HarnessDefinition` with capabilities/paths/macros/sidecars/manifests |
| `loader/` | `ProjectLoader` — reads `skillprism.yaml`, recursively discovers skills from `skills/` directory |
| `resolver/` | `HarnessResolver` — creates `ResolvedPair` (skill + harness) per configured harness, checks capability requirements |
| `validator/` | `Validator` — syntax check (MiniJinja parse), undefined variables vs skill.yaml, undefined harness macros |
| `engine/` | `Engine` — builds rendering context, registers helper functions, renders skill templates and sidecars/manifests |
| `router/` | `Router` — collision detection, atomic file writes, unified diff, overwrite prompt (y/n/s/a), path traversal safety, manifest JSON aggregation |
| `scaffold/` | Generates new project/skill/harness file trees from built-in templates |

### Key data flow

`skillprism.yaml` → `ProjectConfig` + `SkillModel[]` (loader) → `ResolvedPair[]` (resolver) → validated pairs (validator) → `HarnessOutput` (engine) → files on disk (router).

### Error model

Functions return `Result<_, miette::Report>` or custom error enums (`ProjectError`, `ValidateError`, `RouterError`, `EngineError`) that implement `miette::Diagnostic` for rich error reports with help text.

# Vision

**Version:** v0.1.0 (see [changelog.md](./changelog.md))

## Executive Summary

skillprism is a build-time compiler that transforms a single canonical skill source (MiniJinja templates with harness-aware macros) into correct, harness-specific output files for every major agent platform. It eliminates manual cross-harness duplication so that one skill change propagates to Claude Code, Codex, OpenCode, Factory, Pi, and beyond — without editing N copies.

**Target archetype:** CLI tool (single static binary).

## Jobs to Be Done (JTBD)

| Job | When… | …the user wants to… |
| :--- | :--- | :--- |
| 1. Author once | A skill's shared logic changes | Update one template and have all harness outputs reflect the change — no manual propagation. |
| 2. Add a harness | A new agent platform emerges | Add a harness definition (one YAML file) without touching any skill template. |
| 3. Preview before deploy | Before overwriting installed skills | Diff the generated output against current state and confirm correctness. |
| 4. Bootstrap a project | Starting a new multi-harness skill library | Run a single command that scaffolds the project layout, config, and a sample skill. |
| 5. Catch errors early | During authoring or CI | Fail the build on missing macros, undefined variables, or malformed configuration — never ship a broken skill. |
| 6. Deploy to the right path | After building | Install generated skills to the correct agent-specific directory with one flag. |

## Appendix: Operator Preferences

The following technology choices are non-binding implementation hints for downstream stages. They were stated by the operator during the PRD phase and must not appear in requirements sections above.

| Preference | Choice | Rationale |
| :--- | :--- | :--- |
| Language | Rust | Single static binary, no runtime dependencies; ecosystem alignment with `skill-harness` project. |
| Template engine | MiniJinja (Jinja2-compatible) | Near-perfect Jinja2 implementation by Armin Ronacher; designed for text generation (YAML, config, Markdown). |
| Configuration format | YAML | Matches the skill ecosystem's YAML frontmatter convention; supports comments; more readable than JSON/TOML for nested structures. |
| CLI framework | Clap v5 (derive macros) | De facto Rust CLI framework; derive macros reduce boilerplate. |
| Serialization | Serde + serde_yaml | Standard Rust serialization framework; serde_yaml for YAML config loading. |
| Build system | Cargo | Standard Rust build tool. |
| Repository layout | Two repos | skillprism CLI in its own repo; users' skill projects in separate repos. |

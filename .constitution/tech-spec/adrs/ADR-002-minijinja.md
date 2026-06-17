# ADR-002: MiniJinja as the Template Engine

**Status:** Accepted

## Context

The PRD specifies MiniJinja for template rendering (named in vision.md). The architecture's Template Engine container depends on a template rendering engine. The template syntax must support macros, variable substitution, conditionals, and custom functions.

## Decision

Use `minijinja` 2.20.x with its auto-escaped, non-HTML mode. Enable the `builtins` and `custom_syntax` features. Register harness macros as named `Minijinja::add_function` or `add_filter` callbacks. Use `minijinja::parse()` for validation-only mode (no evaluation).

## Consequences

- **Positive:** Macro-per-harness definitions map directly to MiniJinja's function/filter registration. Parse-only mode (`minijinja::parse`) enables validation without side effects, matching the VA-1 flow. Exactly matches the PRD's technology preference.
- **Negative:** MiniJinja is a single-maintainer project (mitsuhiko). Bus factor risk.
- **Mitigation:** The template syntax is Jinja2-compatible. If MiniJinja becomes unmaintainable, switching to `tera` or another Jinja2-compatible engine requires changing only the `engine/` module.

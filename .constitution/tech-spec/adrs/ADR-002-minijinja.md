# ADR-002: MiniJinja as the Template Engine

**Status:** Accepted (Updated 2026-06-17 — Epic B implementation)

## Context

The PRD specifies MiniJinja for template rendering (named in vision.md). The architecture's Template Engine container depends on a template rendering engine. The template syntax must support macros, variable substitution, conditionals, and custom functions.

## Decision

Use `minijinja` 2.20+ with the `json` feature enabled (for `from_json` support in context building). Enable `serde` for type-safe context values.

Harness macros are registered as **context values** in the template environment (not as `add_function`/`add_filter` callbacks). The entire `harness` object (id, name, version, macros as string values) is pushed into the MiniJinja context, so templates access macros via `{{ harness.<macro_name> }}`. Custom helpers (e.g., `skill_ref`) are registered via `add_function`.

For validation-only mode, use `env.add_template()` with the `Template::parse()` method. This registers the template in a throwaway environment and catches parse errors without performing variable substitution or rendering.

## Consequences

- **Positive:** Harness macros as context values avoids the need for per-harness function registration. Parse-only validation is cheap and side-effect-free. The `json` feature enables flexible context construction.
- **Negative:** MiniJinja is a single-maintainer project (mitsuhiko). Bus factor risk.
- **Mitigation:** The template syntax is Jinja2-compatible. If MiniJinja becomes unmaintainable, switching to `tera` or another Jinja2-compatible engine requires changing only the `engine/` module.

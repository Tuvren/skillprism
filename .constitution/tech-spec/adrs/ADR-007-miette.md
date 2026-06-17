# ADR-007: Error Diagnostics via miette

**Status:** Accepted

## Context

The PRD constraint requires "a clear, actionable error message that identifies the file, line, and specific issue." The Validator must collect errors across all skills and present structured diagnostics. Rust's standard error reporting (`Display`/`Debug` on `dyn Error`) produces unstructured text that does not highlight source spans.

Four options were evaluated:

| Alternative | Approach | Span support | Notes |
| :--- | :--- | :--- | :--- |
| `miette` | Derive `Diagnostic` on error types | First-class — `#[source_code]` + `#[label]` attributes | Designed for CLI tools; renders rich terminal output with source snippets |
| `eyre` + `color-eyre` | Custom `EyreHandler` for `Report` | Via manual span capture | Requires building span-tracking infrastructure manually |
| `anyhow` | Ad-hoc context via `.context()` | None | Good for internal errors; poor for user-facing diagnostics |
| Manual `Display` | Custom format strings | Manual line/column formatting | High maintenance; inconsistent formatting |

## Decision

Use `miette` 7.6.x as the universal error type. All user-facing errors derive `miette::Diagnostic`. The `SkillError` type carries `#[source_code]` (the source file content) and `#[label]` (the offending span) attributes. Internal non-user-facing errors (e.g., I/O errors) are wrapped in `miette::Report` via `IntoDiagnostic`.

## Consequences

- **Positive:** Source-span highlighted terminal output satisfies the PRD's actionable-error constraint. Derive-based approach requires minimal boilerplate. Compatible with `thiserror` for structured error enums.
- **Negative:** `miette` adds ~100KB to the binary. The derive macro has edge cases with generic error types.
- **Mitigation:** Binary size increase is acceptable for a CLI tool. Edge cases avoided by keeping error types concrete (no generics on `SkillError`).

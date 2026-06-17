# Logical Risks & Technical Debt

**Version:** v0.1.0

| Risk | Likelihood | Impact | Mitigation |
| :--- | :--- | :--- | :--- |
| **Group-level variable inheritance ambiguity** — Deeply nested skill directories with multiple intermediate `skill.yaml` files can create surprising precedence behavior that is hard to debug. | Medium | Medium | Document the merge rule explicitly (single-level parent → child, not transitive merge). Validate by listing final resolved variables per skill in `--verbose` output. |
| **Harness definition schema drift** — As agent platforms evolve their skill format, built-in harness definitions become stale between releases. | High | High | Ship harness definitions compiled into the binary so they are version-locked to the release. Document the override mechanism so users can patch locally without waiting for a release. |
| **12 P0 capabilities for a v1** — Large surface area increases risk of integration issues between pipeline stages. | Medium | High | The pipe-and-filter decomposition allows each stage to be unit-tested in isolation. The shared Load → Resolve → Validate prefix between build and validate reduces surface area duplication. |
| **MiniJinja parsing edge cases** — Complex template syntax may produce parse errors that are difficult to attribute to the correct source location in the template. | Low | Medium | The Template Engine must preserve source line/column mapping through the render pipeline. The Validator should use MiniJinja's parse-only mode to check templates without evaluating them. |
| **User install path collisions** — Two different skills may generate files that conflict at the same target path (e.g., two skills both writing to `~/.claude/skills/`). | Low | High | The Output Router must detect path collisions within a single build and report them as errors. Cross-build collisions are handled by overwrite confirmation. |
| **Large skill projects** — A project with 200+ skills may exceed the collect-all-errors accumulator's reasonable bounds. | Low | Low | No explicit mitigation for v1. If this becomes a bottleneck, the error accumulator can be bounded (first N errors) or validation can be made concurrent. |

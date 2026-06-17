# Resilience & Cross-Cutting Concerns

**Version:** v0.1.0

## Error Handling

| Concern | Design |
| :--- | :--- |
| **Collect-all-errors** | The Validator stage processes every skill before reporting. Error accumulation is bounded by the total number of skills (expected 5-20, reasonable upper bound 200). Each error identifies file, line, and specific issue. |
| **Fail-fast on config** | If `skillprism.yaml` is unparseable or the project root does not exist, fail immediately — no pipeline can proceed without a valid project model. |
| **Non-zero exit code** | Any validation error or render failure produces exit code 1. A clean build produces exit code 0. |
| **No silent fallbacks** | Undefined variables, unresolved macros, and unparseable templates are always errors, never warnings. The PRD explicitly forbids placeholder output. |

## Write Safety

| Concern | Design |
| :--- | :--- |
| **Atomic writes** | Every file write follows: write to a temporary file in the same directory → rename to final path. On crash or interrupt, partial files never appear at the target path. |
| **Scope confinement** | The Output Router must never resolve a path outside the determined scope (project root, user home, or `dist/`). Path traversal attempts in harness installation paths are rejected. |
| **Overwrite confirmation** | Writing to a path where a file already exists requires user confirmation unless `--force` is set. The confirmation shows a diff summary. |
| **Signal handling** | On SIGINT/SIGTERM/^C during build, the Output Router abandons any in-progress atomic write (temp files are left for cleanup or reuse on retry). Already-renamed files remain intact. |

## Configuration

| Concern | Design |
| :--- | :--- |
| **Validation at load** | The Project Loader validates `skillprism.yaml` and all `skill.yaml` files against their schemas on read. Malformed files produce an error with file path and parse issue. |
| **Precedence chain** | Variables follow strict group-level inheritance: parent `skill.yaml` → child `skill.yaml` (child wins). Harness definitions follow: built-in → user override (`harnesses/{name}.yaml` with matching name) → custom new harness. |

## Telemetry & Observability

| Concern | Design |
| :--- | :--- |
| **No telemetry in v1** | The tool does not phone home, collect usage data, or send crash reports. |
| **Verbose output** | A `--verbose` flag on build/validate shows which skills are being processed, which harness targets are being generated, and elapsed time per phase. |
| **Diff output** | The `--diff` flag (or `--dry-run`) displays a diff between generated output and currently installed files without writing anything. This satisfies OB-1. |

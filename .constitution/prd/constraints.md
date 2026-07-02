# Constraints

## Platform

- The tool must run on Linux x86_64 and macOS (x86_64 + ARM).
- Windows is not a target platform for v1.

## Build Performance

- No explicit latency target for v1. Build times under 1 second for typical skill projects (5-20 skills) are expected but not enforced.

## Binary Distribution

- Single static binary with no runtime dependencies, *except for the `git` binary which is required for the `add` and `update` distribution commands.*
- The `git` binary is assumed to be present on the user's PATH. If it is missing, the OS-level `ENOENT` from `Command::status` surfaces as a normal runtime error (the same path as any missing external command); no dedicated startup gate is implemented.
- The `build`, `validate`, `init`, and `completions` commands remain purely static-binary and have no runtime dependencies.
- Binary is built from source via Cargo; no pre-built binary distribution for v1.

## Error Handling

- All configuration and template errors must produce a clear, actionable error message that identifies the file, line, and specific issue.
- No silent fallbacks or placeholder output.

## Safety

- Writing to any target scope must not overwrite existing files without user confirmation unless --force is explicitly provided.
- When writing to dist/ (--target dist) or any explicit output path, output must never modify files outside that path.

## Maintainability

- Built-in harness definitions must be versioned and released alongside the tool (same repo, same release cycle).
- Adding a new built-in harness must not require template changes — only a new harness definition YAML.

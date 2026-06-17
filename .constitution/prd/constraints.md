# Constraints

## Platform

- The tool must run on Linux x86_64 and macOS (x86_64 + ARM).
- Windows is not a target platform for v1.

## Build Performance

- No explicit latency target for v1. Build times under 1 second for typical skill projects (5-20 skills) are expected but not enforced.

## Binary Distribution

- Single static binary with no runtime dependencies.
- Binary is built from source via Cargo; no pre-built binary distribution for v1.

## Error Handling

- All configuration and template errors must produce a clear, actionable error message that identifies the file, line, and specific issue.
- No silent fallbacks or placeholder output.

## Safety

- The --target deploy flag must not overwrite existing files without user confirmation unless --force is explicitly provided.
- Build output to dist/ must never modify files outside of dist/.

## Maintainability

- Built-in harness definitions must be versioned and released alongside the tool (same repo, same release cycle).
- Adding a new built-in harness must not require template changes — only a new harness definition YAML.

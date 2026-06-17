# ADR-005: Atomic Writes via Temp-File-Rename

**Status:** Accepted (Updated 2026-06-17 — Epic B implementation)

## Context

The architecture's resilience section mandates that partial files must never appear at the target path on crash or interrupt. The Output Router must write files atomically.

## Decision

Every file write follows: write to a temporary file in the same directory (sibling of the final path) → `std::fs::rename` to the final path. Temp files use a `.tmp` extension appended to the path. On SIGINT/SIGTERM, in-progress temp files are left on disk (cleanup on next successful build or manual removal). Already-renamed files are never rolled back.

**Current implementation detail:** Temp files are created via `path.with_extension("tmp")`. This replaces the final extension (e.g., `SKILL.md` → `SKILL.tmp`). A future version should use `path.with_extension(format!("{}.tmp", path.extension().unwrap_or_default()))` to append rather than replace the extension, preventing collisions with extensionless files. A startup cleanup pass for orphaned `.tmp` files is deferred.

## Consequences

- **Positive:** Atomic on POSIX filesystems (`rename` is atomic within the same filesystem). No partial files visible at target paths. No need for a WAL or transaction log.
- **Negative:** Temp files accumulate on crash if not cleaned up. Non-atomic on cross-filesystem moves (mitigated by writing to same-directory temp file first).
- **Mitigation:** Document temp file naming convention (`{path}.tmp`) so users can identify and clean them. Startup cleanup pass is deferred to a future epic.

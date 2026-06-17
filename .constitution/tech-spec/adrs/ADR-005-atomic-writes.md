# ADR-005: Atomic Writes via Temp-File-Rename

**Status:** Accepted

## Context

The architecture's resilience section mandates that partial files must never appear at the target path on crash or interrupt. The Output Router must write files atomically.

## Decision

Every file write follows: write to a temporary file in the same directory (sibling of the final path) → `std::fs::rename` to the final path. Temp files use a `.tmp` suffix with a random component. On SIGINT/SIGTERM, in-progress temp files are left on disk (cleanup on next successful build or manual removal). Already-renamed files are never rolled back.

## Consequences

- **Positive:** Atomic on POSIX filesystems (`rename` is atomic within the same filesystem). No partial files visible at target paths. No need for a WAL or transaction log.
- **Negative:** Temp files accumulate on crash if not cleaned up. Non-atomic on cross-filesystem moves (mitigated by writing to same-directory temp file first).
- **Mitigation:** Document temp file naming convention (`{path}.{random}.tmp`) so users can clean them. The router runs a temp-cleanup pass at startup.

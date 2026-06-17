# Out of Scope: Pre-built Binary Distribution

**Context:** Determined out of scope during initial scope definition.

**Reasoning:** v1 distributes via Cargo build from source. Pre-built binaries (Homebrew taps, GitHub Releases with asset uploads, etc.) introduce CI/CD, signing, and platform-specific packaging complexity that is better addressed after the core tool is validated.

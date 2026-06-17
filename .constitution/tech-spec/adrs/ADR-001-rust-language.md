# ADR-001: Rust as the Implementation Language

**Status:** Accepted

## Context

The PRD requires a single static binary with no runtime dependencies compiled via Cargo. The Template Engine container must support MiniJinja, which is a native Rust library. The architecture defines 7 in-process library containers communicating within one binary — no IPC, no network.

## Decision

Build skillprism in Rust, using Edition 2024 (Rust 1.85+).

## Consequences

- **Positive:** Single static binary is trivial (`cargo build --release` produces a statically linked binary). MiniJinja's native Rust API eliminates FFI overhead. Clap (Rust) provides the best-in-class CLI derive macros. Serde ecosystem handles YAML schemas natively.
- **Negative:** Rust's compile times are longer than Go or Zig. The async ecosystem (Tokio) adds cognitive overhead if we later need concurrency.
- **Mitigation:** Sync-only pipeline (see ADR-004) avoids async entirely. Compile times are acceptable for a CLI tool with ~7 small modules.

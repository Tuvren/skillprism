# Flow: Ship Built-in Harness Definitions

**PRD Capability:** HS-1 — Ship built-in harness definitions for Claude Code, Codex, OpenCode, Factory, and Pi that cover skill format, installation paths, subagent API patterns, invocation syntax, frontmatter fields, sidecar requirements, and validation strictness.

**Primary actors:** Skill Author (Solo), Team Lead, Tool Integrator

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Registry as Harness Registry
  participant Binary as Compiled Binary

  Note over Binary,Registry: At compile time
  Binary->>Binary: include_str!("harnesses/claude.yaml")
  Binary->>Binary: include_str!("harnesses/codex.yaml")
  Binary->>Binary: include_str!("harnesses/opencode.yaml")
  Binary->>Binary: include_str!("harnesses/factory.yaml")
  Binary->>Binary: include_str!("harnesses/pi.yaml")

  Note over Registry: At runtime
  User->>CLI: skillprism build
  CLI->>Registry: getHarness("claude")
  Registry->>Registry: lookup compiled-in definitions
  Registry-->>CLI: resolved HarnessDefinition for claude

  CLI->>Registry: getHarness("opencode")
  Registry-->>CLI: resolved HarnessDefinition for opencode

  CLI->>CLI: continue build with all 5 harnesses
```

# Flow: Override Built-in Harness

**PRD Capability:** HS-2 — Allow users to override a built-in harness definition by placing a `harnesses/{name}.yaml` file in the project root.

**Primary actors:** Team Lead, Tool Integrator

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Loader as Project Loader
  participant Registry as Harness Registry
  participant FS as Filesystem

  User->>FS: write harnesses/claude.yaml (override)
  User->>CLI: skillprism build

  CLI->>Loader: load(projectRoot)
  Loader->>FS: scan harnesses/*.yaml
  FS-->>Loader: [claude.yaml, custom-foo.yaml]

  Loader->>Registry: load_user_overrides([claude.yaml, custom-foo.yaml])
  Registry->>Registry: for each, merge with builtin if exists, else add as-is
  Note over Registry: claude merges with builtin, custom-foo added as-is

  CLI->>Registry: resolve("claude")
  Registry-->>CLI: overridden definition
  CLI->>Registry: resolve("custom-foo")
  Registry-->>CLI: custom definition

  CLI->>CLI: build proceeds with overridden + custom harnesses
```

(End of file - total 38 lines)

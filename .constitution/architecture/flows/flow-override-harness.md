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

  Loader->>Registry: registerOverride("claude", claudeOverrideYaml)
  Registry->>Registry: merge(builtinClaude, userOverride)
  Note over Registry: user fields win, builtin fills gaps

  Loader->>Registry: registerCustom("custom-foo", customFooYaml)
  Registry->>Registry: add(customFooYaml)
  Note over Registry: no builtin to merge, added as-is

  CLI->>Registry: getHarness("claude")
  Registry-->>CLI: overridden definition
  CLI->>Registry: getHarness("custom-foo")
  Registry-->>CLI: custom definition

  CLI->>CLI: build proceeds with overridden + custom harnesses
```

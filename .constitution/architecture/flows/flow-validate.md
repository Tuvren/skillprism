# Flow: Validate Without Writing

**PRD Capability:** VA-1 — Provide a validate command that checks template syntax, variable definitions, and macro references without writing output.

**Primary actors:** Skill Author (Solo), Team Lead, CI pipeline

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Loader as Project Loader
  participant Registry as Harness Registry
  participant Valid8 as Validator
  participant FS as Filesystem

  User->>CLI: skillprism validate
  CLI->>Loader: load(projectRoot)
  Loader-->>CLI: project model

  CLI->>Valid8: validate(model)

  loop over every skill x harness combination
    Valid8->>Valid8: parse template syntax
    Valid8->>Registry: resolve all macro references
    Valid8->>Valid8: resolve all variable references
  end

  alt any errors found
    Valid8-->>CLI: ErrorList (all errors, all skills)
    CLI-->>User: "Validation failed — 3 errors across 2 skills"
    CLI->>CLI: exit code 1
  else all clean
    Valid8-->>CLI: validated model
    CLI-->>User: "Validation passed — 5 harnesses, 3 skills OK"
    CLI->>CLI: exit code 0
  end

  Note over Router: No filesystem writes occur
```

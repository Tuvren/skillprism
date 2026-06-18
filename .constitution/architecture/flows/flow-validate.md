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
  participant Resolver as Resolver
  participant Valid8 as Validator
  participant FS as Filesystem

  User->>CLI: skillprism validate
  CLI->>Loader: load(projectRoot)
  Loader-->>CLI: ProjectModel

  CLI->>Resolver: resolve(projectModel)
  Resolver->>Registry: build resolved pairs
  Registry-->>Resolver: harness definitions
  Resolver-->>CLI: Vec<ResolvedPair>

  CLI->>Valid8: validate(pairs)

  loop over every ResolvedPair
    Valid8->>Valid8: undeclared_variables() via MiniJinja
    Valid8->>Valid8: scan for harness.<name> refs
    Valid8->>Valid8: check template syntax
  end

  alt any errors found
    Valid8-->>CLI: ErrorList (all errors, all skills)
    CLI-->>User: "Validation failed — 3 errors across 2 skills"
    CLI->>CLI: exit code 1
  else all clean
    Valid8-->>CLI: validated
    CLI-->>User: "Validation passed — 5 harnesses, 3 skills OK"
    CLI->>CLI: exit code 0
  end

  Note over Valid8: No filesystem writes occur
```

(End of file - total 44 lines)

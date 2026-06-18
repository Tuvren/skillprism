# Flow: Fail on Missing Reference

**PRD Capability:** TC-6 — Fail the build on any missing macro reference or undefined template variable.

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

  User->>CLI: skillprism build
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

  alt errors found
    Valid8-->>CLI: ErrorList [err1: line 12 unknown macro, err2: line 45 undefined variable "bar"]
    CLI-->>User: Build failed — 2 errors
    CLI->>CLI: exit code 1
  else no errors
    Valid8-->>CLI: validated
    CLI->>CLI: continue to render
  end
```

(End of file - total 43 lines)

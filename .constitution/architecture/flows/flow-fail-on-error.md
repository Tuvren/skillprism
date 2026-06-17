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
  participant Valid8 as Validator
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>Loader: load(projectRoot)
  Loader-->>CLI: project model

  CLI->>Valid8: validate(model)

  loop over every skill
    Valid8->>Valid8: check template syntax
    Valid8->>Registry: check macro "{{ macros.foo }}" exists
    Registry-->>Valid8: undefined: foo
    Valid8->>Valid8: check variable "{{ bar }}" is defined
    Valid8->>Valid8: check variable "{{ baz }}" references are resolvable
  end

  alt errors found
    Valid8-->>CLI: ErrorList [err1: line 12 unknown macro "foo", err2: line 45 unknown var "bar"]
    CLI-->>User: Build failed — 2 errors
    CLI->>CLI: exit code 1
  else no errors
    Valid8-->>CLI: validated model
    CLI->>CLI: continue to render
  end
```

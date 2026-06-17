# Flow: Compile a Template

**PRD Capability:** TC-1 — Compile a template into a harness-specific SKILL.md file, resolving macro references and variable substitutions from the harness definition and skill configuration.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Loader as Project Loader
  participant Registry as Harness Registry
  participant Resolver as Resolver
  participant Valid8 as Validator
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>Loader: load(projectRoot)
  Loader->>FS: read skillprism.yaml, skill.yaml tree
  FS-->>Loader: project config + skill hierarchy
  Loader-->>CLI: ProjectModel

  CLI->>Resolver: resolve(projectModel)
  Resolver->>Registry: resolve(name) for each harness
  Registry-->>Resolver: HarnessDefinition[]
  Resolver-->>CLI: Vec<ResolvedPair>

  CLI->>Valid8: validate(pairs)
  Valid8->>Valid8: undeclared_variables() via MiniJinja
  Valid8->>Valid8: scan for harness.<name> refs
  Valid8-->>CLI: validated (or errors)

  CLI->>Engine: render(skill.template, variables, macros)
  Engine-->>CLI: rendered SKILL.md

  CLI->>Router: write(rendered, targetScope)
  Router->>Router: resolve path per scope
  Router->>FS: atomicWrite(.tmp → rename)
  FS-->>Router: confirmed
  Router-->>User: build complete
```

(End of file - total 43 lines)

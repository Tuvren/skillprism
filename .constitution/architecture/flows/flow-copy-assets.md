# Flow: Copy Shared Assets

**PRD Capability:** TC-2 — Copy shared assets (references/, scripts/) from the skill source to each harness output directory unchanged.

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
  Loader-->>CLI: ProjectModel with asset paths

  CLI->>Resolver: resolve(projectModel)
  Resolver->>Registry: build resolved pairs
  Registry-->>Resolver: harness definitions
  Resolver-->>CLI: Vec<ResolvedPair>

  CLI->>Valid8: validate(pairs)
  Valid8->>Valid8: undeclared_variables() via MiniJinja
  Valid8->>Valid8: scan for harness.<name> refs
  Valid8-->>CLI: validated

  CLI->>Engine: render(skill.template, ...)
  Engine-->>CLI: rendered SKILL.md

  CLI->>Router: writeRendered(rendered, targetScope)
  Router->>Router: resolve path per scope
  Router->>FS: atomicWrite(.tmp → rename) SKILL.md

  Router->>FS: copyTree(sourceRefs, targetRefs)
  FS-->>Router: assets copied

  Router->>FS: copyTree(sourceScripts, targetScripts)
  FS-->>Router: scripts copied

  Router-->>User: build complete
```

(End of file - total 45 lines)

# Flow: Copy Shared Assets

**PRD Capability:** TC-2 — Copy shared assets (references/, scripts/) from the skill source to each harness output directory unchanged.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Loader as Project Loader
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>Loader: load(projectRoot)
  Loader-->>CLI: project model with asset paths

  CLI->>Engine: render(skill.template, ...)
  Engine-->>CLI: rendered SKILL.md

  CLI->>Router: writeRendered(rendered, targetScope)
  Router->>FS: write SKILL.md

  Router->>FS: copyTree(sourceRefs, targetRefs)
  FS-->>Router: assets copied

  Router->>FS: copyTree(sourceScripts, targetScripts)
  FS-->>Router: scripts copied

  Router-->>User: build complete
```

# Flow: Generate Sidecar Files

**PRD Capability:** TC-4 — Generate harness-specific sidecar files from inline templates defined in the harness definition.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Registry as Harness Registry
  participant Resolver as Resolver
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>CLI: load, resolve, validate (omitted for brevity)

  CLI->>Engine: render(skill.template, variables, macros)
  Engine-->>CLI: rendered SKILL.md

  CLI->>CLI: harness.sidecars — inline sidecar templates from harness definition
  Note over CLI: sidecar templates are a field on the resolved HarnessDefinition

  CLI->>Engine: render(sidecarTemplate, skillContext, variables)
  Engine-->>CLI: rendered sidecar content

  CLI->>Router: writeSidecar(content, harness, skill)
  Router->>Router: harness.paths.sidecar(skillName)
  Note over Router: resolves to e.g., agents/my-skill.yaml
  Router->>FS: atomicWrite(.tmp → rename, content)
  FS-->>Router: sidecar written

  Router-->>User: build complete
```

(End of file - total 39 lines)

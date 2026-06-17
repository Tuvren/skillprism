# Flow: Generate Sidecar Files

**PRD Capability:** TC-4 — Generate harness-specific sidecar files from inline templates defined in the harness definition.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Registry as Harness Registry
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>CLI: load, validate (omitted for brevity)

  CLI->>Engine: render(skill.template, variables, macros)
  Engine-->>CLI: rendered SKILL.md

  CLI->>Registry: getSidecarTemplate(harnessName)
  Registry-->>CLI: sidecarTemplate (e.g., "agents/{skill}.yaml.j2")

  CLI->>Engine: render(sidecarTemplate, skillContext, variables)
  Engine-->>CLI: rendered sidecar content

  CLI->>Router: writeSidecar(content, harness, skill)
  Router->>Registry: getSidecarPath(harness, skillName)
  Registry-->>Router: e.g., agents/my-skill.yaml
  Router->>FS: atomicWrite(path, content)
  FS-->>Router: sidecar written

  Router-->>User: build complete
```

# Flow: Build to Project Scope

**PRD Capability:** BD-1 — Write all generated output to project-level harness paths by default, with each subdirectory mirroring the exact layout that harness expects.

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
  Note over CLI: default scope = project

  CLI->>CLI: load, validate, render (omitted for brevity)

  CLI->>Router: writeAll(renderedFiles, target=project)
  Router->>Registry: getInstallationPath("claude", scope=project)
  Registry-->>Router: ./.claude/skills/

  Router->>FS: atomicWrite(./.claude/skills/my-skill/SKILL.md)
  Router->>FS: atomicWrite(./.claude/skills/my-skill/references/guide.md)

  Router->>Registry: getInstallationPath("opencode", scope=project)
  Registry-->>Router: ./.agents/skills/

  Router->>FS: atomicWrite(./.agents/skills/my-skill/SKILL.md)

  Router->>FS: atomicWrite(./.claude/marketplace.json)
  Note over Router: plugin manifests written once per harness

  Router-->>User: build complete — 5 harnesses, 3 skills, 15 files
```

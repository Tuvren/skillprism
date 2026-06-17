# Flow: Generate Plugin Manifest

**PRD Capability:** TC-5 — Generate harness-specific plugin manifests that register skills with the agent's discovery system.

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
  CLI->>CLI: process all skills for harness H

  CLI->>CLI: harness.manifest — manifest template from harness definition
  Note over CLI: manifest is a field on the resolved HarnessDefinition

  CLI->>Engine: render(manifestTemplate, allSkillsForHarness, harnessDef)
  Note over Engine: manifest aggregates across all skills
  Engine-->>CLI: rendered manifest content

  CLI->>Router: writeManifest(content, harness)
  Router->>Router: harness.paths.manifest_path
  Note over Router: resolves to e.g., claude/marketplace.json
  Router->>FS: atomicWrite(.tmp → rename, content)
  FS-->>Router: manifest written

  Router-->>User: build complete
```

(End of file - total 37 lines)

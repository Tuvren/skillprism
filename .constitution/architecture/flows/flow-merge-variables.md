# Flow: Merge Group-Level Variables

**PRD Capability:** TC-3 — Merge group-level variables with per-skill variables (skill wins) before rendering each template.

**Primary actors:** Skill Author (Solo), Team Lead

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

  Loader->>FS: read skillprism.yaml
  FS-->>Loader: project config
  Loader->>Loader: walk skill directory tree

  Loader->>FS: read parent/skill.yaml
  FS-->>Loader: parent variables {theme: dark, lang: en}
  Loader->>FS: read child/skill.yaml
  FS-->>Loader: child variables {lang: fr, timeout: 30}

  Loader->>Loader: merge(parent, child) -> {theme: dark, lang: fr, timeout: 30}
  Note over Loader: child wins on collision (lang: fr)

  Loader-->>CLI: resolved model with merged variables
  CLI->>Valid8: validate(model)
  Valid8-->>CLI: validated

  CLI->>CLI: continue to render with resolved variables
```

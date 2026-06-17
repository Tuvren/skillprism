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
  participant Resolver as Resolver
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

  Loader-->>CLI: ProjectModel with merged variables
  CLI->>Resolver: resolve(projectModel)
  Resolver->>Registry: build resolved pairs
  Registry-->>Resolver: harness definitions
  Resolver-->>CLI: Vec<ResolvedPair>
  CLI->>Valid8: validate(pairs)
  Valid8->>Valid8: undeclared_variables() via MiniJinja
  Valid8->>Valid8: scan for harness.<name> refs
  Valid8-->>CLI: validated

  CLI->>CLI: continue to render with resolved variables
```

(End of file - total 42 lines)

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
  participant Valid8 as Validator
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build
  CLI->>Loader: load(projectRoot)
  Loader->>FS: read skillprism.yaml, skill.yaml tree
  FS-->>Loader: project config + skill hierarchy
  Loader->>Registry: resolveHarness(name, userOverrides)
  Registry-->>Loader: merged HarnessDefinition
  Loader-->>CLI: resolved ProjectModel

  CLI->>Valid8: validate(model)
  Valid8->>Loader: get all skills
  Valid8->>Registry: check macro references
  Valid8-->>CLI: validated model (or errors)

  CLI->>Engine: render(skill.template, variables, macros)
  Engine-->>CLI: rendered SKILL.md

  CLI->>Router: write(rendered, targetScope)
  Router->>Registry: getInstallationPaths(harness)
  Router->>FS: atomicWrite(targetPath + skillName/SKILL.md)
  FS-->>Router: confirmed
  Router-->>User: build complete
```

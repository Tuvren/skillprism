# Flow: Deploy to User Scope

**PRD Capability:** BD-2 — Accept a `--target` flag (project | user) that deploys generated output to the agent's installation path instead of the default project scope.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Registry as Harness Registry
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build --target user
  CLI->>CLI: parse --target user

  CLI->>CLI: load, validate, render (omitted for brevity)

  alt --force not set AND files exist at target
    CLI->>Router: checkExisting(targetPaths)
    Router->>FS: stat every target file
    FS-->>Router: 8 files already exist
    Router-->>User: "8 files will be overwritten. Continue? [y/N]"
    User->>CLI: y
  end

  CLI->>Router: writeAll(renderedFiles, target=user)

  Router->>Registry: getInstallationPath("claude", scope=user)
  Registry-->>Router: ~/.claude/skills/

  Router->>FS: atomicWrite(~/.claude/skills/my-skill/SKILL.md)

  Router->>Registry: getInstallationPath("opencode", scope=user)
  Registry-->>Router: ~/.config/opencode/skills/

  Router->>FS: atomicWrite(~/.config/opencode/skills/my-skill/SKILL.md)

  Router-->>User: deploy complete — 5 harnesses globally installed
```

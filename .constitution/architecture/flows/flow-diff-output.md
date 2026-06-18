# Flow: Diff Build Output

**PRD Capability:** OB-1 — Generate a diff preview that compares rendered output against whatever currently exists at the target paths, without writing anything.

**Primary actors:** Skill Author (Solo), Team Lead

## Sequence

```mermaid
sequenceDiagram
  actor User
  participant CLI as CLI Entrypoint
  participant Engine as Template Engine
  participant Router as Output Router
  participant FS as Filesystem

  User->>CLI: skillprism build --diff
  CLI->>CLI: parse --diff flag

  CLI->>CLI: load, resolve, validate, render (omitted for brevity)

  CLI->>Router: diffOutput(renderedFiles, targetScope=project)

  loop over every (file, harness, skill)
    Router->>FS: read current file at install path
    alt file exists
      FS-->>Router: current content
      Router->>Router: compute diff (rendered vs current)
    else file does not exist
      Note over Router: new file — full content shown
    end
  end

  Router-->>User: "Diff for claude/my-skill/SKILL.md: +3/-1 lines"
  Router-->>User: "Diff for opencode/my-skill/agents.yaml: new file"
  Router-->>User: "8 files changed, 0 files unchanged"

  Note over Router: No filesystem writes occur
  Note over CLI: User inspects diff before running build without --diff
```

(End of file - total 40 lines)

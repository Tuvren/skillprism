# Epic E — Scaffolding Enhancements (SCAFF)

**Acronym:** SCAFF
**Total Effort:** 7 SP
**Dependencies:** None (isolated to scaffold module; no pipeline dependencies)
**Goal:** Bring the `init` commands in line with the PRD specification and add missing capabilities. Current scaffold was a minimal P1 implementation; this fills the gaps.

---

#### SCAFF-E001 init project --harnesses Flag

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Add a `--harnesses` flag to `init project` that accepts a comma-separated list of harness IDs instead of hardcoding `claude, opencode`. Default remains `claude, opencode` when flag is omitted. Update the generated `skillprism.yaml` to include only the specified harnesses.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given running init project --harnesses claude,codex,pi
  When the project is scaffolded
  Then skillprism.yaml contains harnesses: claude, codex, pi

  Given running init project without --harnesses
  When the project is scaffolded
  Then skillprism.yaml contains the default harnesses (claude, opencode)
  ```

#### SCAFF-E002 init project Creates Sample Skill

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Make `init project` scaffold a sample skill inside the `skills/` directory. The sample skill should include a `skill.yaml` with description and a `SKILL.md.j2` template that demonstrates variable substitution and harness macro usage. This fulfills the PRD's SC-1 requirement for "a sample skill template."
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given running init project
  When the project is scaffolded
  Then skills/ contains a sample skill directory with skill.yaml and SKILL.md.j2
  And the sample template demonstrates at least {{ skill_name }} and {{ harness.id }}
  ```

#### SCAFF-E003 init Skill Creates Asset Directories

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** Update `init skill` to create `references/` and `scripts/` subdirectories inside the skill directory. These are the standard asset directories that the engine's `copy_assets` expects. The engine silently skips missing asset dirs, but the scaffold should set them up correctly.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given running init skill
  When the skill is scaffolded
  Then the skill directory contains references/ and scripts/ subdirectories
  ```

#### SCAFF-E004 init harness Command

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Add `init harness` subcommand that scaffolds a new custom harness definition YAML file in the `harnesses/` directory. The generated file includes all required fields (id, name, capabilities, paths) with placeholder values. User edits the file to customize. This supports HS-3 (P1: user-defined harnesses) by making harness creation discoverable.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given running init harness my-custom
  When the harness is scaffolded
  Then harnesses/my-custom.yaml exists with id, name, capabilities, and paths fields

  Given the generated harness file
  When inspecting its content
  Then it contains placeholder values ready for editing
  ```

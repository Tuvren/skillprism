# Epic B — Pipeline

Acronym: **PIPE** | Story Points: **16**

---

#### PIPE-B001 Harness Resolution and Compatibility
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** FND-A004
- **Description:** For each skill, resolve which harnesses apply. Match skills to harnesses by examining the skill's harness references against available definitions. Implement the compatibility check (HS-2) — validate that a skill's required capabilities are satisfied by the target harness. Return resolved pairs of (skill, harness, target_paths) for downstream processing.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a skill that references harness "claude_code"
  And a harness registry containing "claude_code"
  When the resolver matches the skill to its harness
  Then it returns a resolved pair with the skill, harness, and target scope paths
  When a skill references a harness not in the registry
  Then it returns a diagnostic error citing the unknown harness
  When a skill requires a capability the harness does not declare
  Then it reports a compatibility error with the missing capability name
  ```

---

#### PIPE-B002 Validator with Collect-All-Errors
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** PIPE-B001
- **Description:** Implement the validation engine that processes resolved skill-harness pairs. Check template variable completeness, harness capability constraints, and any structural invariants. Fail on undefined template variables or missing macro references (TC-6). Implement the collect-all-errors pattern (implements VA-1): gather every validation error across all skills before reporting. Format diagnostic output with miette (rich source snippets, error codes, severity levels).
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given 3 skills where 2 have validation errors
  When the validator runs
  Then it returns diagnostics for both failing skills
  And the passing skill is included in the valid output set
  When a skill references an undefined template variable
  Then the diagnostic includes the variable name and template path
  When all skills pass validation
  Then it returns an empty error set
  ```

---

#### PIPE-B003 MiniJinja Template Engine
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** PIPE-B001
- **Description:** Integrate MiniJinja as the template rendering engine. Register templates from harness files, inject scoped variables from the skill model (TC-1). Support relational dereferencing of nested skill fields and template name collision detection with error reporting (TC-6). Generate sidecar files from inline harness templates (TC-4) and plugin manifests for agent discovery systems (TC-5). Expose a `render(skill, harness) -> HarnessOutput` function returning skill content, sidecar files, and manifest entries.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a harness with a template "config.tmpl" containing {{ skill_name }}
  And a skill with name "my-agent"
  When the engine renders the template
  Then the output contains "my-agent"
  When two harnesses register templates with the same name
  Then the engine reports a collision error with both harness names
  When a template dereferences a related field via dot notation
  Then the engine resolves the nested value correctly
  Given a harness with an inline sidecar template
  When rendering produces output
  Then a sidecar file is generated alongside the SKILL.md
  Given a harness that requires a plugin manifest
  When rendering produces output
  Then a manifest entry is created in the harness output
  ```

---

#### PIPE-B004 Output Router and Atomic Writer
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** PIPE-B003, PIPE-B002
- **Description:** Compute output file paths from resolved target scopes (project, user, dist). Implement the atomic write strategy (ADR-005): write to a temporary sibling file, then rename. Copy shared assets (TC-2) — replicate `references/` and `scripts/` directories from the skill source to each harness output directory unchanged. Handle `--target` flag routing, directory creation, and existing-file collision checks.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a resolved (skill, harness) pair with target scope "project"
  When the router computes output paths
  Then the path is under <project_root>/.claude/skills/<skill_name>.md
  When target scope is "user"
  Then the path is under ~/.config/skillprism/skills/<skill_name>.md
  When an atomic write succeeds
  Then the target file contains the rendered content
  And no .tmp files remain in the target directory
  When the target directory does not exist
  Then it is created before writing
  Given a skill with a references/ directory and a scripts/ directory
  When the output router finishes
  Then both directories are replicated in each target output directory
  And file contents are identical to the source (not modified)
  ```

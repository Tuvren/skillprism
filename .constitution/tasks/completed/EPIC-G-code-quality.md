# Epic G — Code Quality

Acronym: **CLEAN** | Story Points: **9**

**Dependencies:** Epic F (TEST) — CI should be green before cleanup to catch regressions from refactoring

---

#### CLEAN-G001 Remove Dead TemplateCollision Variant

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Remove the unused `EngineError::TemplateCollision` variant from `engine/mod.rs` and its associated `#[allow(dead_code)]` attribute. Verify no compilation errors after removal.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the engine module
  When compiling
  Then no dead_code warnings are suppressed for TemplateCollision
  And the code compiles cleanly
  ```

---

#### CLEAN-G002 Remove Dead MissingField Variant

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Remove the unused `ProjectError::MissingField` variant from `types/error.rs` and its associated `#[allow(dead_code)]` attribute.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the types/error module
  When compiling
  Then no dead_code warnings are suppressed for MissingField
  ```

---

#### CLEAN-G003 Remove Dead skill_output_dir Function

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Remove the unused `skill_output_dir()` function from `router/paths.rs` and its `#[allow(dead_code)]` attribute.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the router/paths module
  When compiling
  Then no dead_code warnings are suppressed for skill_output_dir
  ```

---

#### CLEAN-G004 Fix Module-Level Allow in project.rs

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Replace the module-level `#![allow(clippy::redundant_pub_crate, dead_code)]` in `types/project.rs` with targeted `#[allow(...)]` attributes on the specific items that genuinely need them. Remove the module-level attr entirely.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given types/project.rs
  When running cargo clippy
  Then no module-level #[allow(...)] attributes remain
  And clippy passes with -D warnings
  ```

---

#### CLEAN-G005 Fix Module-Level Allow in registry/types.rs

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Replace the module-level `#![allow(clippy::struct_excessive_bools, dead_code)]` in `registry/types.rs` with targeted allows. `struct_excessive_bools` may remain as module-level if justified with a `// reason:` comment. Remove the `dead_code` allow entirely.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given registry/types.rs
  When running cargo clippy
  Then only justified allow attributes remain
  And no dead_code suppression exists at module level
  ```

---

#### CLEAN-G006 Spike: Evaluate yaml_serde → serde_yml Migration

- **Type:** Spike
- **Effort:** 2
- **Dependencies:** None
- **Description:** Research `serde_yml` as a replacement for `yaml_serde` (itself a fork of the deprecated `serde_yaml`). Compare API compatibility, maintenance status (last commit, open issues, downloads), and migration complexity. The spike report must include: API compatibility matrix, migration effort estimate in hours, and any breaking changes. Write findings to `.constitution/spikes/SPK-CLEAN-G006.md`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the spike completes
  When reading .constitution/spikes/SPK-CLEAN-G006.md
  Then the report contains an API compatibility matrix
  And migration effort estimate
  And a clear recommendation (migrate now, defer, or don't migrate)
  ```

---

#### CLEAN-G007 Fix Module-Level Allow in loader/mod.rs

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Replace the module-level `#![allow(dead_code, unused_imports)]` in `loader/mod.rs` with targeted `#[allow(...)]` attributes on the specific items that genuinely need them. Remove the module-level attr entirely.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given loader/mod.rs
  When running cargo clippy
  Then no module-level #[allow(...)] attributes remain
  And clippy passes with -D warnings
  ```

---

#### CLEAN-G008 Fix Module-Level Allow in types/mod.rs

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Replace the module-level `#![allow(unused_imports)]` in `types/mod.rs` with targeted `#[allow(...)]` attributes on the specific items that genuinely need them. Remove the module-level attr entirely.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given types/mod.rs
  When running cargo clippy
  Then no module-level #[allow(...)] attributes remain
  And clippy passes with -D warnings
  ```

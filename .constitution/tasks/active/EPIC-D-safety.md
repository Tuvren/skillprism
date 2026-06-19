# Epic D — Safety & Robustness (SAFE)

**Acronym:** SAFE
**Total Effort:** 16 SP
**Dependencies:** None (foundation for all subsequent epics)
**Goal:** Implement 9 documented-but-unbuilt safety and robustness items from architecture/resilience.md and risks.md. These were specified in the constitution but never coded.

---

#### SAFE-D001 Path Traversal Protection

- **Type:** Security
- **Effort:** 2
- **Dependencies:** None
- **Description:** Add scope confinement validation to the Output Router to reject harness installation paths containing `..` traversal that escape the determined scope (project root, user home, or `dist/`). Canonicalize resolved paths via `std::fs::canonicalize` and verify the final path is a prefix of the allowed base directory. Both project-scope and user-scope paths must be validated.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a harness with project_scope_path containing "../" traversal
  When resolving the skill output path
  Then the router returns an error rejecting path traversal

  Given a harness with a valid project_scope_path
  When resolving the skill output path
  Then the path resolves normally within the project root
  ```

#### SAFE-D002 Interactive Overwrite Confirmation

- **Type:** Feature
- **Effort:** 3
- **Dependencies:** None
- **Description:** Implement interactive user prompt when writing to existing files without `--force`. Show a diff summary of what would change using the existing diff infrastructure. Support `yes/no/skip-all/abort` responses. Extend skip protection to project scope (currently only user scope has it). The existing `--force` flag bypasses prompts. Prompts read from stdin and print to stderr.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given existing files at target paths
  When running build without --force
  Then the user is prompted with a diff summary before each overwrite

  Given running build with --force
  When existing files are encountered
  Then files are overwritten without prompting
  ```

#### SAFE-D003 Signal Handling

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Register SIGINT/SIGTERM handlers using `ctrlc` crate or manual signal handling. On signal during build, abandon in-progress atomic writes and exit with code 130 (SIGINT) or 143 (SIGTERM). Already-renamed files remain intact. Temp `.tmp` files are left for cleanup or reuse.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a build operation in progress
  When SIGINT is received
  Then in-progress writes are abandoned cleanly
  And exit code is 130

  Given a build operation in progress
  When SIGTERM is received
  Then in-progress writes are abandoned cleanly
  And exit code is 143
  ```

#### SAFE-D004 Verbose Output with Phase Timing

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** Add elapsed-time tracking per pipeline phase in `--verbose` mode. Use `std::time::Instant` to measure and print duration after each stage (load, resolve, validate, render, route). Format as `[build] loaded N skills (XXms)`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a build with --verbose
  When the pipeline runs
  Then each phase prints its elapsed time in milliseconds
  ```

#### SAFE-D005 Verbose Resolved Variable Listing

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** In `--verbose` mode, print final resolved variables per skill after the Project Loader finishes. Show merged parent+child variable set. Format as `[build] skill <name> variables: theme=dark, lang=fr`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a build with --verbose
  When a skill has inherited parent variables
  Then the final resolved variables are printed in verbose output
  ```

#### SAFE-D006 Path Collision Detection

- **Type:** Feature
- **Effort:** 3
- **Dependencies:** None
- **Description:** Before writing, resolve all output paths for all skill-harness pairs and check for collisions (two pairs resolving to the same file path). Report all collisions as errors before any write. Integrate into the pipeline between validation and rendering. Collect all collisions — do not fail on first.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given two skills configured for the same harness
  When both resolve to the same output path
  Then a path collision error is reported for both skills
  And the build fails before any writes occur

  Given no path collisions
  When the build runs
  Then writing proceeds normally
  ```

#### SAFE-D007 Source Line/Column in Render Errors

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Extract MiniJinja error line/column information using `minijinja::Error::kind()` and `minijinja::Error::line()`/`minijinja::Error::column()`. Include source file path, line, and column in `EngineError::RenderError` diagnostics. Map template source paths so errors show the correct `.j2` file location.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a template with a syntax error at a specific line
  When rendering fails
  Then the error message includes the file path, line number, and column

  Given a template with an undefined variable
  When rendering fails
  Then the error message includes the variable name and source location
  ```

#### SAFE-D008 Missing Asset Directory Warning

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** When an asset directory listed in `skill.asset_dirs` does not exist on disk, emit a warning to stderr instead of silently skipping. Continue the build despite the warning. Include the skill name and expected path in the warning.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a skill with a non-existent asset directory in its config
  When building
  Then a warning is printed with the skill name and missing path
  And the build continues successfully
  ```

#### SAFE-D009 home_dir Fallback Returns Error

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** Change `router/paths.rs` `home_dir()` from falling back to `/tmp` to returning an error when `$HOME` is unset. User-scope builds should fail with a clear, actionable message: `$HOME is not set; cannot resolve user-scope path`. The function signature must change to return `Result`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given $HOME is not set in the environment
  When running build --target user
  Then an error message says "$HOME is not set"

  Given $HOME is set
  When running build --target user
  Then user paths resolve against $HOME as before
  ```

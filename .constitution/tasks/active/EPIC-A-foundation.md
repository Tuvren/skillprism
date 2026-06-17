# Epic A — Foundation

Acronym: **FND** | Story Points: **9**

---

#### FND-A001 Phase 0 — Developer Environment
- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Scaffold the Rust crate, configure the developer environment with devenv, and establish project conventions. No application logic.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a blank workspace
  When a developer runs `devenv shell`
  Then the Rust 1.85+ toolchain is available
  And `cargo build` compiles with zero warnings
  And `cargo fmt` applies `style_edition = "2024"` rules
  And `.gitignore` excludes `target/` and `.devenv/`
  ```

---

#### FND-A002 CLI Argument Parsing
- **Type:** Feature
- **Effort:** 2
- **Dependencies:** FND-A001
- **Description:** Implement the CLI entrypoint using clap derive. Parse `build` and `validate` subcommands. The `build` subcommand accepts `--target`, `--diff`, and `--force` flags; `validate` accepts an optional path argument. Define `TargetScope` enum (`Project`, `User`, `Dist`) and a `--verbose` global flag. Wire the CLI struct into `main.rs` as the sole entrypoint. Defer the `init` subcommand (SC-1/SC-2, P1) to future scope.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the compiled `skillprism` binary
  When invoked with `skillprism build --target user`
  Then it parses with subcommand "build" and target_scope "User"
  When invoked with `skillprism build --diff --force`
  Then both `diff` and `force` flags are set to true
  When invoked with `skillprism --verbose validate`
  Then `verbose` is set to true and subcommand is "validate"
  When invoked with `skillprism build --target invalid`
  Then it exits with a clap parse error
  ```

---

#### FND-A003 Project Configuration and Skill Loading
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** FND-A002
- **Description:** Parse `skillprism.yaml` (project config) and discover/parse individual `skill.yaml` files from the skills directory. Implement `ProjectConfig` and `SkillModel` structs per the TechSpec domain model. Support group-level variable merging (TC-3): when a parent directory has a `skill.yaml` and a child has its own, merge variables with child values winning on collision. Report structured errors for missing files, invalid YAML, and missing required fields using miette diagnostics.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a valid skillprism.yaml with skills_dir pointing to "skills/"
  And skills/ contains two valid skill.yaml files
  When the loader runs
  Then it returns a ProjectConfig with both skill models loaded
  When skillprism.yaml is missing
  Then it exits with a diagnostic error citing the missing file
  When a skill.yaml has an invalid YAML syntax
  Then it reports the parse error with file path and line number
  Given a parent skill.yaml with variables {theme: dark, lang: en}
  And a child skill.yaml with variables {lang: fr}
  When the loader merges them
  Then the resolved variables are {theme: dark, lang: fr}
  And child value "fr" wins over parent "en"
  ```

---

#### FND-A004 Harness Registry with Builtin Harnesses
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** FND-A003
- **Description:** Implement `HarnessRegistry` with two sources: (1) builtins compiled into the binary via `include_str!()` at compile time, and (2) user overrides loaded from the project's `harnesses/` directory at runtime. Author builtin harness YAML files under `src/builtin_harnesses/` for the five PRD-approved platforms (Claude Code, Codex, OpenCode, Factory, Pi). Provide a lookup method that resolves user overrides first, falling back to builtins.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a builtin harness "claude_code" is compiled into the binary
  When it is requested by name
  Then it resolves without a filesystem lookup
  And all required fields (name, capabilities, skill_ref_pattern) are populated
  Given a project with a user override harnesses/opencode.yaml
  When the registry resolves "opencode"
  Then the user override is returned instead of the builtin
  When a requested harness name does not exist in either source
  Then it returns a diagnostic error citing the unknown harness
  ```

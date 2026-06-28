# Epic H — Release Readiness

Acronym: **RELS** | Story Points: **13**

**Dependencies:** Epic G (CLEAN) — code quality should be clean before release artifact generation

---

#### RELS-H001 Shell Completions

- **Type:** Feature
- **Effort:** 2
- **Dependencies:** None
- **Description:** Add a `completions` subcommand to the CLI that generates shell completion scripts for bash, fish, and zsh using `clap_complete`. Alternatively, add a build script or Makefile target that generates completion files into a `completions/` directory. Document how to install them in the README.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the built binary
  When generating bash completions
  Then the output can be sourced to enable tab completion for all subcommands and flags

  Given completion files are generated
  When inspecting the bash completion output
  Then it includes the build, validate, and init subcommands
  ```

---

#### RELS-H002 Add --dry-run Alias for --diff

- **Type:** Feature
- **Effort:** 1
- **Dependencies:** None
- **Description:** Add `--dry-run` as a visible alias for `--diff` in the `Build` subcommand. Both flags should be interchangeable. Update `--help` output and any documentation references. Add a test verifying both flags produce identical behavior.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given running build --dry-run
  When executing
  Then it behaves identically to build --diff
  And no files are written

  Given the CLI help output
  When inspecting
  Then both --dry-run and --diff are listed
  ```

---

#### RELS-H003 .gitignore Polish

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Add missing common entries to `.gitignore`: `.direnv/` (nix/direnv cache), `dist/` (build output target), `*.tmp` (leftover atomic write temp files). Keep existing entries for `target/`, `.devenv/`, and `.pre-commit-config.yaml`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the .gitignore file
  When inspecting
  Then .direnv/, dist/, and *.tmp are listed as ignored patterns
  ```

---

#### RELS-H004 CLI Help Polish

- **Type:** Chore
- **Effort:** 2
- **Dependencies:** RELS-H001, RELS-H002
- **Description:** Review all clap `about`, `long_about`, and flag description strings across every subcommand for consistency, correctness, and professional tone. Ensure examples use consistent formatting. Verify `--help` output is scannable and complete for a v1 release.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the built binary
  When running skillprism --help
  Then every subcommand has a description and usage line
  And all flags are listed with consistent descriptions
  And no placeholder or TODO text appears in help output

  Given the built binary
  When running skillprism build --help
  Then --dry-run and --diff are both listed
  ```

---

#### RELS-H005 Man Page Generation

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** RELS-H001, RELS-H002
- **Description:** Generate a man page for the skillprism CLI. Provide a script or Makefile target to regenerate it. Include installation instructions in the README or a man page reference.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a built binary
  When generating the man page
  Then a skillprism.1 man page exists
  And man skillprism displays the correct synopsis, subcommands, and flags
  ```

---

#### RELS-H006 Release CI Workflow

- **Type:** Feature
- **Effort:** 3
- **Dependencies:** None
- **Description:** Add a GitHub Actions workflow triggered by git tags matching `v*`. Builds for Linux (x86_64) and macOS (x86_64 + ARM), attaches binaries to a GitHub Release, and optionally runs `cargo publish`. The release matrix should expand beyond the CI baseline to include a macOS ARM runner (e.g., `macos-14`).
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a tag v1.0.0 is pushed
  When the release workflow runs
  Then a GitHub Release is created with the tag name
  And Linux x86_64 binary is attached
  And macOS (x86_64 + ARM) binaries are attached
  ```

---

#### RELS-H007 User-Facing CHANGELOG

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** RELS-H001, RELS-H002, RELS-H003, RELS-H004, RELS-H005, RELS-H006
- **Description:** Create a `CHANGELOG.md` at the project root summarizing all work from Epics A through H in user-facing language. Target audience: someone evaluating or installing the tool. Reference the license, supported harnesses, and key CLI commands.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the repository root
  When inspecting CHANGELOG.md
  Then it lists all epics from A (Foundation) through H (Release Readiness)
  And each entry describes user-facing changes, not implementation detail
  ```

---

#### RELS-H008 Cargo Publish Readiness

- **Type:** Chore
- **Effort:** 2
- **Dependencies:** RELS-H006, RELS-H007
- **Description:** Verify all Cargo.toml metadata fields are correct for publication. Run `cargo publish --dry-run` to validate. Add any missing metadata (e.g., `[package.metadata.docs.rs]`, `[package.metadata.release]`). Publish to crates.io.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the Cargo.toml
  When running cargo publish --dry-run
  Then it passes without errors

  Given the dry-run passes
  When publishing to crates.io
  Then the crate is available under the skillprism name
  And version matches the release tag
  ```

# Epic H — Release Readiness

Acronym: **RELS** | Story Points: **5**

**Dependencies:** Epic G (CLEAN) — code quality should be clean before release artifact generation

---

#### RELS-H001 Add MIT LICENSE File

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Add the standard MIT license text as `LICENSE` to the project root. The copyright holder should match the repository owner (Tuvren). The `Cargo.toml` already declares `license = "MIT"` — this makes it official on disk.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the repository root
  When checking for a LICENSE file
  Then a file named LICENSE exists with the MIT license text
  And the copyright holder matches the repository owner
  ```

#### RELS-H002 Shell Completions

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

#### RELS-H003 Add --dry-run Alias for --diff

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

#### RELS-H004 .gitignore Polish

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Add missing common entries to `.gitignore`: `.direnv/` (nix/direnv cache), `dist/` (build output target), `*.tmp` (leftover atomic write temp files). Keep existing entries for `target/` and `.devenv/`.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the .gitignore file
  When inspecting
  Then .direnv/, dist/, and *.tmp are listed as ignored patterns
  ```

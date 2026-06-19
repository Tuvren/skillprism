# Epic F — Testing & CI

Acronym: **TEST** | Story Points: **8**

**Dependencies:** Epic D (SAFE) — path traversal fix should be in place before integration tests exercise path resolution

---

#### TEST-F001 Integration Test Suite

- **Type:** Feature
- **Effort:** 5
- **Dependencies:** None
- **Description:** Create `tests/integration/` with a complete fixture project directory containing skill definitions, templates, and multiple harness targets. Write end-to-end tests that exercise the full build pipeline (`ProjectLoader → HarnessResolver → Validator → Engine → Router`) and verify output file contents, paths, and sidecars. Use `assert_cmd` or `std::process::Command` to also test the CLI binary end-to-end. Include test cases for: successful build, validate command, diff output, and the `--target` flag variants.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a fixtures project with 2 skills and 2 harnesses
  When running the build pipeline
  Then output files exist at the expected paths with correct rendered content

  Given a fixtures project with a template error
  When running the validate command
  Then validation fails with a non-zero exit code and diagnostic output

  Given a fixtures project with existing output files
  When running build --diff
  Then diff output is printed to stdout without modifying any files
  ```

#### TEST-F002 CI Pipeline (GitHub Actions)

- **Type:** Chore
- **Effort:** 2
- **Dependencies:** TEST-F001
- **Description:** Create `.github/workflows/ci.yml` with a matrix build (Linux, macOS). Steps: checkout, install Rust 1.85 via `actions-rust-lang/setup-rust-toolchain`, `cargo build --all-targets`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`. Run on push to main and pull requests. Add a badge to README.md.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a PR is opened against the main branch
  When CI runs
  Then build, test, clippy, and fmt checks execute
  And a failure in any step fails the CI run

  Given a successful CI run
  When the workflow completes
  Then a passing badge is displayed in the README
  ```

#### TEST-F003 Pre-commit Hooks

- **Type:** Chore
- **Effort:** 1
- **Dependencies:** None
- **Description:** Create `.pre-commit-config.yaml` with hooks for `cargo fmt` and `cargo clippy`. Add setup instructions to README.md (`pip install pre-commit && pre-commit install`). The `cargo clippy` hook must use `-- -D warnings` to match CI strictness.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a staged change with formatting issues
  When running git commit
  Then pre-commit fails and reports formatting errors

  Given a staged change that passes clippy
  When running git commit
  Then the commit proceeds successfully
  ```

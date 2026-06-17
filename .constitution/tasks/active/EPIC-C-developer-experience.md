# Epic C — Developer Experience

Acronym: **DX** | Story Points: **10**

---

#### DX-C001 Diff Preview Mode
- **Type:** Feature
- **Effort:** 5
- **Dependencies:** PIPE-B004
- **Description:** Implement `--diff` flag behavior (OB-1). When `--diff` is set, render output to in-memory buffers and produce a colored diff between the rendered content and the current file on disk (if any). Display the diff to stdout instead of writing to disk. Handle new-file (no existing file) and deleted-target scenarios.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a skill with existing output on disk
  When running with `--diff` and rendered content differs
  Then a unified diff is printed with additions and removals highlighted
  When running with `--diff` and rendered content matches disk content
  Then a "no changes" message is printed for that file
  When running with `--diff` and no file exists on disk
  Then the full rendered content is shown as an addition
  When running without `--diff`
  Then no diff is computed and output is written to disk normally
  ```

---

#### DX-C002 Force Flag Enforcement and Error UX Polish
- **Type:** Feature
- **Effort:** 3
- **Dependencies:** DX-C001
- **Description:** Implement the `--force` flag (BD-2) to skip safety checks (overwriting user-scope files, missing output directories). Polish all miette diagnostic rendering for consistency (error codes, severity coloring, suggestion hints). Add human-readable error messages for common failure modes.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given a target with user-scope path that already exists
  When running without `--force`
  Then it prints a warning and skips the file
  When running with `--force`
  Then it overwrites the existing file without warning
  When the output path is invalid (permissions, read-only filesystem)
  Then it reports a diagnostic with the system error and the path
  ```

---

#### DX-C003 Rustdoc Documentation and README
- **Type:** Chore
- **Effort:** 2
- **Dependencies:** DX-C002
- **Description:** Document all public API items with rustdoc. Write a top-level README.md covering installation (cargo install, devenv), quickstart usage, supported agent platforms, and a development guide. Ensure `cargo doc --no-deps --document-private-items` produces complete documentation with no warnings.
- **Acceptance Criteria (Gherkin):**
  ```gherkin
  Given the project source
  When `cargo doc --no-deps` is run
  Then it completes with zero warnings
  And all public items have doc comments
  When the README.md is viewed
  Then it includes installation, quickstart, and development sections
  ```

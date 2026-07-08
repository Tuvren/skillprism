// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixtures_dir() -> PathBuf {
    project_root().join("tests/fixtures")
}

fn copy_fixture(name: &str) -> TempDir {
    let tmp = TempDir::with_prefix(format!("skillprism_{name}_")).unwrap();
    let src = fixtures_dir().join(name);
    cp_dir(&src, tmp.path()).unwrap();
    tmp
}

fn cp_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            cp_dir(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

/// Creates a temporary state directory and returns both paths.
struct TestEnv {
    project: TempDir,
    _state: TempDir,
    state_config: PathBuf,
}

impl TestEnv {
    fn new(fixture: &str) -> Self {
        let project = copy_fixture(fixture);
        let state = TempDir::with_prefix("skillprism_state_").unwrap();
        let state_config = state.path().to_path_buf();
        Self {
            project,
            _state: state,
            state_config,
        }
    }

    fn project_dir(&self) -> &Path {
        self.project.path()
    }

    fn bin(&self) -> Command {
        let mut cmd = Command::cargo_bin("skillprism").unwrap();
        cmd.current_dir(self.project_dir())
            .env("XDG_CONFIG_HOME", &self.state_config);
        cmd
    }
}

const SKILLPRISM_SKILL: &str = "skillprism-skill";
const PLAIN_SKILL: &str = "plain-skill";

#[test]
fn distribution_lifecycle_add_list_remove() {
    let env = TestEnv::new("dist-simple");

    // --- add ---
    env.bin()
        .arg("add")
        .arg(fixtures_dir().join("dist-simple"))
        .arg("--force")
        .assert()
        .success();

    // Verify both skills installed to each harness
    for skill in &[SKILLPRISM_SKILL, PLAIN_SKILL] {
        for harness in &["claude", "opencode"] {
            let output_path = env
                .project_dir()
                .join(format!(".{harness}/skills/{skill}/SKILL.md"));
            assert!(
                output_path.exists(),
                "expected {skill} at {}",
                output_path.display()
            );
        }
    }

    // Verify skillprism-skill rendered through harness (content differs per harness)
    let claude_content = fs::read_to_string(
        env.project_dir()
            .join(format!(".claude/skills/{SKILLPRISM_SKILL}/SKILL.md")),
    )
    .unwrap();
    assert!(claude_content.contains("Harness: claude"));
    let opencode_content = fs::read_to_string(
        env.project_dir()
            .join(format!(".opencode/skills/{SKILLPRISM_SKILL}/SKILL.md")),
    )
    .unwrap();
    assert!(opencode_content.contains("Harness: opencode"));

    // Verify plain-skill copied as-is (same content in both harnesses)
    for harness in &["claude", "opencode"] {
        let content = fs::read_to_string(
            env.project_dir()
                .join(format!(".{harness}/skills/{PLAIN_SKILL}/SKILL.md")),
        )
        .unwrap();
        assert!(content.contains("Version: A"));
    }

    // --- list ---
    let list_result = env.bin().arg("list").assert().success();
    let list_stdout = String::from_utf8_lossy(&list_result.get_output().stdout);
    assert!(
        list_stdout.contains(SKILLPRISM_SKILL),
        "list should contain {}",
        SKILLPRISM_SKILL
    );
    assert!(
        list_stdout.contains(PLAIN_SKILL),
        "list should contain {}",
        PLAIN_SKILL
    );

    // --- remove ---
    env.bin()
        .arg("remove")
        .arg("--all")
        .arg("--force")
        .assert()
        .success();

    // Verify files removed
    for skill in &[SKILLPRISM_SKILL, PLAIN_SKILL] {
        for harness in &["claude", "opencode"] {
            let output_path = env
                .project_dir()
                .join(format!(".{harness}/skills/{skill}/SKILL.md"));
            assert!(
                !output_path.exists(),
                "{skill} should be removed from {}",
                output_path.display()
            );
        }
    }

    // Verify list shows empty. The "No installed skills" notice is a status
    // message, so it goes to stderr; stdout stays clean for piping.
    let list_after_result = env.bin().arg("list").assert().success();
    let list_after_stdout = String::from_utf8_lossy(&list_after_result.get_output().stdout);
    let list_after_stderr = String::from_utf8_lossy(&list_after_result.get_output().stderr);
    assert!(
        list_after_stderr.contains("No installed skills"),
        "empty-list notice should be on stderr, got stderr: {list_after_stderr}"
    );
    assert!(
        list_after_stdout.trim().is_empty(),
        "stdout should stay clean when no skills are installed, got: {list_after_stdout}"
    );
}

#[test]
fn distribution_add_rejects_dist_target() {
    let env = TestEnv::new("dist-simple");

    let result = env
        .bin()
        .arg("add")
        .arg(fixtures_dir().join("dist-simple"))
        .arg("--target")
        .arg("dist")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        stderr.contains("invalid value") || stderr.contains("dist"),
        "should reject --target dist, got: {stderr}"
    );
}

#[test]
fn distribution_add_empty_source_is_usage_error() {
    // DIST-I002: a whitespace-only source is a usage error → exit code 2.
    // Source validation runs before any interactive scope/harness prompt, so
    // this fails fast with no flags even on a non-TTY test runner.
    let env = TestEnv::new("dist-simple");
    let result = env.bin().arg("add").arg("   ").assert().failure().code(2);
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        stderr.contains("source cannot be empty or whitespace"),
        "expected empty-source message, got: {stderr}"
    );
}

#[test]
fn distribution_add_undefined_variable_fails_without_writing() {
    // DIST-I002: a skillprism-format skill whose template references an
    // undefined variable must fail validation *before* anything is written.
    // Regression guard: `add` (and `update`) route skillprism renders through
    // the Validator, so a lenient-undefined render can never silently emit a
    // file with a blank in place of the missing value.
    let env = TestEnv::new("dist-undefined");

    let result = env
        .bin()
        .arg("add")
        .arg(fixtures_dir().join("dist-undefined"))
        .arg("--force")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        stderr.contains("this_var_is_not_defined") || stderr.contains("validation failed"),
        "expected an undefined-variable validation error, got: {stderr}"
    );

    // No partial output may be written to any harness on validation failure.
    for harness in &["claude", "opencode"] {
        let output_path = env
            .project_dir()
            .join(format!(".{harness}/skills/bad-skill/SKILL.md"));
        assert!(
            !output_path.exists(),
            "no file should be written on validation failure, found {}",
            output_path.display()
        );
    }
}

#[test]
fn distribution_remove_nonexistent_skill_fails() {
    let env = TestEnv::new("dist-simple");

    // First add a skill to create state
    env.bin()
        .arg("add")
        .arg(fixtures_dir().join("dist-simple"))
        .arg("--force")
        .assert()
        .success();

    // Then try to remove a non-existent skill
    let result = env
        .bin()
        .arg("remove")
        .arg("nonexistent")
        .arg("--force")
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        stderr.contains("not found")
            || stderr.contains("No matches")
            || stderr.contains("nonexistent"),
        "should report skill not found, got: {stderr}"
    );
}

fn init_git_repo(path: &Path) {
    run_git(path, &["init"])
        .args(["--initial-branch", "main"])
        .status()
        .unwrap();
    run_git(path, &["config", "user.email", "test@example.com"])
        .status()
        .unwrap();
    run_git(path, &["config", "user.name", "Test User"])
        .status()
        .unwrap();
}

fn commit_all(repo: &Path, message: &str) {
    run_git(repo, &["add", "."]).status().unwrap();
    run_git(repo, &["commit", "-m", message]).status().unwrap();
}

fn run_git(repo: &Path, args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(repo).args(args);
    cmd
}

#[test]
fn distribution_update_applies_source_changes() {
    let project = tempfile::TempDir::with_prefix("skillprism_update_project_").unwrap();
    fs::write(
        project.path().join("skillprism.yaml"),
        "name: update-test\nharnesses:\n  - claude\n  - opencode\nskills_dir: skills\n",
    )
    .unwrap();
    let source = copy_fixture("dist-update");
    let state = tempfile::TempDir::with_prefix("skillprism_update_state_").unwrap();

    init_git_repo(source.path());
    commit_all(source.path(), "Version A");

    let source_url = format!("file://{}", source.path().display());

    // Install version A from the local git repo.
    let mut add_cmd = Command::cargo_bin("skillprism").unwrap();
    add_cmd
        .current_dir(project.path())
        .env("XDG_CONFIG_HOME", state.path())
        .arg("add")
        .arg(&source_url)
        .arg("--force");
    add_cmd.assert().success();

    let read_plain = || {
        fs::read_to_string(
            project
                .path()
                .join(format!(".claude/skills/{PLAIN_SKILL}/SKILL.md")),
        )
        .unwrap()
    };
    let read_skillprism = || {
        fs::read_to_string(
            project
                .path()
                .join(format!(".claude/skills/{SKILLPRISM_SKILL}/SKILL.md")),
        )
        .unwrap()
    };

    assert!(read_plain().contains("Version: A"));
    assert!(read_skillprism().contains("Flavor: A"));

    // Mutate the source from version A to version B and commit.
    for path in [
        source.path().join(format!("skills/{PLAIN_SKILL}/SKILL.md")),
        source
            .path()
            .join(format!("skills/{SKILLPRISM_SKILL}/skill.yaml")),
    ] {
        let content = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            content
                .replace("Version: A", "Version: B")
                .replace("flavor: A", "flavor: B"),
        )
        .unwrap();
    }
    commit_all(source.path(), "Version B");

    // Run update --force and assert files reflect version B.
    let mut update_cmd = Command::cargo_bin("skillprism").unwrap();
    update_cmd
        .current_dir(project.path())
        .env("XDG_CONFIG_HOME", state.path())
        .arg("update")
        .arg("--force");
    let output = update_cmd.assert().success();
    // Status/progress goes to stderr; stdout is reserved for `--diff` output.
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(
        stderr.contains("Updated") || stderr.contains("is up to date"),
        "update should report progress on stderr, got: {stderr}"
    );

    assert!(
        read_plain().contains("Version: B"),
        "plain skill should be updated to version B"
    );
    assert!(
        read_skillprism().contains("Flavor: B"),
        "skillprism skill should be updated to version B"
    );
}

#[test]
fn distribution_update_no_skills_in_state() {
    let env = TestEnv::new("dist-simple");

    // First add a skill so state exists, then remove it so state is empty
    env.bin()
        .arg("add")
        .arg(fixtures_dir().join("dist-simple"))
        .arg("--force")
        .assert()
        .success();
    env.bin()
        .arg("remove")
        .arg("--all")
        .arg("--force")
        .assert()
        .success();

    // Now update with empty state
    let result = env.bin().arg("update").assert().success();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(
        stderr.contains("No installed skills"),
        "update with empty state should report 'No installed skills' on stderr, got: {stderr}"
    );
}

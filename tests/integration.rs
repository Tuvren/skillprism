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
use predicates::prelude::*;
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

fn bin(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("skillprism").unwrap();
    let isolated_home = home.join(".home");
    let isolated_config = home.join(".config");
    cmd.env("HOME", isolated_home)
        .env("XDG_CONFIG_HOME", isolated_config);
    cmd
}

#[test]
fn full_build_pipeline() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--force")
        .assert();
    assert.success();

    // 2 skills × 2 harnesses = 4 output files
    for skill in &["alpha", "beta"] {
        for harness in &["claude", "opencode"] {
            let output_path = project_dir.join(format!(".{harness}/skills/{skill}/SKILL.md"));
            assert!(
                output_path.exists(),
                "expected output at {}",
                output_path.display()
            );
        }
    }

    // Verify rendered content for alpha × claude
    let alpha_claude =
        fs::read_to_string(project_dir.join(".claude/skills/alpha/SKILL.md")).unwrap();
    // The Agent Skills spec requires YAML frontmatter (name + description) — without
    // it no client can discover the skill. The fixture templates must emit it.
    assert!(
        alpha_claude.starts_with("---\n"),
        "rendered SKILL.md must start with YAML frontmatter, got: {}",
        &alpha_claude[..alpha_claude.len().min(80)]
    );
    assert!(alpha_claude.contains("name: alpha"));
    assert!(alpha_claude.contains("description: First test skill"));
    assert!(alpha_claude.contains("# alpha"));
    assert!(alpha_claude.contains("Hello from Alpha"));
    assert!(alpha_claude.contains("Theme: dark"));
    assert!(alpha_claude.contains("Harness: claude (Claude Code)"));

    // Verify rendered content for beta × opencode
    let beta_opencode =
        fs::read_to_string(project_dir.join(".opencode/skills/beta/SKILL.md")).unwrap();
    assert!(
        beta_opencode.starts_with("---\n"),
        "rendered SKILL.md must start with YAML frontmatter"
    );
    assert!(beta_opencode.contains("name: beta"));
    assert!(beta_opencode.contains("description: Second test skill"));
    assert!(beta_opencode.contains("# beta"));
    assert!(beta_opencode.contains("Hello from Beta"));
    assert!(beta_opencode.contains("Message:"));

    // Verify manifest files exist for claude (has plugin.json) with correct content
    let manifest_path = project_dir.join(".claude/plugin.json");
    assert!(manifest_path.exists(), "claude manifest should exist");
    let manifest_content = fs::read_to_string(manifest_path).unwrap();
    assert!(
        manifest_content.contains("alpha"),
        "manifest should reference alpha skill"
    );
    assert!(
        manifest_content.contains("beta"),
        "manifest should reference beta skill"
    );
    assert!(
        manifest_content.starts_with('['),
        "manifest should be a JSON array"
    );
    assert!(
        manifest_content.ends_with(']'),
        "manifest should be a JSON array"
    );
}

#[test]
fn validate_reports_errors() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    // Introduce a syntax error into one template
    let broken_template = project_dir.join("skills/alpha/SKILL.md.j2");
    fs::write(&broken_template, "# {{ broken\n").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("validate")
        .assert();
    assert.failure().code(predicate::ne(0)).stderr(
        predicate::str::contains("Syntax error")
            .or(predicate::str::contains("unexpected"))
            .or(predicate::str::contains("expected")),
    );
}

#[test]
fn completions_produce_output() {
    let tmp = TempDir::with_prefix("skillprism_completions_").unwrap();
    for shell in &["bash", "fish", "zsh"] {
        let assert = bin(tmp.path())
            .arg("completions")
            .arg(shell)
            .assert()
            .success();
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            !stdout.is_empty(),
            "completions for {shell} should produce output"
        );
        assert!(
            stdout.contains("skillprism"),
            "completions for {shell} should reference the binary name"
        );
    }
}

#[test]
fn build_diff_does_not_write() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--diff")
        .assert();
    assert.success();

    assert!(
        !project_dir.join(".claude").exists(),
        "diff mode must not write output files"
    );
    assert!(
        !project_dir.join(".opencode").exists(),
        "diff mode must not write output files"
    );

    // First do a real build so files exist
    let build_assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--force")
        .assert();
    build_assert.success();

    // Now run build --diff — should show diff output without modifying files
    let diff_assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--diff")
        .assert()
        .success();

    // stdout should contain diff output
    let stdout = String::from_utf8_lossy(&diff_assert.get_output().stdout);
    assert!(
        stdout.contains("no changes"),
        "expected 'no changes' in diff output, got: {stdout}"
    );

    // Verify no files were modified (diff is read-only)
    for skill in &["alpha", "beta"] {
        for harness in &["claude", "opencode"] {
            let output_path = project_dir.join(format!(".{harness}/skills/{skill}/SKILL.md"));
            assert!(output_path.exists(), "file should still exist after --diff");
        }
    }
}

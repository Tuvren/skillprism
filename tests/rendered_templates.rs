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

//! End-to-end coverage for the `.j2` suffix rule: every `*.j2` file under a
//! skill directory (root and nested) renders once per harness with the suffix
//! stripped, while non-`.j2` files keep the existing copy behavior.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

const SKILL_TEMPLATE: &str = "---\nname: {{ skill_name | yaml_str }}\ndescription: {{ skill_description | yaml_str }}\n---\nBody\n";

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("skillprism.yaml"),
        "harnesses: [claude]\nskills_dir: skills\n",
    )
    .unwrap();
    dir
}

fn add_skill(root: &Path, name: &str) {
    let skill = root.join("skills").join(name);
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("skill.yaml"),
        format!("skillprism: '1'\nname: {name}\ndescription: A test skill\n"),
    )
    .unwrap();
    fs::write(skill.join("SKILL.md.j2"), SKILL_TEMPLATE).unwrap();
}

fn bin(root: &Path) -> Command {
    let mut cmd = Command::cargo_bin("skillprism").unwrap();
    cmd.current_dir(root)
        .env("HOME", root.join(".home"))
        .env("XDG_CONFIG_HOME", root.join(".config"))
        .env("NO_COLOR", "1")
        .env_remove("NO_GRAPHICS");
    cmd
}

#[test]
fn renders_every_j2_template_and_strips_the_suffix() {
    let dir = project();
    add_skill(dir.path(), "sample");
    let skill = dir.path().join("skills/sample");

    fs::create_dir_all(skill.join("references")).unwrap();
    fs::create_dir_all(skill.join("notes/deep")).unwrap();
    fs::write(
        skill.join("references/ref.md.j2"),
        "# Ref for {{ harness.id }}\n",
    )
    .unwrap();
    fs::write(
        skill.join("notes/deep/overview.md.j2"),
        "Overview on {{ harness.id }}\n",
    )
    .unwrap();
    fs::write(skill.join("note.md.j2"), "Root note for {{ harness.id }}\n").unwrap();

    // A non-`.j2` file in a copied directory stays literal.
    fs::write(
        skill.join("references/plain.md"),
        "Literal {{ harness.id }}\n",
    )
    .unwrap();
    // A root-level non-`.j2` file is not copied.
    fs::write(skill.join("root.txt"), "uncopied\n").unwrap();

    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();

    let out = dir.path().join("dist/claude/sample");
    assert_eq!(
        fs::read_to_string(out.join("references/ref.md")).unwrap(),
        "# Ref for claude\n"
    );
    assert!(!out.join("references/ref.md.j2").exists());
    assert_eq!(
        fs::read_to_string(out.join("notes/deep/overview.md")).unwrap(),
        "Overview on claude\n"
    );
    assert!(!out.join("notes/deep/overview.md.j2").exists());
    assert_eq!(
        fs::read_to_string(out.join("note.md")).unwrap(),
        "Root note for claude\n"
    );
    assert!(!out.join("note.md.j2").exists());

    // Copied verbatim, not rendered.
    assert_eq!(
        fs::read_to_string(out.join("references/plain.md")).unwrap(),
        "Literal {{ harness.id }}\n"
    );
    assert!(!out.join("root.txt").exists());
}

#[test]
fn skill_template_is_not_rendered_twice() {
    let dir = project();
    add_skill(dir.path(), "sample");
    // A root-level `.j2` that is not the skill template still renders.
    fs::write(
        dir.path().join("skills/sample/note.md.j2"),
        "note {{ harness.id }}\n",
    )
    .unwrap();

    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();

    let out = dir.path().join("dist/claude/sample");
    let skill_content = fs::read_to_string(out.join("SKILL.md")).unwrap();
    assert!(skill_content.starts_with("---\nname: \"sample\"\n"));
    assert_eq!(skill_content.matches("Body").count(), 1);
    assert!(!out.join("SKILL.md.j2").exists());
}

#[test]
fn validate_rejects_syntax_error_in_extra_template() {
    let dir = project();
    add_skill(dir.path(), "sample");
    fs::create_dir_all(dir.path().join("skills/sample/references")).unwrap();
    fs::write(
        dir.path().join("skills/sample/references/broken.md.j2"),
        "{{ broken\n",
    )
    .unwrap();

    let assert = bin(dir.path()).arg("validate").assert().code(1);
    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap();
    assert!(stderr.contains("syntax"), "{stderr}");
    assert!(stderr.contains("broken.md.j2"), "{stderr}");
}

#[test]
fn build_rejects_syntax_error_in_extra_template() {
    let dir = project();
    add_skill(dir.path(), "sample");
    fs::create_dir_all(dir.path().join("skills/sample/references")).unwrap();
    fs::write(
        dir.path().join("skills/sample/references/broken.md.j2"),
        "{{ broken\n",
    )
    .unwrap();

    let assert = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap();
    assert!(stderr.contains("broken.md.j2"), "{stderr}");
    assert!(!dir.path().join("dist/claude/sample").exists());
}

#[test]
fn validate_and_build_reject_undefined_variable_in_extra_template() {
    let dir = project();
    add_skill(dir.path(), "sample");
    fs::create_dir_all(dir.path().join("skills/sample/references")).unwrap();
    fs::write(
        dir.path().join("skills/sample/references/bad.md.j2"),
        "{{ unknown_thing }}\n",
    )
    .unwrap();

    let validate = bin(dir.path()).arg("validate").assert().code(1);
    let stderr = std::str::from_utf8(&validate.get_output().stderr).unwrap();
    assert!(stderr.contains("unknown_thing"), "{stderr}");
    assert!(stderr.contains("bad.md.j2"), "{stderr}");

    let build = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&build.get_output().stderr).unwrap();
    assert!(stderr.contains("unknown_thing"), "{stderr}");
    assert!(!dir.path().join("dist/claude/sample").exists());
}

#[test]
fn rendered_template_colliding_with_copied_file_fails() {
    let dir = project();
    add_skill(dir.path(), "sample");
    let references = dir.path().join("skills/sample/references");
    fs::create_dir_all(&references).unwrap();
    fs::write(references.join("ref.md.j2"), "rendered\n").unwrap();
    fs::write(references.join("ref.md"), "copied\n").unwrap();

    let assert = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assert.get_output().stderr).unwrap();
    assert!(
        stderr.contains("Path collision") || stderr.contains("Build aborted"),
        "{stderr}"
    );
    assert!(!dir.path().join("dist/claude/sample").exists());
}

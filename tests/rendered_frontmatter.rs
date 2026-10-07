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
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

const ISSUE_DESCRIPTION: &str =
    "Router: use mode X. See also: reference docs. Say \"hello\" and use `gh` CLI.";

fn project(template: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("skillprism.yaml"),
        "harnesses: [claude, codex]\n",
    )
    .unwrap();
    add_skill(dir.path(), "sample", template);
    dir
}

fn add_skill(root: &Path, name: &str, template: &str) {
    let skill = root.join("skills").join(name);
    fs::create_dir_all(&skill).unwrap();
    let description = yaml_serde::to_string(ISSUE_DESCRIPTION).unwrap();
    fs::write(
        skill.join("skill.yaml"),
        format!("skillprism: '1'\nname: {name}\ndescription: {description}"),
    )
    .unwrap();
    fs::write(skill.join("SKILL.md"), template).unwrap();
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

fn assert_no_skill_output(root: &Path, name: &str) {
    for harness in ["claude", "codex"] {
        assert!(!root.join("dist").join(harness).join(name).exists());
    }
}

#[test]
fn build_rejects_issue_repro_with_rendered_location_and_filter_hint() {
    let dir = project("---\nname: {{ skill_name }}\ndescription: {{ skill_description }}\n---\n");
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    for expected in [
        "sample",
        "claude",
        "codex",
        "description",
        "SKILL.md",
        "yaml_str",
        "frontmatter line 2",
        "rendered line 3",
        "mapping values",
    ] {
        assert!(stderr.contains(expected), "missing {expected}: {stderr}");
    }
    assert!(stderr.contains("skills/sample/SKILL.md"), "{stderr}");
    assert!(!stderr.contains("\u{1b}["), "{stderr}");
    assert_no_skill_output(dir.path(), "sample");
}

#[test]
fn validate_remains_parse_only_for_issue_repro() {
    let dir = project("---\nname: {{ skill_name }}\ndescription: {{ skill_description }}\n---\n");
    bin(dir.path())
        .arg("validate")
        .assert()
        .success()
        .stdout(predicates::str::contains("Validation passed"));
    assert!(!dir.path().join("dist").exists());
}

#[test]
fn build_with_yaml_str_round_trips_issue_description() {
    let dir = project(
        "---\nname: {{ skill_name | yaml_str }}\ndescription: {{ skill_description | yaml_str }}\n---\nBody\n",
    );
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    for harness in ["claude", "codex"] {
        let rendered = fs::read_to_string(
            dir.path()
                .join("dist")
                .join(harness)
                .join("sample/SKILL.md"),
        )
        .unwrap();
        let frontmatter = rendered
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---")
            .unwrap()
            .0;
        let value: yaml_serde::Value = yaml_serde::from_str(frontmatter).unwrap();
        assert_eq!(value["description"].as_str(), Some(ISSUE_DESCRIPTION));
    }
}

#[test]
fn build_checks_bom_prefixed_frontmatter_and_accepts_yaml_str() {
    let dir =
        project("\u{feff}---\nname: {{ skill_name }}\ndescription: {{ skill_description }}\n---\n");
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    for expected in [
        "description",
        "frontmatter line 2",
        "rendered line 3",
        "mapping values",
        "yaml_str",
        "rendered claude/SKILL.md:3:20",
        "rendered codex/SKILL.md:3:20",
    ] {
        assert!(stderr.contains(expected), "missing {expected}: {stderr}");
    }
    assert_no_skill_output(dir.path(), "sample");

    fs::write(
        dir.path().join("skills/sample/SKILL.md"),
        "\u{feff}---\nname: {{ skill_name }}\ndescription: {{ skill_description | yaml_str }}\n---\n",
    )
    .unwrap();
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    for harness in ["claude", "codex"] {
        let rendered = fs::read_to_string(
            dir.path()
                .join("dist")
                .join(harness)
                .join("sample/SKILL.md"),
        )
        .unwrap();
        let frontmatter = rendered
            .strip_prefix("\u{feff}---\n")
            .unwrap()
            .split_once("\n---")
            .unwrap()
            .0;
        let value: yaml_serde::Value = yaml_serde::from_str(frontmatter).unwrap();
        assert_eq!(value["description"].as_str(), Some(ISSUE_DESCRIPTION));
    }
}

#[test]
fn build_checks_opening_fences_with_trailing_spaces_and_tabs() {
    for (bom, newline, suffix) in [
        ("", "\n", " "),
        ("", "\n", "\t"),
        ("\u{feff}", "\r\n", " \t"),
    ] {
        let dir = project(&format!(
            "{bom}---{suffix}{newline}name: sample{newline}description: invalid: YAML{newline}---{newline}"
        ));
        let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
        let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
        for expected in [
            "description",
            "mapping values",
            "yaml_str",
            "rendered line 3",
        ] {
            assert!(stderr.contains(expected), "missing {expected}: {stderr}");
        }
        assert_no_skill_output(dir.path(), "sample");
    }
}

#[test]
fn build_accepts_closing_fences_with_trailing_spaces_and_tabs() {
    for (bom, newline, suffix) in [
        ("", "\n", " "),
        ("", "\n", "\t"),
        ("\u{feff}", "\r\n", " \t"),
    ] {
        let dir = project(&format!(
            "{bom}---{newline}name: sample{newline}description: safe{newline}---{suffix}{newline}Body{newline}"
        ));
        bin(dir.path())
            .args(["build", "--force"])
            .assert()
            .success();
    }
}

#[test]
fn build_reports_yaml_str_collection_rejection_detail() {
    let dir = project(
        "---\nname: sample\ndescription: safe\narguments: {{ arguments | yaml_str }}\n---\n",
    );
    let config = dir.path().join("skills/sample/skill.yaml");
    let mut content = fs::read_to_string(&config).unwrap();
    content.push_str("arguments: [one, two]\n");
    fs::write(config, content).unwrap();
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    assert!(
        stderr.contains("yaml_str only accepts scalar values; leave list fields unfiltered"),
        "{stderr}"
    );
    assert!(stderr.contains("at line 4"), "{stderr}");
    assert_no_skill_output(dir.path(), "sample");
}

#[test]
fn build_checks_string_fields_supplied_by_yaml_merges() {
    for field in ["name", "description"] {
        for merge in ["*d", "[*d]"] {
            let dir = project(&format!(
                "---\ndefaults: &d\n  {field}: 123\n<<: {merge}\n---\n"
            ));
            let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
            let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
            for expected in [field, "YAML string", "resolving YAML merges", "yaml_str"] {
                assert!(stderr.contains(expected), "missing {expected}: {stderr}");
            }
            assert_no_skill_output(dir.path(), "sample");
        }
    }
    // Explicit strings override non-string defaults under YAML merge semantics.
    let dir = project(
        "---\ndefaults: &d {name: 123, description: false}\n<<: *d\nname: sample\ndescription: safe\n---\n",
    );
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
}

#[test]
fn build_diagnostic_uses_the_harness_skill_filename() {
    let dir = project("---\nname: sample\ndescription: 123\n---\n");
    fs::write(dir.path().join("skillprism.yaml"), "harnesses: [custom]\n").unwrap();
    let harnesses = dir.path().join("harnesses");
    fs::create_dir(&harnesses).unwrap();
    fs::write(
        harnesses.join("custom.yaml"),
        "id: custom\nname: Custom\nversion: '1'\ncapabilities:\n  supports_subagent: false\npaths:\n  project_scope_path: .custom/skills\n  user_scope_path: .custom/skills\n  skill_filename: GUIDE.md\n",
    )
    .unwrap();
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    for expected in [
        "Invalid rendered GUIDE.md frontmatter",
        "rendered custom/GUIDE.md:3:",
    ] {
        assert!(stderr.contains(expected), "missing {expected}: {stderr}");
    }
    assert!(!dir.path().join("dist/custom/sample").exists());
}

#[test]
fn build_rejects_non_string_name_and_description() {
    for field in ["name", "description"] {
        for value in ["null", "~", "123", "true", "false", "[]", "{}"] {
            let dir = project(&format!("---\n{field}: {value}\n---\n"));
            let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
            let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
            for expected in [
                "sample",
                "claude",
                "codex",
                field,
                "YAML string",
                "yaml_str",
                "rendered line 2",
            ] {
                assert!(
                    stderr.contains(expected),
                    "{field}: {value}; missing {expected}: {stderr}"
                );
            }
            assert_no_skill_output(dir.path(), "sample");
        }
    }
}

#[test]
fn build_reports_every_broken_skill_and_harness_pair() {
    let dir = project("---\ndescription: {{ skill_description }}\n---\n");
    add_skill(dir.path(), "second", "---\nname: null\n---\n");
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    for skill in ["sample", "second"] {
        for harness in ["claude", "codex"] {
            assert!(stderr.contains(&format!("[{skill}] {harness}")), "{stderr}");
        }
        assert_no_skill_output(dir.path(), skill);
    }
}

#[test]
fn build_requires_a_mapping_and_closed_frontmatter() {
    for (template, help, label) in [
        (
            "---\n[]\n---\n",
            "Add a YAML mapping",
            "expected a YAML frontmatter mapping",
        ),
        (
            "---\nscalar\n---\n",
            "Add a YAML mapping",
            "expected a YAML frontmatter mapping",
        ),
        (
            "---\n---\n",
            "Add a YAML mapping",
            "expected a YAML frontmatter mapping",
        ),
        (
            "---\nname: sample\n",
            "Add a closing --- fence",
            "frontmatter starts here but has no closing fence",
        ),
    ] {
        let dir = project(template);
        let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
        let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
        assert!(stderr.contains(help), "{stderr}");
        assert!(stderr.contains(label), "{stderr}");
        assert!(!stderr.contains("yaml_str"), "{stderr}");
        assert_no_skill_output(dir.path(), "sample");
    }
}

#[test]
fn build_accepts_strings_independent_of_skill_config_and_only_checks_leading_frontmatter() {
    // yaml_serde uses YAML 1.2: yes is a string, not a coerced boolean.
    for template in [
        "---\nname: yes\ndescription: a different string\n---\n",
        "---\n\"name\": renamed\n'description': \"123\"\n---\n",
        "---\r\nname: renamed\r\ndescription: safe\r\n---\r\n",
        "---\nother: 123\n---\n---\nname: null\n---\n",
        "Body\n---\nname: null\n---\n",
        "No frontmatter: {{ skill_description }}\n",
    ] {
        let dir = project(template);
        bin(dir.path())
            .args(["build", "--force"])
            .assert()
            .success();
    }
}

#[test]
fn build_locates_non_string_fields_in_flow_mappings() {
    let dir = project("---\n{name: safe, description: 123}\n---\n");
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    for expected in ["description", "YAML string", "rendered line 2", "yaml_str"] {
        assert!(stderr.contains(expected), "{stderr}");
    }
}

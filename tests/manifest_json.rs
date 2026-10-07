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

const DESCRIPTION: &str =
    "Say \"hello\" followed by a backslash \\, then a real newline\nthen more";
const MANIFESTS: [(&str, &str); 2] = [
    ("claude", "dist/claude/.claude/plugin.json"),
    ("codex", "dist/codex/.agents/marketplace.json"),
];

fn project() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("skillprism.yaml"),
        "harnesses: [claude, codex]\n",
    )
    .unwrap();
    let skill = dir.path().join("skills/sample");
    fs::create_dir_all(&skill).unwrap();
    fs::write(
        skill.join("skill.yaml"),
        format!(
            "skillprism: '1'\nname: sample\ndescription: {}",
            yaml_serde::to_string(DESCRIPTION).unwrap()
        ),
    )
    .unwrap();
    fs::write(
        skill.join("SKILL.md"),
        "---\nname: {{ skill_name | yaml_str }}\ndescription: {{ skill_description | yaml_str }}\n---\nBody\n",
    )
    .unwrap();
    dir
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

fn custom_harness(root: &Path, template: &str) {
    fs::write(
        root.join("skillprism.yaml"),
        "harnesses: [claude, codex, custom]\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("harnesses")).unwrap();
    let mut definition: yaml_serde::Value = yaml_serde::from_str(
        "id: custom\nname: Custom\ncapabilities:\n  supports_subagent: true\n  requires_manifest: true\npaths:\n  project_scope_path: .custom/skills\n  user_scope_path: .custom/skills\n  skill_filename: SKILL.md\n  manifest_scope_path: .custom\n  manifest_filename: index.json\nmanifest:\n  template: ''\n",
    )
    .unwrap();
    definition["manifest"]["template"] = template.into();
    fs::write(
        root.join("harnesses/custom.yaml"),
        yaml_serde::to_string(&definition).unwrap(),
    )
    .unwrap();
}

#[test]
fn built_in_manifest_descriptions_round_trip_json() {
    let dir = project();
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    for (harness, path) in MANIFESTS {
        let content = fs::read_to_string(dir.path().join(path)).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&content)
            .unwrap_or_else(|error| panic!("{harness}: {error}; {content}"));
        assert_eq!(manifest[0]["name"], "sample");
        assert_eq!(manifest[0]["description"], DESCRIPTION);
        assert_eq!(manifest[0]["skill_ref"], "/sample");
    }
}

#[test]
fn built_in_manifest_skill_refs_escape_overridden_patterns() {
    let dir = project();
    fs::write(
        dir.path().join("skills/sample/skill.yaml"),
        "skillprism: '1'\nname: sample\ndescription: safe\n",
    )
    .unwrap();
    let pattern = "quote\"backslash\\{name}";
    fs::create_dir_all(dir.path().join("harnesses")).unwrap();
    for (harness, definition) in [
        (
            "claude",
            include_str!("../src/builtin_harnesses/claude.yaml"),
        ),
        ("codex", include_str!("../src/builtin_harnesses/codex.yaml")),
    ] {
        let mut definition: yaml_serde::Value = yaml_serde::from_str(definition).unwrap();
        definition["skill_ref_pattern"] = pattern.into();
        fs::write(
            dir.path().join(format!("harnesses/{harness}.yaml")),
            yaml_serde::to_string(&definition).unwrap(),
        )
        .unwrap();
    }
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    for (_, path) in MANIFESTS {
        let content = fs::read_to_string(dir.path().join(path)).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(
            manifest[0]["skill_ref"],
            pattern.replace("{name}", "sample")
        );
    }
}

#[test]
fn hand_quoted_custom_manifest_fails_before_any_write_or_diff() {
    for flags in [
        vec!["build"],
        vec!["build", "--force"],
        vec!["build", "--diff"],
    ] {
        let dir = project();
        let good_skill = dir.path().join("skills/good-skill");
        fs::create_dir_all(&good_skill).unwrap();
        fs::write(
            good_skill.join("skill.yaml"),
            "skillprism: '1'\nname: good-skill\ndescription: safe\n",
        )
        .unwrap();
        fs::copy(
            dir.path().join("skills/sample/SKILL.md"),
            good_skill.join("SKILL.md"),
        )
        .unwrap();
        custom_harness(dir.path(), r#"{"description": "{{ skill_description }}"}"#);
        let assertion = bin(dir.path()).args(&flags).assert().code(1);
        let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
        for expected in [
            "[sample] custom:",
            "dist/custom/.custom/index.json",
            "JSON",
            "line",
            "column",
            "tojson",
            "without surrounding quotes",
        ] {
            assert!(stderr.contains(expected), "missing {expected}: {stderr}");
        }
        let rendered_entry = format!(r#"{{"description": "{DESCRIPTION}"}}"#);
        let reason = serde_json::from_str::<serde_json::Value>(&rendered_entry)
            .unwrap_err()
            .to_string();
        assert_eq!(stderr.matches(&reason).count(), 1, "{stderr}");
        assert!(!stderr.contains("[good-skill]"), "{stderr}");
        assert!(!stderr.contains("format: json"), "{stderr}");
        assert!(!stderr.contains("\u{1b}["), "{stderr}");
        assert!(!dir.path().join("dist").exists(), "{flags:?} wrote output");
        assert!(
            assertion.get_output().stdout.is_empty(),
            "printed a partial diff"
        );
    }
}

#[test]
fn manifest_render_errors_fail_before_any_write() {
    let dir = project();
    custom_harness(dir.path(), "{{ skill_ref() }}");
    let assertion = bin(dir.path()).args(["build", "--force"]).assert().code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    assert!(stderr.contains("custom"), "{stderr}");
    assert!(!dir.path().join("dist").exists());
}

#[test]
fn invalid_custom_manifest_preserves_existing_output_with_force() {
    let dir = project();
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    let skill_path = dir.path().join("dist/claude/sample/SKILL.md");
    let manifest_path = dir.path().join(MANIFESTS[0].1);
    let old_manifest = fs::read_to_string(&manifest_path).unwrap();
    fs::write(&skill_path, "preserve this output").unwrap();
    custom_harness(dir.path(), r#"{"description": "{{ skill_description }}"}"#);
    bin(dir.path()).args(["build", "--force"]).assert().code(1);
    assert_eq!(
        fs::read_to_string(skill_path).unwrap(),
        "preserve this output"
    );
    assert_eq!(fs::read_to_string(manifest_path).unwrap(), old_manifest);
    assert!(!dir.path().join("dist/custom").exists());
}

#[test]
fn docs_manifest_example_builds_and_round_trips_json() {
    let docs = include_str!("../site/content/docs/harnesses.md");
    let example = docs
        .split_once("### Manifests")
        .unwrap()
        .1
        .split_once("```yaml\n")
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0;
    let value: yaml_serde::Value = yaml_serde::from_str(example).unwrap();
    let template = value["manifest"]["template"].as_str().unwrap();
    assert!(value["manifest"].get("format").is_none());
    assert!(template.contains("{{ skill_name | tojson }}"));
    assert!(template.contains("{{ skill_description | tojson }}"));
    assert!(!template.contains("\"{{"));
    let dir = project();
    custom_harness(dir.path(), template);
    let harness_path = dir.path().join("harnesses/custom.yaml");
    let mut definition: yaml_serde::Value =
        yaml_serde::from_str(&fs::read_to_string(&harness_path).unwrap()).unwrap();
    // Pass the actual documented manifest mapping through ManifestDef's loader.
    definition["manifest"] = value["manifest"].clone();
    fs::write(harness_path, yaml_serde::to_string(&definition).unwrap()).unwrap();
    bin(dir.path())
        .args(["build", "--force"])
        .assert()
        .success();
    let content = fs::read_to_string(dir.path().join("dist/custom/.custom/index.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(manifest[0]["name"], "sample");
    assert_eq!(manifest[0]["description"], DESCRIPTION);
}

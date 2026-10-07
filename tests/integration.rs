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
fn graphical_diagnostics_piped_no_color() {
    let tmp = copy_fixture("invalid-unknown-project-field");
    let assertion = bin(tmp.path())
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .env_remove("NO_GRAPHICS")
        .env("FORCE_COLOR", "1")
        .env("TERM", "xterm-256color")
        .arg("validate")
        .assert()
        .code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    assert!(!stderr.contains("Diagnostic {"), "{stderr}");
    assert!(!stderr.contains("install miette"), "{stderr}");
    assert!(stderr.contains("Invalid config in"), "{stderr}");
    assert!(stderr.contains("unknown field `name`"), "{stderr}");
    assert!(
        stderr.contains("Check field names and value types"),
        "{stderr}"
    );
    assert!(
        stderr.contains("https://tuvren.github.io/skillprism/docs/quickstart/"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("https://tuvren.github.io/skillprism/docs/skill-yaml/"),
        "{stderr}"
    );
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    assert!(stderr.is_ascii(), "{stderr}");
}

#[test]
fn documentation_frontmatter_strings_use_yaml_str() {
    const NON_STRING_FIELDS: &[&str] = &[
        "metadata",
        "arguments",
        "disable_model_invocation",
        "user_invocable",
        "disallowed_tools",
        "context_fork",
        "hooks",
        "activation_paths",
        "required_capabilities",
    ];

    for (page, read_error) in [
        (
            "site/content/docs/templating.md",
            "read site/content/docs/templating.md",
        ),
        (
            "site/content/docs/quickstart.md",
            "read site/content/docs/quickstart.md",
        ),
    ] {
        let content = fs::read_to_string(project_root().join(page)).expect(read_error);
        let mut lines = content.lines().enumerate();
        while let Some((_, line)) = lines.next() {
            let line = line.trim_start();
            let fence = if line.starts_with("```") {
                "```"
            } else if line.starts_with("~~~") {
                "~~~"
            } else {
                continue;
            };
            let mut block = lines
                .by_ref()
                .take_while(|(_, line)| !line.trim_start().starts_with(fence));
            let mut in_frontmatter = block.next().is_some_and(|(_, line)| line.trim() == "---");
            for (line_number, line) in block {
                if line.trim() == "---" {
                    in_frontmatter = false;
                }
                if !in_frontmatter || line.contains("yaml_str") {
                    continue;
                }
                for expression in line.split("{{").skip(1) {
                    let name = expression.split(['|', '}']).next().unwrap_or("").trim();
                    assert!(
                        NON_STRING_FIELDS.contains(&name),
                        "{page}:{}: frontmatter string interpolation needs yaml_str: {line}",
                        line_number + 1
                    );
                }
            }
        }
    }
}

#[test]
fn validate_accepts_yaml_str_filter() {
    let tmp = copy_fixture("valid");
    fs::write(
        tmp.path().join("skills/alpha/SKILL.md.j2"),
        "---\nname: {{ skill_name | yaml_str }}\ndescription: {{ skill_description | yaml_str }}\n---\nBody\n",
    )
    .unwrap();
    bin(tmp.path())
        .current_dir(tmp.path())
        .arg("validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("Validation passed"));
}

#[test]
fn scaffolded_skills_preserve_hostile_description_in_frontmatter() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("demo");
    bin(tmp.path())
        .current_dir(tmp.path())
        .args(["init", "project", "demo", "-H", "claude"])
        .assert()
        .success();
    bin(tmp.path())
        .current_dir(&project)
        .args(["init", "skill", "additional"])
        .assert()
        .success();

    let description = "Router: use mode X. Say \"hello\" ok";
    for name in ["sample", "additional"] {
        let config_path = project.join(format!("skills/{name}/skill.yaml"));
        let mut config: yaml_serde::Value =
            yaml_serde::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        config["description"] = yaml_serde::Value::String(description.to_string());
        fs::write(config_path, yaml_serde::to_string(&config).unwrap()).unwrap();
    }

    bin(tmp.path())
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    for name in ["sample", "additional"] {
        let content =
            fs::read_to_string(project.join(format!("dist/claude/{name}/SKILL.md"))).unwrap();
        let (frontmatter, body) = content
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap();
        let parsed: yaml_serde::Value = yaml_serde::from_str(frontmatter).unwrap();
        let config: yaml_serde::Value = yaml_serde::from_str(
            &fs::read_to_string(project.join(format!("skills/{name}/skill.yaml"))).unwrap(),
        )
        .unwrap();
        assert_eq!(parsed["name"].as_str(), Some(name));
        assert_eq!(parsed["description"].as_str(), Some(description));
        assert_eq!(parsed["description"], config["description"]);
        assert_eq!(body, format!("\n# {name}\n\n{description}\n"));
    }
}

#[test]
fn graphical_diagnostics_source_snippet_when_piped() {
    let tmp = copy_fixture("invalid-unknown-project-field");
    let assertion = bin(tmp.path())
        .current_dir(tmp.path())
        .env_remove("NO_COLOR")
        .env_remove("NO_GRAPHICS")
        .env("FORCE_COLOR", "1")
        .env("TERM", "xterm-256color")
        .env("LANG", "en_US.UTF-8")
        .arg("validate")
        .assert()
        .code(1);
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    let lines: Vec<_> = stderr.lines().collect();
    let source_line = lines
        .iter()
        .position(|line| line.contains("name: my-skills"))
        .unwrap_or_else(|| panic!("missing source snippet: {stderr}"));
    assert!(
        lines[source_line + 1..]
            .iter()
            .any(|line| line.contains("here")),
        "missing label beneath source: {stderr}"
    );
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
    assert!(stderr.is_ascii(), "{stderr}");
}

#[test]
fn quickstart_project_config_validates_with_sample_skill() {
    let tmp = TempDir::new().unwrap();
    let project = tmp.path().join("my-skills");
    bin(tmp.path())
        .current_dir(tmp.path())
        .args(["init", "project", "my-skills", "-H", "claude,opencode"])
        .assert()
        .success();
    let quickstart = include_str!("../site/content/docs/quickstart.md");
    let config = quickstart
        .split_once("```yaml\n")
        .expect("quickstart must contain the project config YAML block")
        .1
        .split_once("\n```")
        .expect("quickstart project config YAML block must have a closing fence")
        .0;
    fs::write(project.join("skillprism.yaml"), config).unwrap();
    let skill = project.join("skills/dice-roller");
    fs::create_dir_all(&skill).unwrap();
    let skill_config = quickstart
        .split("```yaml\n")
        .nth(2)
        .expect("quickstart must contain a second YAML block for the sample skill config")
        .split_once("\n```")
        .expect("quickstart sample skill config YAML block must have a closing fence")
        .0;
    fs::write(skill.join("skill.yaml"), skill_config).unwrap();
    // The template contains a nested Bash fence, so use its closing paragraph.
    let template = quickstart
        .split_once("```jinja\n")
        .expect("quickstart must contain the sample skill Jinja template block")
        .1
        .split_once("\n```\n\nThe YAML frontmatter")
        .expect("quickstart Jinja template must end before the YAML frontmatter paragraph")
        .0;
    fs::write(skill.join("SKILL.md"), template).unwrap();
    bin(tmp.path())
        .current_dir(&project)
        .arg("validate")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("ok: sample")
                .and(predicate::str::contains("ok: dice-roller"))
                .and(predicate::str::contains("Validation passed")),
        );
}

#[test]
fn config_diagnostics_unknown_project_field() {
    let tmp = copy_fixture("invalid-unknown-project-field");
    bin(tmp.path())
        .current_dir(tmp.path())
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unknown field `name`")
                .and(predicate::str::contains("skillprism.yaml"))
                .and(predicate::str::contains("line 1 column 1"))
                .and(predicate::str::contains("Invalid YAML").not()),
        );
}

#[test]
fn config_diagnostics_missing_skillprism_field() {
    let tmp = copy_fixture("invalid-missing-skillprism-field");
    bin(tmp.path())
        .current_dir(tmp.path())
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("missing required field `skillprism`")
                .and(predicate::str::contains("skills/demo/skill.yaml"))
                .and(predicate::str::contains("Invalid YAML").not())
                .and(predicate::str::contains("line 1").not()),
        );
}

#[test]
fn config_diagnostics_wrong_type() {
    let tmp = copy_fixture("valid");
    fs::write(
        tmp.path().join("skills/alpha/skill.yaml"),
        "skillprism: '1'\nname: alpha\nvariables: text\n",
    )
    .unwrap();
    bin(tmp.path())
        .current_dir(tmp.path())
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("variables")
                .and(predicate::str::contains("expected a map"))
                .and(predicate::str::contains("line 3 column 12"))
                .and(predicate::str::contains("Invalid YAML").not()),
        );
}

#[test]
fn config_diagnostics_malformed_yaml() {
    let tmp = copy_fixture("valid");
    fs::write(
        tmp.path().join("skillprism.yaml"),
        "# project\n# broken sequence\nharnesses: [claude",
    )
    .unwrap();
    let assertion = bin(tmp.path())
        .current_dir(tmp.path())
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("YAML does not parse")
                .and(predicate::str::contains("skillprism.yaml"))
                // yaml_serde reports EOF on line 4 and the sequence start on line 3.
                .and(predicate::str::contains("at line 4 column 1"))
                .and(predicate::str::contains(
                    "flow sequence at line 3 column 12",
                )),
        );
    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    assert_eq!(stderr.matches("at line 4 column 1").count(), 1, "{stderr}");
}

#[test]
fn config_diagnostics_other_config_paths() {
    let cases = [
        (
            "skillprism.yaml",
            "# config\nharnesses: claude\n",
            "harnesses: invalid type",
            "expected a sequence",
        ),
        (
            "skills/alpha/skill.yaml",
            "skillprism: '1'\n# config\nnam: alpha\n",
            "unknown field `nam`",
            "expected one of",
        ),
        (
            "skills/alpha/skill.yaml",
            "skillprism: '1'\n# config\nvariables: [text",
            "YAML does not parse",
            "line 3 column",
        ),
        (
            "harnesses/custom.yaml",
            "typo: custom\n",
            "unknown field `typo`",
            "expected one of",
        ),
        (
            "harnesses/custom.yaml",
            "name: Custom\n",
            "missing field `id`",
            "Invalid config",
        ),
        (
            "harnesses/custom.yaml",
            "id: custom\nname: Custom\ncapabilities: text\n",
            "capabilities: invalid type",
            "line 3 column 15",
        ),
        (
            "harnesses/custom.yaml",
            "id: custom\nname: Custom\ncapabilities: [text",
            "YAML does not parse",
            "line 3 column",
        ),
    ];
    for (file, content, reason, detail) in cases {
        let tmp = copy_fixture("valid");
        let path = tmp.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        let assertion = bin(tmp.path())
            .current_dir(tmp.path())
            .arg("validate")
            .assert()
            .failure()
            .stderr(
                predicate::str::contains(file)
                    .and(predicate::str::contains(reason))
                    .and(predicate::str::contains(detail))
                    .and(predicate::str::contains("Invalid YAML").not()),
            );
        if reason.starts_with("missing field") {
            let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
            assert!(!stderr.contains("column"), "{stderr}");
            assert!(!stderr.contains("here"), "{stderr}");
        }
    }
}

#[test]
fn config_diagnostics_help_matches_config_kind() {
    let cases = [
        ("skillprism.yaml", "typo: value\n", "quickstart"),
        (
            "skills/alpha/skill.yaml",
            "skillprism: '1'\ntypo: value\n",
            "skill-yaml",
        ),
        ("harnesses/custom.yaml", "typo: value\n", "harnesses"),
    ];
    for (file, content, docs) in cases {
        let tmp = copy_fixture("valid");
        let path = tmp.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        let assertion = bin(tmp.path())
            .current_dir(tmp.path())
            .env("NO_COLOR", "1")
            .arg("validate")
            .assert()
            .failure();
        let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
        for kind in ["quickstart", "skill-yaml", "harnesses"] {
            let url = format!("https://tuvren.github.io/skillprism/docs/{kind}/");
            assert_eq!(stderr.contains(&url), kind == docs, "{stderr}");
        }
    }
}

#[test]
fn config_diagnostics_scalar_errors_name_field_paths() {
    for (fields, expected) in [
        (
            "name: 42\n",
            "name: invalid type: integer `42`, expected a string",
        ),
        (
            "description: true\n",
            "description: invalid type: boolean `true`, expected a string",
        ),
        (
            "metadata: {owner: 42}\n",
            "metadata.owner: invalid type: integer `42`",
        ),
        (
            "arguments: [ok, 42]\n",
            "arguments[1]: invalid type: integer `42`",
        ),
        (
            "overrides: {claude: {macros: {hello: 42}}}\n",
            "overrides.claude.macros.hello: invalid type: integer `42`",
        ),
        (
            "overrides: {1: {}}\n",
            "overrides: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "metadata: {1: x}\n",
            "metadata: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "variables: {valid: 1, 1: x}\n",
            "variables: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "overrides: {claude: {variables: {valid: 1, 1: x}}}\n",
            "overrides.claude.variables: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "overrides: {claude: {macros: {1: x}}}\n",
            "overrides.claude.macros: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "overrides: {1: {macros: {hello: 1}}}\n",
            "overrides: key `1`: invalid type: integer `1`, expected a string",
        ),
        (
            "metadata: {!label owner: 42}\n",
            "metadata.owner: invalid type: integer `42`",
        ),
        (
            "overrides: {!label claude: {macros: {hello: 42}}}\n",
            "overrides.claude.macros.hello: invalid type: integer `42`",
        ),
        // A later direct-parse error must not replace the earlier Value rejection.
        (
            "name: 42\nvariables: text\n",
            "name: invalid type: integer `42`, expected a string",
        ),
    ] {
        let tmp = copy_fixture("valid");
        fs::write(
            tmp.path().join("skills/alpha/skill.yaml"),
            format!("skillprism: '1'\n{fields}"),
        )
        .unwrap();
        let assertion = bin(tmp.path())
            .current_dir(tmp.path())
            .env("NO_COLOR", "1")
            .arg("validate")
            .assert()
            .failure();
        let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
        // The graphical reporter wraps long paths and reasons across lines.
        let message = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(message.contains(expected), "{stderr}");
        assert!(!stderr.contains("column"), "{stderr}");
        assert!(!stderr.contains("here"), "{stderr}");
    }
}

#[test]
fn config_diagnostics_earlier_complex_key_has_no_version_label() {
    let tmp = copy_fixture("valid");
    fs::write(
        tmp.path().join("skills/alpha/skill.yaml"),
        "? [complex, key]\n: hello\nskillprism: '2'\n",
    )
    .unwrap();
    bin(tmp.path())
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("unsupported `skillprism:` value `2`")
                .and(predicate::str::contains("skill.yaml"))
                .and(predicate::str::contains("line").not())
                .and(predicate::str::contains("here").not()),
        );
}

#[test]
fn config_diagnostics_nested_missing_fields_have_no_label() {
    let tmp = copy_fixture("valid");
    fs::create_dir_all(tmp.path().join("harnesses")).unwrap();
    fs::write(
        tmp.path().join("harnesses/custom.yaml"),
        "id: custom\nname: Custom\ncapabilities: {}\n",
    )
    .unwrap();
    bin(tmp.path())
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .arg("validate")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("missing field `supports_subagent`")
                .and(predicate::str::contains("column").not())
                .and(predicate::str::contains("here").not()),
        );
}

#[test]
fn init_project_non_tty_without_harnesses_applies_default() {
    let tmp = TempDir::new().unwrap();
    let assertion = bin(tmp.path())
        .current_dir(tmp.path())
        .args(["init", "project", "demo"])
        .write_stdin("")
        .assert()
        .success();

    let config: yaml_serde::Value =
        yaml_serde::from_str(&fs::read_to_string(tmp.path().join("demo/skillprism.yaml")).unwrap())
            .unwrap();
    assert_eq!(
        config["harnesses"],
        yaml_serde::to_value(["claude", "opencode"]).unwrap()
    );

    let stderr = std::str::from_utf8(&assertion.get_output().stderr).unwrap();
    assert_eq!(stderr.lines().count(), 1);
    assert!(stderr.contains("default"));
    assert!(stderr.contains("claude, opencode"));
    assert!(stderr.contains("-H"));
}

#[test]
fn init_project_non_tty_explicit_harnesses_override_default() {
    let tmp = TempDir::new().unwrap();
    bin(tmp.path())
        .current_dir(tmp.path())
        .args(["init", "project", "demo", "-H", "codex"])
        .write_stdin("")
        .assert()
        .success()
        .stderr("");

    let config: yaml_serde::Value =
        yaml_serde::from_str(&fs::read_to_string(tmp.path().join("demo/skillprism.yaml")).unwrap())
            .unwrap();
    assert_eq!(
        config["harnesses"],
        yaml_serde::to_value(["codex"]).unwrap()
    );
}

#[test]
fn init_project_existing_project_fails_without_overwriting() {
    for harnesses in [None, Some("codex")] {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().join("demo");
        bin(tmp.path())
            .current_dir(tmp.path())
            .args(["init", "project", "demo", "-H", "claude"])
            .write_stdin("")
            .assert()
            .success();

        let config_path = project_dir.join("skillprism.yaml");
        let readme_path = project_dir.join("README.md");
        let mut config = fs::read(&config_path).unwrap();
        config.extend_from_slice(b"# Keep this project configuration.\n");
        fs::write(&config_path, &config).unwrap();
        let mut readme = fs::read(&readme_path).unwrap();
        readme.extend_from_slice(b"\nKeep this project documentation.\n");
        fs::write(&readme_path, &readme).unwrap();

        let mut command = bin(tmp.path());
        command
            .current_dir(tmp.path())
            .args(["init", "project", "demo", "--out"])
            .arg(&project_dir)
            .write_stdin("");
        if let Some(harnesses) = harnesses {
            command.args(["-H", harnesses]);
        }
        command.assert().code(2).stderr(
            predicate::str::contains("project already exists")
                .and(predicate::str::contains(project_dir.display().to_string())),
        );

        assert_eq!(fs::read(&config_path).unwrap(), config);
        assert_eq!(fs::read(&readme_path).unwrap(), readme);
    }
}

#[test]
fn init_project_non_tty_explicit_empty_harnesses_fails_with_usage_error() {
    for harnesses in ["", ",", " , "] {
        let tmp = TempDir::new().unwrap();
        bin(tmp.path())
            .current_dir(tmp.path())
            .args(["init", "project", "demo", "-H", harnesses])
            .write_stdin("")
            .assert()
            .code(2)
            .stderr(predicate::str::contains("No harnesses selected."));

        assert!(!tmp.path().join("demo").exists());
    }
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

    // 3 skills × 2 harnesses = 6 output files (alpha and beta are spot-checked below)
    for skill in &["alpha", "beta"] {
        for harness in &["claude", "opencode"] {
            let output_path = project_dir.join(format!("dist/{harness}/{skill}/SKILL.md"));
            assert!(
                output_path.exists(),
                "expected output at {}",
                output_path.display()
            );
        }
    }

    // Verify rendered content for alpha × claude
    let alpha_claude = fs::read_to_string(project_dir.join("dist/claude/alpha/SKILL.md")).unwrap();
    // The Agent Skills spec requires YAML frontmatter (name + description) — without
    // it no client can discover the skill. The fixture templates must emit it.
    assert!(
        alpha_claude.starts_with("---\n"),
        "rendered SKILL.md must start with YAML frontmatter, got: {}",
        &alpha_claude[..alpha_claude.len().min(80)]
    );
    assert!(alpha_claude.contains("name: \"alpha\""));
    assert!(alpha_claude.contains("description: \"First test skill\""));
    assert!(alpha_claude.contains("# alpha"));
    assert!(alpha_claude.contains("Hello from Alpha"));
    assert!(alpha_claude.contains("Theme: dark"));
    assert!(alpha_claude.contains("Harness: claude (Claude Code)"));

    // Verify rendered content for beta × opencode
    let beta_opencode =
        fs::read_to_string(project_dir.join("dist/opencode/beta/SKILL.md")).unwrap();
    assert!(
        beta_opencode.starts_with("---\n"),
        "rendered SKILL.md must start with YAML frontmatter"
    );
    assert!(beta_opencode.contains("name: \"beta\""));
    assert!(beta_opencode.contains("description: \"Second test skill\""));
    assert!(beta_opencode.contains("# beta"));
    assert!(beta_opencode.contains("Hello from Beta"));
    assert!(beta_opencode.contains("Message:"));

    // Verify manifest files exist for claude (has plugin.json) with correct content
    let manifest_path = project_dir.join("dist/claude/.claude/plugin.json");
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
fn skill_ref_helper_validates_and_renders_with_harness_pattern() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    // User harness overrides replace the full definition; isolate this one in the copy.
    fs::create_dir_all(project_dir.join("harnesses")).unwrap();
    fs::write(
        project_dir.join("harnesses/opencode.yaml"),
        r#"
id: opencode
name: OpenCode Custom
capabilities:
  supports_subagent: false
paths:
  project_scope_path: .opencode/skills
  user_scope_path: .config/opencode/skills
  skill_filename: SKILL.md
skill_ref_pattern: "@{name}"
"#,
    )
    .unwrap();

    // `gamma`'s template calls `{{ skill_ref("other") }}` — validate must accept the
    // registered helper rather than reporting it as an undefined variable.
    bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("validate")
        .assert()
        .success();

    // Build uses both Claude's default pattern and the project override's custom pattern.
    bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--force")
        .assert()
        .success();

    let gamma_claude = fs::read_to_string(project_dir.join("dist/claude/gamma/SKILL.md")).unwrap();
    assert!(
        gamma_claude.contains("Ref: /other"),
        "rendered skill_ref should use the harness pattern, got: {gamma_claude}"
    );
    let gamma_opencode =
        fs::read_to_string(project_dir.join("dist/opencode/gamma/SKILL.md")).unwrap();
    assert!(
        gamma_opencode.contains("Ref: @other"),
        "rendered skill_ref should use the user harness pattern, got: {gamma_opencode}"
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
        !project_dir.join("dist").exists(),
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
            let output_path = project_dir.join(format!("dist/{harness}/{skill}/SKILL.md"));
            assert!(output_path.exists(), "file should still exist after --diff");
        }
    }
}

#[test]
fn build_harness_flag_filters_and_deduplicates() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    // Pass --harness claude,claude (duplicate) — should build claude, skip opencode, and not crash on collision
    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--harness")
        .arg("claude,claude")
        .arg("--force")
        .assert();
    assert.success();

    assert!(project_dir.join("dist/claude/alpha/SKILL.md").exists());
    assert!(project_dir.join("dist/claude/beta/SKILL.md").exists());
    assert!(!project_dir.join("dist/opencode").exists());
}

#[test]
fn build_harness_unknown_harness_fails() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--harness")
        .arg("nonexistent_harness")
        .assert();
    assert.failure().stderr(predicate::str::contains(
        "Unknown harness: nonexistent_harness",
    ));
}

#[test]
fn build_harness_mixed_valid_invalid_fails_without_writing() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--harness")
        .arg("claude,invalid_harness")
        .assert();
    assert
        .failure()
        .stderr(predicate::str::contains("Unknown harness: invalid_harness"));

    assert!(
        !project_dir.join("dist").exists(),
        "build must not write output files if any harness ID is invalid"
    );
}

#[test]
fn build_harness_trims_whitespace() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--harness")
        .arg(" claude , opencode ")
        .arg("--force")
        .assert();
    assert.success();

    assert!(project_dir.join("dist/claude/alpha/SKILL.md").exists());
    assert!(project_dir.join("dist/opencode/alpha/SKILL.md").exists());
}

#[test]
fn build_target_flag_rejected() {
    let tmp = copy_fixture("valid");
    let project_dir = tmp.path().to_path_buf();
    let home_tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(home_tmp.path())
        .current_dir(&project_dir)
        .arg("build")
        .arg("--target")
        .arg("project")
        .assert();
    assert.failure();
}

#[test]
fn validate_nonexistent_path_fails() {
    let tmp = TempDir::with_prefix("skillprism_home_").unwrap();

    let assert = bin(tmp.path())
        .arg("validate")
        .arg("nonexistent_directory_xyz")
        .assert();
    assert
        .failure()
        .stderr(predicate::str::contains("does not exist"));
}

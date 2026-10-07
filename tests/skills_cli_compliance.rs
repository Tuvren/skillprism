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

//! Compliance with the pinned Vercel `skills` CLI contract.
//!
//! The oracle is `tests/contracts/skills-cli-v1.7.1.json`, taken from
//! `vercel-labs/skills` v1.7.1 (`src/agents.ts` and `src/cli.ts`). It covers
//! install directories for the five built-in harnesses and the shared
//! `add` / `list` / `remove` / `update` flags. It does not require `find`,
//! `use`, lockfile interop, or a symlink canonical store.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use serde::Deserialize;
use tempfile::TempDir;

const CONTRACT: &str = include_str!("contracts/skills-cli-v1.7.1.json");
const PLAIN_SKILL: &str = "plain-skill";

#[derive(Debug, Deserialize)]
struct Contract {
    harnesses: Vec<HarnessContract>,
    add_flags: Vec<String>,
    list_flags: Vec<String>,
    remove_flags: Vec<String>,
    update_flags: Vec<String>,
    out_of_scope: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HarnessContract {
    skillprism_id: String,
    skills_agent: String,
    aliases: Vec<String>,
    project_scope_path: String,
    user_scope_path: String,
    user_scope_base: String,
}

fn contract() -> Contract {
    serde_json::from_str(CONTRACT).expect("pinned skills CLI contract")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn fixture_source() -> PathBuf {
    project_root().join("tests/fixtures/dist-simple")
}

struct Env {
    project: TempDir,
    home: TempDir,
}

impl Env {
    fn new() -> Self {
        let project = TempDir::with_prefix("skillprism_skills_cli_").unwrap();
        fs::write(
            project.path().join("skillprism.yaml"),
            "harnesses:\n  - claude\nskills_dir: skills\n",
        )
        .unwrap();
        let home = TempDir::with_prefix("skillprism_skills_cli_home_").unwrap();
        Self { project, home }
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("skillprism").unwrap();
        cmd.current_dir(self.project.path())
            .env("HOME", self.home.path())
            .env("XDG_CONFIG_HOME", self.home.path().join("xdg"));
        cmd
    }
}

fn help(command: &str) -> String {
    let output = Command::cargo_bin("skillprism")
        .unwrap()
        .arg(command)
        .arg("--help")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{command} --help failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn help_has_flag(help_text: &str, flag: &str) -> bool {
    help_text
        .split_whitespace()
        .any(|token| token.trim_matches(|c: char| matches!(c, ',' | '[' | ']')) == flag)
}

fn missing_flags(help_text: &str, flags: &[String]) -> Vec<String> {
    flags
        .iter()
        .filter(|flag| !help_has_flag(help_text, flag))
        .cloned()
        .collect()
}

fn scope_paths(yaml: &str) -> (String, String) {
    let mut in_paths = false;
    let mut project = None;
    let mut user = None;
    for line in yaml.lines() {
        if line.starts_with("paths:") {
            in_paths = true;
            continue;
        }
        if in_paths && !line.starts_with([' ', '\t']) && !line.is_empty() {
            break;
        }
        if !in_paths {
            continue;
        }
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("project_scope_path:") {
            project = Some(value.trim().to_string());
        }
        if let Some(value) = trimmed.strip_prefix("user_scope_path:") {
            user = Some(value.trim().to_string());
        }
    }
    (
        project.expect("project_scope_path"),
        user.expect("user_scope_path"),
    )
}

fn output_text(output: &Output) -> String {
    format!(
        "status: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn contract_keeps_full_cli_parity_out_of_scope() {
    let contract = contract();
    for item in [
        "find and the skills.sh search API",
        "use, including launching another agent",
        "experimental_install and experimental_sync",
        "skills-lock.json as a shared lockfile",
    ] {
        assert!(
            contract.out_of_scope.iter().any(|entry| entry == item),
            "contract dropped the boundary for {item}"
        );
    }
}

#[test]
fn builtin_harness_directories_match_skills_cli() {
    let contract = contract();
    let mut mismatches = Vec::new();
    for harness in &contract.harnesses {
        let path = project_root()
            .join("src/builtin_harnesses")
            .join(format!("{}.yaml", harness.skillprism_id));
        let yaml = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("read {}: {error}", path.display());
        });
        let (project, user) = scope_paths(&yaml);
        if project != harness.project_scope_path {
            mismatches.push(format!(
                "{} project path is `{project}`, skills CLI `{}` uses `{}`",
                harness.skillprism_id, harness.skills_agent, harness.project_scope_path
            ));
        }
        if user != harness.user_scope_path {
            mismatches.push(format!(
                "{} user path is `{user}`, skills CLI `{}` uses `{}` (base: {})",
                harness.skillprism_id,
                harness.skills_agent,
                harness.user_scope_path,
                harness.user_scope_base
            ));
        }
        if harness.skills_agent != harness.skillprism_id
            && !harness.aliases.contains(&harness.skills_agent)
        {
            mismatches.push(format!(
                "{} must accept skills CLI agent id `{}` as an alias",
                harness.skillprism_id, harness.skills_agent
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "built-in harness directories drifted from skills CLI v1.7.1:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn add_help_accepts_shared_lifecycle_flags() {
    let missing = missing_flags(&help("add"), &contract().add_flags);
    assert!(
        missing.is_empty(),
        "skillprism add is missing skills CLI flags: {}",
        missing.join(", ")
    );
}

#[test]
fn list_help_accepts_shared_lifecycle_flags() {
    let missing = missing_flags(&help("list"), &contract().list_flags);
    assert!(
        missing.is_empty(),
        "skillprism list is missing skills CLI flags: {}",
        missing.join(", ")
    );
}

#[test]
fn remove_help_accepts_shared_lifecycle_flags() {
    let missing = missing_flags(&help("remove"), &contract().remove_flags);
    assert!(
        missing.is_empty(),
        "skillprism remove is missing skills CLI flags: {}",
        missing.join(", ")
    );
}

#[test]
fn update_help_accepts_shared_lifecycle_flags() {
    let missing = missing_flags(&help("update"), &contract().update_flags);
    assert!(
        missing.is_empty(),
        "skillprism update is missing skills CLI flags: {}",
        missing.join(", ")
    );
}

#[test]
fn plain_skill_bytes_land_on_the_claude_project_path() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--force", "-H", "claude", "--skill", PLAIN_SKILL])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));

    let installed = env
        .project
        .path()
        .join(".claude/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    let expected = fixture_source()
        .join("skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert_eq!(
        fs::read(&installed).unwrap_or_else(|error| panic!("{}: {error}", installed.display())),
        fs::read(&expected).unwrap(),
        "plain SKILL.md must be copied unchanged"
    );
}

#[test]
fn codex_project_install_uses_agents_skills() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--force", "-H", "codex", "--skill", PLAIN_SKILL])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let installed = env
        .project
        .path()
        .join(".agents/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(installed.is_file(), "missing {}", installed.display());
}

#[test]
fn opencode_project_install_uses_agents_skills() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--force", "-H", "opencode", "--skill", PLAIN_SKILL])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let installed = env
        .project
        .path()
        .join(".agents/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    let legacy = env
        .project
        .path()
        .join(".opencode/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(
        installed.is_file(),
        "OpenCode project skills belong at {}, not {}",
        installed.display(),
        legacy.display()
    );
}

#[test]
fn opencode_user_install_follows_xdg_config_home() {
    let env = Env::new();
    let output = env
        .cmd()
        .args([
            "add",
            "--force",
            "--target",
            "user",
            "-H",
            "opencode",
            "--skill",
            PLAIN_SKILL,
        ])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let installed = env
        .home
        .path()
        .join("xdg/opencode/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(
        installed.is_file(),
        "OpenCode global skills belong at {} (XDG_CONFIG_HOME/opencode/skills)",
        installed.display()
    );
}

#[test]
fn factory_project_install_uses_the_droid_directory() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--force", "-H", "droid", "--skill", PLAIN_SKILL])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let installed = env
        .project
        .path()
        .join(".agents/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(
        installed.is_file(),
        "Factory's skills CLI id is `droid` and its project directory is {}",
        installed.display()
    );
}

#[test]
fn pi_install_uses_universal_agents_skills() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--force", "-H", "pi", "--skill", PLAIN_SKILL])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let project = env
        .project
        .path()
        .join(".agents/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(project.is_file(), "missing {}", project.display());

    let global = env
        .cmd()
        .args([
            "add",
            "--force",
            "--target",
            "user",
            "-H",
            "pi",
            "--skill",
            PLAIN_SKILL,
        ])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(global.status.success(), "{}", output_text(&global));
    let user = env
        .home
        .path()
        .join(".agents/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(user.is_file(), "missing {}", user.display());
}

#[test]
fn global_and_agent_flags_install_claude_code() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "-g", "-a", "claude-code", "-s", PLAIN_SKILL, "-y"])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let installed = env
        .home
        .path()
        .join(".claude/skills")
        .join(PLAIN_SKILL)
        .join("SKILL.md");
    assert!(installed.is_file(), "missing {}", installed.display());
}

#[test]
fn add_list_prints_skills_without_writing_them() {
    let env = Env::new();
    let output = env
        .cmd()
        .args(["add", "--list"])
        .arg(fixture_source())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", output_text(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(PLAIN_SKILL),
        "expected the skill name on stdout, got: {stdout}"
    );
    assert!(
        !env.project.path().join(".claude").exists(),
        "add --list must not install files"
    );
}

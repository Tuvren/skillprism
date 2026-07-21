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

//! Builds the real `examples/` project end-to-end and asserts on the rendered output.
//! Two skills (`mcp-builder`, `webapp-testing`) are ported from Anthropic's public
//! Agent Skills repository; doubles as a regression suite for three gaps between
//! skillprism's documented schema and its implementation that surfaced while authoring
//! them, and that were fixed in the same change that added this file — see
//! `examples/README.md` "Issues found and fixed while porting these skills" for the
//! full writeup with file:line citations.
//!
//! `mcp-builder` deliberately requires `allowed-tools`, a capability only `claude`
//! supports among this project's three targeted harnesses — so it resolves for
//! `claude` only, and is skipped (not a build failure) for `opencode`/`codex`. This is
//! the regression test for the fix to that exact behavior.
//!
//! The third skill, `quickstart`, is deliberately synthetic rather than ported from
//! anywhere real — it exists purely to exercise skillprism's per-harness override
//! mechanism (`skill.yaml`'s `harnesses:` block), which neither ported skill needs and
//! so leaves otherwise untested by this file.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

const SKILLS: [&str; 3] = ["mcp-builder", "webapp-testing", "quickstart"];
const HARNESSES: [(&str, &str); 3] = [
    ("claude", ".claude/skills"),
    ("opencode", ".opencode/skills"),
    ("codex", ".agents/skills"), // project scope — NOT .codex/skills (that's user-scope only)
];

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn examples_src_dir() -> PathBuf {
    project_root().join("examples")
}

fn copy_examples() -> TempDir {
    let tmp = TempDir::with_prefix("skillprism_examples_").unwrap();
    cp_dir(&examples_src_dir(), tmp.path()).unwrap();
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
    cmd.env("HOME", home.join(".home"))
        .env("XDG_CONFIG_HOME", home.join(".config"));
    cmd
}

fn skill_output_path(project_dir: &Path, scope_dir: &str, skill: &str) -> PathBuf {
    project_dir.join(scope_dir).join(skill).join("SKILL.md")
}

/// Builds the copied `examples/` project and returns the tempdir plus captured stderr
/// (where `[resolve] skipped: ...` warnings land).
fn build_examples() -> (TempDir, String) {
    let tmp = copy_examples();
    let assert = bin(tmp.path())
        .current_dir(tmp.path())
        .arg("build")
        .arg("--force")
        .assert();
    let assert = assert.success();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
    (tmp, stderr)
}

#[test]
fn examples_build_succeeds_and_skips_incompatible_pairs() {
    let (tmp, stderr) = build_examples();
    let project_dir = tmp.path();

    // webapp-testing has no required-capabilities — resolves for all three harnesses.
    for (_harness, scope_dir) in &HARNESSES {
        let output_path = skill_output_path(project_dir, scope_dir, "webapp-testing");
        assert!(
            output_path.exists(),
            "expected rendered SKILL.md at {}",
            output_path.display()
        );
    }

    // mcp-builder requires `allowed-tools`, which only claude supports among the three
    // targeted harnesses — it should resolve there and be skipped (not abort the
    // build) for opencode/codex.
    assert!(skill_output_path(project_dir, ".claude/skills", "mcp-builder").exists());
    assert!(!skill_output_path(project_dir, ".opencode/skills", "mcp-builder").exists());
    assert!(!skill_output_path(project_dir, ".agents/skills", "mcp-builder").exists());

    assert!(
        stderr.contains("[resolve] skipped")
            && stderr.contains("mcp-builder")
            && stderr.contains("allowed-tools"),
        "expected a skip warning for mcp-builder's allowed-tools requirement, got stderr: {stderr}"
    );
    // Both skipped pairs (opencode and codex) should be reported, not just one.
    assert_eq!(
        stderr.matches("[resolve] skipped").count(),
        2,
        "expected exactly 2 skip warnings (mcp-builder x opencode, mcp-builder x codex), got: {stderr}"
    );
}

#[test]
fn examples_subagent_guide_varies_by_harness() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    // Neither template hand-writes per-harness branching — the only thing that varies
    // cross-harness in these skills is `{{ harness.subagent_guide }}`, a builtin macro
    // every harness defines for itself (src/builtin_harnesses/*.yaml). webapp-testing
    // resolves everywhere, so its rendered output can be checked against all three.
    let webapp_expectations = [
        (".claude/skills", "Claude Code mechanisms"),
        (".opencode/skills", "composable subagents"),
        (".agents/skills", "separate agent orchestration context"),
    ];
    for (scope_dir, subagent_phrase) in webapp_expectations {
        let content =
            fs::read_to_string(skill_output_path(project_dir, scope_dir, "webapp-testing"))
                .unwrap();
        assert!(
            content.contains("## Subagent Instructions") && content.contains(subagent_phrase),
            "{scope_dir}/webapp-testing SKILL.md missing subagent_guide phrase {subagent_phrase:?}"
        );
    }

    // mcp-builder only resolves for claude in this project (see
    // examples_build_succeeds_and_skips_incompatible_pairs) — its rendered output must
    // carry claude's subagent_guide text, not opencode's or codex's.
    let claude_mcp = fs::read_to_string(skill_output_path(
        project_dir,
        ".claude/skills",
        "mcp-builder",
    ))
    .unwrap();
    assert!(
        claude_mcp.contains("## Subagent Instructions")
            && claude_mcp.contains("Claude Code mechanisms")
    );
    assert!(!claude_mcp.contains("composable subagents"));
    assert!(!claude_mcp.contains("separate agent orchestration context"));
}

#[test]
fn examples_asset_copy_matches_skillprism_convention() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    let webapp_examples_files = [
        "element_discovery.py",
        "static_html_automation.py",
        "console_logging.py",
    ];

    // webapp-testing's asset directory keeps its real upstream name, `examples/` —
    // not `references/` — proving asset-directory discovery is no longer hardcoded to
    // a fixed pair of names (Finding #3 fix, src/loader/project.rs::load_skill).
    for (_harness, scope_dir) in &HARNESSES {
        let webapp_dir = project_dir.join(scope_dir).join("webapp-testing");

        for filename in &webapp_examples_files {
            let copied = webapp_dir.join("examples").join(filename);
            let source = examples_src_dir()
                .join("skills/webapp-testing/examples")
                .join(filename);
            assert!(
                copied.exists(),
                "expected {} to be copied",
                copied.display()
            );
            assert_eq!(
                fs::read(&copied).unwrap(),
                fs::read(&source).unwrap(),
                "{filename} should be byte-identical to source"
            );
        }

        let copied_script = webapp_dir.join("scripts/with_server.py");
        let source_script = examples_src_dir().join("skills/webapp-testing/scripts/with_server.py");
        assert!(copied_script.exists());
        assert_eq!(
            fs::read(&copied_script).unwrap(),
            fs::read(&source_script).unwrap()
        );
    }

    // mcp-builder only resolves for claude — check its asset copy there. Its real
    // asset directory also keeps its upstream name, `reference/` (singular), which
    // previously was silently dropped (the original bug this fix addresses).
    let mcp_dir = project_dir.join(".claude/skills/mcp-builder");

    let copied_reqs = mcp_dir.join("scripts/requirements.txt");
    let source_reqs = examples_src_dir().join("skills/mcp-builder/scripts/requirements.txt");
    assert!(copied_reqs.exists(), "requirements.txt should be copied");
    assert_eq!(
        fs::read(&copied_reqs).unwrap(),
        fs::read(&source_reqs).unwrap()
    );

    let copied_ref = mcp_dir.join("reference/mcp_best_practices.md");
    let source_ref = examples_src_dir().join("skills/mcp-builder/reference/mcp_best_practices.md");
    assert!(
        copied_ref.exists(),
        "reference/ (singular) should now be copied like any other asset directory — \
         it's no longer limited to the hardcoded `references/`/`scripts/` pair"
    );
    assert_eq!(
        fs::read(&copied_ref).unwrap(),
        fs::read(&source_ref).unwrap()
    );
}

#[test]
fn examples_manifests_reflect_resolved_pairs_only() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    // claude resolves both skills — its manifest references both.
    let claude_manifest = project_dir.join(".claude/plugin.json");
    assert!(claude_manifest.exists());
    let claude_content = fs::read_to_string(&claude_manifest).unwrap();
    assert!(claude_content.trim_start().starts_with('['));
    assert!(claude_content.trim_end().ends_with(']'));
    for skill in &SKILLS {
        assert!(claude_content.contains(skill));
    }

    // codex only resolves webapp-testing (mcp-builder was skipped there) — its
    // manifest must reference webapp-testing and must NOT reference mcp-builder.
    let codex_manifest = project_dir.join(".agents/marketplace.json");
    assert!(codex_manifest.exists());
    let codex_content = fs::read_to_string(&codex_manifest).unwrap();
    assert!(codex_content.contains("webapp-testing"));
    assert!(codex_content.contains("quickstart"));
    assert!(
        !codex_content.contains("mcp-builder"),
        "codex manifest should not reference the skipped mcp-builder skill, got: {codex_content}"
    );

    // opencode has no manifest defined (requires_manifest: false) — must not be written.
    assert!(!project_dir.join(".opencode/plugin.json").exists());
}

#[test]
fn examples_skill_metadata_fields_render_correctly() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    // Finding #1 fix: documented skill.yaml metadata fields now reach the Jinja render
    // context (src/engine/context.rs::build_context) instead of silently disappearing.
    // These two skills only echo the fields a real skill body would actually use
    // (license/allowed-tools in frontmatter, when_to_use as a real section) — fields
    // like `version` and `metadata.*` are packaging/attribution metadata that reach the
    // context too (see context::tests::context_includes_every_skill_metadata_field) but
    // aren't meant to be echoed into every rendered body just to prove it.
    let claude_mcp = fs::read_to_string(skill_output_path(
        project_dir,
        ".claude/skills",
        "mcp-builder",
    ))
    .unwrap();
    assert!(claude_mcp.contains("license: Apache-2.0"));
    assert!(claude_mcp.contains("allowed-tools: Read, Write, Bash, WebFetch, WebSearch"));
    assert!(claude_mcp.contains(r#"Trigger phrases: "build an MCP server for X""#));

    for (_harness, scope_dir) in &HARNESSES {
        let webapp_content =
            fs::read_to_string(skill_output_path(project_dir, scope_dir, "webapp-testing"))
                .unwrap();
        assert!(webapp_content.contains("license: Apache-2.0"));
        assert!(webapp_content.contains(r#"Trigger phrases: "test this web app""#));
    }
}

#[test]
fn examples_quickstart_demonstrates_harness_variable_override() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    // skill.yaml's top-level `variables.greeting` default applies to claude and codex;
    // `harnesses.opencode.variables.greeting` overrides it for opencode only.
    let claude = fs::read_to_string(skill_output_path(
        project_dir,
        ".claude/skills",
        "quickstart",
    ))
    .unwrap();
    let codex = fs::read_to_string(skill_output_path(
        project_dir,
        ".agents/skills",
        "quickstart",
    ))
    .unwrap();
    let opencode = fs::read_to_string(skill_output_path(
        project_dir,
        ".opencode/skills",
        "quickstart",
    ))
    .unwrap();

    assert!(claude.contains("Current value: **Hello from skillprism**"));
    assert!(codex.contains("Current value: **Hello from skillprism**"));
    assert!(
        opencode.contains(
            "Current value: **Hello from skillprism, rendered specifically for OpenCode**"
        )
    );
}

#[test]
fn examples_quickstart_demonstrates_harness_macro_override() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    // `harnesses.codex.macros.subagent_guide` in quickstart's skill.yaml overrides
    // Codex's own builtin `subagent_guide` macro for this skill only — claude and
    // opencode still render their harness's unmodified builtin text (same text
    // `examples_subagent_guide_varies_by_harness` checks for webapp-testing/mcp-builder).
    let claude = fs::read_to_string(skill_output_path(
        project_dir,
        ".claude/skills",
        "quickstart",
    ))
    .unwrap();
    let opencode = fs::read_to_string(skill_output_path(
        project_dir,
        ".opencode/skills",
        "quickstart",
    ))
    .unwrap();
    let codex = fs::read_to_string(skill_output_path(
        project_dir,
        ".agents/skills",
        "quickstart",
    ))
    .unwrap();

    assert!(claude.contains("Claude Code mechanisms"));
    assert!(opencode.contains("composable subagents"));
    assert!(
        codex.contains("This skill overrides Codex's own subagent_guide macro for itself only")
    );
    assert!(
        !codex.contains("separate agent orchestration context"),
        "codex's quickstart output should show the per-skill override, not Codex's unmodified builtin subagent_guide text"
    );
}

#[test]
fn examples_quickstart_assets_copied_to_every_harness() {
    let (tmp, _stderr) = build_examples();
    let project_dir = tmp.path();

    let source = examples_src_dir().join("skills/quickstart/references/note.md");
    for (_harness, scope_dir) in &HARNESSES {
        let copied = project_dir
            .join(scope_dir)
            .join("quickstart/references/note.md");
        assert!(
            copied.exists(),
            "expected {} to be copied",
            copied.display()
        );
        assert_eq!(fs::read(&copied).unwrap(), fs::read(&source).unwrap());
    }
}

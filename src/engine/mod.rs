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

mod context;
mod frontmatter;
mod helpers;

use std::collections::BTreeMap;
use std::fs;

use miette::Diagnostic;
use thiserror::Error;

use crate::registry::ManifestDef;
use crate::resolver::ResolvedPair;

pub use context::build_context;
pub use frontmatter::FrontmatterError;
pub use helpers::register_helpers;

/// Output produced by rendering a skill template through a harness.
#[derive(Debug, Clone)]
pub struct HarnessOutput {
    /// The rendered content of the skill's main template file.
    pub skill_content: String,
    /// Sidecar files produced alongside the main skill file.
    pub sidecars: Vec<SidecarOutput>,
}

/// A sidecar file produced during skill rendering.
#[derive(Debug, Clone)]
pub struct SidecarOutput {
    /// Filename of the sidecar (e.g. "config.yaml").
    pub filename: String,
    /// Rendered content of the sidecar.
    pub content: String,
    /// Optional subdirectory within the skill output directory.
    pub output_dir: Option<String>,
}

/// Errors that occur during template rendering in the engine.
#[derive(Debug, Diagnostic, Error)]
pub enum EngineError {
    /// Failed to read a template file from disk.
    #[error("[{skill}] {harness}: Failed to read template `{path}`")]
    #[diagnostic(help("{detail}"))]
    TemplateRead {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },

    /// The template failed to render (syntax error or missing variable).
    #[error("[{skill}] {harness}: {detail}")]
    #[diagnostic(help("Check the template syntax and its referenced variables"))]
    RenderError {
        skill: String,
        harness: String,
        template: String,
        line: Option<usize>,
        detail: String,
    },

    /// The rendered skill has invalid YAML frontmatter.
    #[error(transparent)]
    #[diagnostic(transparent)]
    Frontmatter(Box<FrontmatterError>),
}

/// The rendering engine that processes skill templates through harnesses.
pub struct Engine;

impl Engine {
    /// Renders a skill template using the resolved harness, producing output files.
    pub fn render(pair: &ResolvedPair) -> Result<HarnessOutput, EngineError> {
        let content = fs::read_to_string(&pair.skill.template_path).map_err(|e| {
            EngineError::TemplateRead {
                skill: pair.skill.name.clone(),
                harness: pair.harness.id.clone(),
                path: pair.skill.template_path.to_string_lossy().to_string(),
                detail: e.to_string(),
            }
        })?;

        let ctx = build_context(pair);

        let mut env = minijinja::Environment::new();
        register_helpers(&mut env, pair.harness.skill_ref_pattern.as_deref());

        let name = pair.skill.template_path.to_string_lossy();
        env.add_template_owned(name.to_string(), content)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

        let tmpl = env
            .get_template(&name)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

        let skill_content = tmpl
            .render(&ctx)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

        frontmatter::check(pair, &skill_content).map_err(EngineError::Frontmatter)?;

        let sidecars = render_sidecars(pair, &ctx).map_err(|e| EngineError::RenderError {
            skill: pair.skill.name.clone(),
            harness: pair.harness.id.clone(),
            template: "(sidecar)".to_string(),
            line: None,
            detail: format!("Sidecar rendering failed: {e}"),
        })?;

        Ok(HarnessOutput {
            skill_content,
            sidecars,
        })
    }

    /// Renders a single manifest entry for a resolved skill-harness pair.
    ///
    /// Returns `Ok(None)` if the harness does not define a manifest template.
    /// Returns `Err(EngineError::RenderError)` if the manifest template is
    /// invalid or fails to render.
    pub fn render_manifest_entry(pair: &ResolvedPair) -> Result<Option<String>, EngineError> {
        let Some(manifest) = pair.harness.manifest.as_ref() else {
            return Ok(None);
        };
        let ctx = build_context(pair);
        render_manifest(manifest, &ctx, pair.harness.skill_ref_pattern.as_deref())
            .map(Some)
            .map_err(|e| render_error_from_minijinja(pair, "(manifest)", &e))
    }
}

fn render_error_from_minijinja(
    pair: &ResolvedPair,
    template_name: &str,
    err: &minijinja::Error,
) -> EngineError {
    let detail = fmt_minijinja_error(err);
    EngineError::RenderError {
        skill: pair.skill.name.clone(),
        harness: pair.harness.id.clone(),
        template: template_name.to_string(),
        line: err.line(),
        detail,
    }
}

fn fmt_minijinja_error(err: &minijinja::Error) -> String {
    let kind = format!("{}", err.kind());
    if let Some(line) = err.line() {
        format!("{kind} at line {line}")
    } else {
        kind
    }
}

fn render_sidecars(
    pair: &ResolvedPair,
    ctx: &BTreeMap<String, minijinja::Value>,
) -> Result<Vec<SidecarOutput>, String> {
    let mut sidecars = Vec::new();

    for def in &pair.harness.sidecars {
        let mut env = minijinja::Environment::new();
        register_helpers(&mut env, pair.harness.skill_ref_pattern.as_deref());
        env.add_template_owned(def.filename.clone(), def.template.clone())
            .map_err(|e| format!("{}: {e}", def.filename))?;

        let tmpl = env
            .get_template(&def.filename)
            .map_err(|e| format!("{}: {e}", def.filename))?;

        let content = tmpl
            .render(ctx)
            .map_err(|e| format!("{}: {e}", def.filename))?;

        sidecars.push(SidecarOutput {
            filename: def.filename.clone(),
            content,
            output_dir: def.output_dir.clone(),
        });
    }

    Ok(sidecars)
}

fn render_manifest(
    manifest: &ManifestDef,
    ctx: &BTreeMap<String, minijinja::Value>,
    skill_ref_pattern: Option<&str>,
) -> Result<String, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    register_helpers(&mut env, skill_ref_pattern);
    env.add_template_owned("manifest_tmpl", manifest.template.clone())?;
    let tmpl = env.get_template("manifest_tmpl")?;
    tmpl.render(ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;
    use crate::resolver::HarnessResolver;
    use crate::resolver::tests::test_skill;
    use crate::types::SkillModel;
    use std::path::Path;

    fn create_skill_with_template(
        name: &str,
        template_content: &str,
        vars: BTreeMap<String, yaml_serde::Value>,
    ) -> (tempfile::TempDir, SkillModel) {
        let dir = tempfile::tempdir().unwrap();
        let tmpl_path = dir.path().join("SKILL.md.j2");
        std::fs::write(&tmpl_path, template_content).unwrap();

        let mut skill = test_skill(name, vec![]);
        skill.template_path = tmpl_path;
        skill.variables = vars;
        (dir, skill)
    }

    fn render_pair(
        skill_name: &str,
        template: &str,
        harness_name: &str,
    ) -> Result<HarnessOutput, EngineError> {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template(skill_name, template, BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, harness_name, &registry).unwrap();
        Engine::render(&pair)
    }

    #[test]
    fn renders_skill_name_in_template() {
        let output = render_pair("my-agent", "{{ skill_name }}", "claude").unwrap();
        assert_eq!(output.skill_content, "my-agent");
    }

    #[test]
    fn yaml_str_descriptions_round_trip_through_frontmatter() {
        let values = [
            "Router: use mode X. See also: reference docs. Say \"hello\" and use the gh CLI.",
            "it's a 'quoted' value",
            "back\\slash and a # hash",
            "- starts with a dash",
            "line one\nline two",
            "\tcontrol\0\u{7}\u{8}\u{b}\u{c}\r\u{1b}\u{1f}\u{7f}\u{80}\u{85}\u{9f}",
            "unicode\u{2028}line\u{2029}paragraph\u{fffe}\u{ffff}",
            "Yáñez 日本語 🦀",
            "",
            "  leading and trailing whitespace  ",
            "true",
            "null",
            "123",
            "# comment",
            "!tag",
            "&anchor",
            "*alias",
            "[sequence]",
            "{mapping: value}",
            "---\nname: injected\n...",
        ];
        let registry = HarnessRegistry::with_builtins();
        for value in values {
            let (_dir, mut skill) = create_skill_with_template(
                "yaml-scalar",
                "---\nname: {{ skill_name }}\ndescription: {{ skill_description | yaml_str }}\n---\nBody\n",
                BTreeMap::new(),
            );
            skill.description = value.to_string();
            let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
            let output = Engine::render(&pair).unwrap();
            let frontmatter = output
                .skill_content
                .strip_prefix("---\n")
                .unwrap()
                .split_once("\n---\n")
                .unwrap()
                .0;
            assert!(frontmatter.contains("description: \""), "{value:?}");
            let parsed: yaml_serde::Value = yaml_serde::from_str(frontmatter).unwrap();
            assert_eq!(parsed["description"].as_str(), Some(value), "{value:?}");
        }
    }

    #[test]
    fn yaml_str_stringifies_non_string_scalars() {
        let cases = [
            (yaml_serde::Value::Bool(true), "true"),
            (yaml_serde::Value::Bool(false), "false"),
            (yaml_serde::Value::Number(42.into()), "42"),
            (yaml_serde::Value::Number((-7).into()), "-7"),
            (yaml_serde::Value::Number(1.5.into()), "1.5"),
        ];
        let registry = HarnessRegistry::with_builtins();
        for (value, expected) in cases {
            let (_dir, skill) = create_skill_with_template(
                "yaml-scalar",
                "value: {{ value | yaml_str }}\n",
                BTreeMap::from([("value".to_string(), value)]),
            );
            let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
            let output = Engine::render(&pair).unwrap();
            assert_eq!(output.skill_content, format!("value: \"{expected}\"\n"));
            let parsed: yaml_serde::Value = yaml_serde::from_str(&output.skill_content).unwrap();
            assert_eq!(parsed["value"].as_str(), Some(expected));
        }
    }

    #[test]
    fn yaml_str_undefined_optional_field_parses_as_null() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template(
            "yaml-null",
            "---\nlicense: {{ license | yaml_str }}\nbare_license: {{ license }}\n---\nBody\n",
            BTreeMap::new(),
        );
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        assert!(build_context(&pair)["license"].is_undefined());
        let output = Engine::render(&pair).unwrap();
        assert_eq!(
            output.skill_content,
            "---\nlicense: null\nbare_license: \n---\nBody\n"
        );
        let frontmatter = output
            .skill_content
            .strip_prefix("---\n")
            .unwrap()
            .split_once("\n---\n")
            .unwrap()
            .0;
        let parsed: yaml_serde::Value = yaml_serde::from_str(frontmatter).unwrap();
        assert_eq!(parsed["license"], yaml_serde::Value::Null);
        assert_eq!(parsed["license"], parsed["bare_license"]);
    }

    #[test]
    fn yaml_str_none_parses_as_null() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template(
            "yaml-null",
            "value: {{ value | yaml_str }}\n",
            BTreeMap::from([("value".to_string(), yaml_serde::Value::Null)]),
        );
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        assert!(build_context(&pair)["value"].is_none());
        // Exercise a YAML filename too: null must not be autoescaped into "null".
        pair.harness.sidecars = vec![crate::registry::SidecarDef {
            filename: "config.yaml".to_string(),
            template: "value: {{ value | yaml_str }}\n".to_string(),
            output_dir: None,
        }];
        let output = Engine::render(&pair).unwrap();
        for content in [&output.skill_content, &output.sidecars[0].content] {
            assert_eq!(content, "value: null\n");
            let parsed: yaml_serde::Value = yaml_serde::from_str(content).unwrap();
            assert_eq!(parsed["value"], yaml_serde::Value::Null);
        }
    }

    #[test]
    fn yaml_str_is_available_in_sidecars_and_manifests() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, mut skill) =
            create_skill_with_template("yaml-scalar", "Body\n", BTreeMap::new());
        skill.description = "Router: say \"hello\"\nback\\slash".to_string();
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let template = "description: {{ skill_description | yaml_str }}\n";
        pair.harness.sidecars = vec![crate::registry::SidecarDef {
            filename: "config.yaml".to_string(),
            template: template.to_string(),
            output_dir: None,
        }];
        pair.harness.manifest = Some(ManifestDef {
            template: "{\"description\": {{ skill_description | yaml_str }}}\n".to_string(),
        });
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.sidecars.len(), 1);
        let manifest = Engine::render_manifest_entry(&pair).unwrap().unwrap();
        for content in [&output.sidecars[0].content, &manifest] {
            let parsed: yaml_serde::Value = yaml_serde::from_str(content).unwrap();
            assert_eq!(
                parsed["description"].as_str(),
                Some(skill.description.as_str())
            );
        }
    }

    #[test]
    fn unfiltered_interpolation_remains_unescaped() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, mut skill) = create_skill_with_template(
            "raw-description",
            "---\ndescription: {{ skill_description | yaml_str }}\n---\n{{ skill_description }}\n",
            BTreeMap::new(),
        );
        skill.description = "Router: \"hello\" & <world>".to_string();
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = Engine::render(&pair).unwrap();
        let (_, body) = output.skill_content.split_once("\n---\n").unwrap();
        assert_eq!(body, format!("{}\n", skill.description));
    }

    #[test]
    fn renders_with_variable_substitution() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "theme".to_string(),
            yaml_serde::Value::String("dark".into()),
        );
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template("styled", "{{ theme }}", vars);
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.skill_content, "dark");
    }

    #[test]
    fn renders_harness_macro() {
        let output = render_pair("test-macro", "{{ harness.hints }}", "claude").unwrap();
        assert!(output.skill_content.contains("Agent Skills specification"));
    }

    #[test]
    fn renders_manifest_entry_for_harness_with_manifest() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) =
            create_skill_with_template("test-agent", "{{ skill_name }}", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let entry = Engine::render_manifest_entry(&pair).unwrap();
        assert!(entry.is_some());
        let content = entry.unwrap();
        assert!(content.contains("test-agent"));
    }

    #[test]
    fn render_manifest_entry_none_for_harness_without_manifest() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) =
            create_skill_with_template("test-agent", "{{ skill_name }}", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();
        let entry = Engine::render_manifest_entry(&pair).unwrap();
        assert!(entry.is_none());
    }

    #[test]
    fn skill_ref_uses_harness_pattern_in_rendered_skill() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template(
            "ref-skill",
            "Ref: {{ skill_ref(\"other\") }}",
            BTreeMap::new(),
        );
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        pair.harness.skill_ref_pattern = Some("@{name}".to_string());
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.skill_content, "Ref: @other");
    }

    #[test]
    fn skill_ref_falls_back_to_slash_pattern_when_harness_pattern_unset() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template(
            "ref-skill",
            "Ref: {{ skill_ref(\"other\") }}",
            BTreeMap::new(),
        );
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        pair.harness.skill_ref_pattern = None;
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.skill_content, "Ref: /other");
    }

    #[test]
    fn skill_ref_uses_harness_pattern_in_manifest_entry() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) =
            create_skill_with_template("test-agent", "{{ skill_name }}", BTreeMap::new());
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        pair.harness.skill_ref_pattern = Some("@{name}".to_string());
        let entry = Engine::render_manifest_entry(&pair).unwrap().unwrap();
        assert!(
            entry.contains("@test-agent"),
            "manifest should use the harness skill_ref_pattern, got: {entry}"
        );
    }

    #[test]
    fn skill_ref_uses_harness_pattern_in_sidecar() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) =
            create_skill_with_template("test-agent", "{{ skill_name }}", BTreeMap::new());
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        pair.harness.skill_ref_pattern = Some("@{name}".to_string());
        pair.harness.sidecars = vec![crate::registry::SidecarDef {
            filename: "ref.txt".to_string(),
            template: "{{ skill_ref(skill_name) }}".to_string(),
            output_dir: None,
        }];
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.sidecars.len(), 1);
        assert_eq!(output.sidecars[0].content, "@test-agent");
    }

    #[test]
    fn render_manifest_entry_error_for_invalid_template() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) =
            create_skill_with_template("test-agent", "{{ skill_name }}", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        // Corrupt the manifest template to make rendering fail
        let mut pair = pair;
        if let Some(ref mut manifest) = pair.harness.manifest {
            manifest.template = "{{ .broken".to_string();
        }
        let result = Engine::render_manifest_entry(&pair);
        match result {
            Err(EngineError::RenderError { template, .. }) => {
                assert_eq!(template, "(manifest)");
            }
            other => panic!("expected Err(RenderError), got {other:?}"),
        }
    }

    #[test]
    fn render_template_read_error() {
        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("no-exist", vec![]);
        skill.template_path = Path::new("/nonexistent/template.j2").to_path_buf();
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let result = Engine::render(&pair);
        assert!(result.is_err());
        match result.unwrap_err() {
            EngineError::TemplateRead { .. } => {}
            e @ (EngineError::RenderError { .. } | EngineError::Frontmatter(_)) => {
                panic!("expected TemplateRead, got {e:?}")
            }
        }
    }

    #[test]
    fn render_syntax_error_reported() {
        let registry = HarnessRegistry::with_builtins();
        let (_dir, skill) = create_skill_with_template("broken", "{{ broken", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let result = Engine::render(&pair);
        assert!(result.is_err());
        match result.unwrap_err() {
            EngineError::RenderError { template, line, .. } => {
                assert!(
                    template.ends_with("SKILL.md.j2"),
                    "template path should end with .j2 file, got {template}"
                );
                assert_eq!(line, Some(1), "syntax error on line 1, got {line:?}");
            }
            e @ (EngineError::TemplateRead { .. } | EngineError::Frontmatter(_)) => {
                panic!("expected RenderError, got {e:?}")
            }
        }
    }
}

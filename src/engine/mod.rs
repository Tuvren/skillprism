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
mod helpers;

use std::collections::BTreeMap;
use std::fs;

use miette::Diagnostic;
use thiserror::Error;

use crate::registry::ManifestDef;
use crate::resolver::ResolvedPair;

pub use context::build_context;
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
    RenderError {
        skill: String,
        harness: String,
        template: String,
        line: Option<usize>,
        detail: String,
    },
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
        register_helpers(&mut env);

        let name = pair.skill.template_path.to_string_lossy();
        env.add_template_owned(name.to_string(), content)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

        let tmpl = env
            .get_template(&name)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

        let skill_content = tmpl
            .render(&ctx)
            .map_err(|e| render_error_from_minijinja(pair, &name, &e))?;

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
        render_manifest(manifest, &ctx)
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
        register_helpers(&mut env);
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
) -> Result<String, minijinja::Error> {
    let mut env = minijinja::Environment::new();
    register_helpers(&mut env);
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
            e @ EngineError::RenderError { .. } => panic!("expected TemplateRead, got {e:?}"),
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
            e @ EngineError::TemplateRead { .. } => panic!("expected RenderError, got {e:?}"),
        }
    }
}

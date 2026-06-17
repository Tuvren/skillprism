#![allow(dead_code)]

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

#[derive(Debug, Clone)]
pub struct HarnessOutput {
    pub skill_content: String,
    pub sidecars: Vec<SidecarOutput>,
    pub manifest_entry: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SidecarOutput {
    pub filename: String,
    pub content: String,
    pub output_dir: Option<String>,
}

#[derive(Debug, Diagnostic, Error)]
pub enum EngineError {
    #[error("[{skill}] {harness}: Failed to read template `{path}`")]
    #[diagnostic(help("{detail}"))]
    TemplateRead {
        skill: String,
        harness: String,
        path: String,
        detail: String,
    },

    #[error("[{skill}] {harness}: Template rendering failed")]
    #[diagnostic(help("{detail}"))]
    RenderError {
        skill: String,
        harness: String,
        detail: String,
    },

    #[error("[{skill}] {harness}: Collision — template `{template_name}` registered by multiple harnesses")]
    #[diagnostic(help("Ensure harness templates have unique names"))]
    TemplateCollision {
        skill: String,
        harness: String,
        template_name: String,
    },
}

pub struct Engine;

impl Engine {
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
            .map_err(|e| EngineError::RenderError {
                skill: pair.skill.name.clone(),
                harness: pair.harness.id.clone(),
                detail: e.to_string(),
            })?;

        let tmpl = env.get_template(&name).map_err(|e| EngineError::RenderError {
            skill: pair.skill.name.clone(),
            harness: pair.harness.id.clone(),
            detail: e.to_string(),
        })?;

        let skill_content = tmpl.render(&ctx).map_err(|e| EngineError::RenderError {
            skill: pair.skill.name.clone(),
            harness: pair.harness.id.clone(),
            detail: e.to_string(),
        })?;

        let sidecars = render_sidecars(pair, &ctx).map_err(|e| EngineError::RenderError {
            skill: pair.skill.name.clone(),
            harness: pair.harness.id.clone(),
            detail: format!("Sidecar rendering failed: {e}"),
        })?;

        let manifest_entry = if let Some(ref manifest) = pair.harness.manifest {
            Some(render_manifest(manifest, &ctx).map_err(|e| {
                EngineError::RenderError {
                    skill: pair.skill.name.clone(),
                    harness: pair.harness.id.clone(),
                    detail: format!("Manifest rendering failed: {e}"),
                }
            })?)
        } else {
            None
        };

        Ok(HarnessOutput {
            skill_content,
            sidecars,
            manifest_entry,
        })
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
    use crate::resolver::tests::test_skill;
    use crate::resolver::HarnessResolver;
    use crate::types::SkillModel;
    use std::path::Path;

    fn create_skill_with_template(
        name: &str,
        template_content: &str,
        vars: BTreeMap<String, yaml_serde::Value>,
    ) -> SkillModel {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("engine")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tmpl_path = dir.join("SKILL.md.j2");
        std::fs::write(&tmpl_path, template_content).unwrap();

        let mut skill = test_skill(name, vec![]);
        skill.template_path = tmpl_path;
        skill.variables = vars;
        skill
    }

    fn render_pair(
        skill_name: &str,
        template: &str,
        harness_name: &str,
    ) -> Result<HarnessOutput, EngineError> {
        let registry = HarnessRegistry::with_builtins();
        let skill = create_skill_with_template(skill_name, template, BTreeMap::new());
        let pair =
            HarnessResolver::resolve_skill_harness(&skill, harness_name, &registry).unwrap();
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
        let skill = create_skill_with_template("styled", "{{ theme }}", vars);
        let pair =
            HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let output = Engine::render(&pair).unwrap();
        assert_eq!(output.skill_content, "dark");
    }

    #[test]
    fn renders_harness_macro() {
        let output = render_pair(
            "test-macro",
            "{{ harness.hints }}",
            "claude",
        )
        .unwrap();
        assert!(output.skill_content.contains("Agent Skills specification"));
    }

    #[test]
    fn render_template_read_error() {
        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("no-exist", vec![]);
        skill.template_path = Path::new("/nonexistent/template.j2").to_path_buf();
        let pair =
            HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let result = Engine::render(&pair);
        assert!(result.is_err());
        match result.unwrap_err() {
            EngineError::TemplateRead { .. } => {}
            e => panic!("expected TemplateRead, got {e:?}"),
        }
    }

    #[test]
    fn render_syntax_error_reported() {
        let registry = HarnessRegistry::with_builtins();
        let skill = create_skill_with_template("broken", "{{ broken", BTreeMap::new());
        let pair =
            HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let result = Engine::render(&pair);
        assert!(result.is_err());
        match result.unwrap_err() {
            EngineError::RenderError { .. } => {}
            e => panic!("expected RenderError, got {e:?}"),
        }
    }
}

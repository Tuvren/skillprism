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

mod macros;
mod syntax;
mod variables;

use std::fs;

use miette::Diagnostic;
use thiserror::Error;

use crate::resolver::ResolvedPair;

/// Errors found during template validation.
#[derive(Debug, Diagnostic, Error)]
pub enum ValidationError {
    /// The template has invalid Jinja2 syntax.
    #[error("[{skill}] {harness}: Template syntax error")]
    #[diagnostic(help("{detail}"))]
    SyntaxError {
        skill: String,
        harness: String,
        detail: String,
    },

    /// The template references a variable not defined in skill.yaml.
    #[error("[{skill}] {harness}: Undefined template variable `{variable_name}`")]
    #[diagnostic(help(
        "Ensure the variable is defined in skill.yaml or one of its parent group skill.yaml files. Template: {template_path}"
    ))]
    UndefinedVariable {
        skill: String,
        harness: String,
        variable_name: String,
        template_path: String,
    },

    /// The template references a harness macro that is not defined.
    #[error("[{skill}] {harness}: Undefined harness macro `{macro_name}`")]
    #[diagnostic(help(
        "Ensure the macro is defined in the harness definition. Template: {template_path}"
    ))]
    UndefinedMacro {
        skill: String,
        harness: String,
        macro_name: String,
        template_path: String,
    },

    /// The template file could not be read from disk.
    #[error("[{skill}] {harness}: Failed to read template file")]
    #[diagnostic(help("{detail}"))]
    TemplateRead {
        skill: String,
        harness: String,
        detail: String,
    },
}

/// Outcome of validating a batch of resolved pairs.
pub struct ValidationOutcome {
    /// Pairs that passed all validation checks.
    pub valid: Vec<ResolvedPair>,
    /// Errors collected from all pairs.
    pub errors: Vec<ValidationError>,
}

/// Runs validation checks on resolved skill-harness pairs.
pub struct Validator;

impl Validator {
    /// Validate all resolved pairs, collecting errors without short-circuiting.
    pub fn validate(pairs: Vec<ResolvedPair>) -> ValidationOutcome {
        let mut valid = Vec::new();
        let mut errors = Vec::new();

        for pair in pairs {
            Self::validate_pair(&pair, &mut errors);
            if !has_error_for_skill(&errors, &pair.skill.name, &pair.harness.id) {
                valid.push(pair);
            }
        }

        ValidationOutcome { valid, errors }
    }

    fn validate_pair(pair: &ResolvedPair, errors: &mut Vec<ValidationError>) {
        let template_path = &pair.skill.template_path;
        let content = match fs::read_to_string(template_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(ValidationError::TemplateRead {
                    skill: pair.skill.name.clone(),
                    harness: pair.harness.id.clone(),
                    detail: format!("{}: {e}", template_path.display()),
                });
                return;
            }
        };

        let skill_name = &pair.skill.name;
        let harness_id = &pair.harness.id;

        if let Err(detail) = syntax::check_syntax(&content, template_path) {
            errors.push(ValidationError::SyntaxError {
                skill: skill_name.clone(),
                harness: harness_id.clone(),
                detail,
            });
            return;
        }

        let resolved_variables = pair.skill.variables_for_harness(harness_id);
        let var_errors = variables::check_variables(&content, template_path, &resolved_variables);
        for uvar in var_errors {
            errors.push(ValidationError::UndefinedVariable {
                skill: skill_name.clone(),
                harness: harness_id.clone(),
                variable_name: uvar.variable_name,
                template_path: uvar.template_path,
            });
        }

        let mut resolved_macros = pair.harness.macros.clone();
        if let Some(override_) = pair.skill.harness_overrides.get(harness_id) {
            for (name, content) in &override_.macros {
                resolved_macros.insert(
                    name.clone(),
                    crate::registry::MacroDef::Inline(content.clone()),
                );
            }
        }
        let macro_errors = macros::check_macros(&content, template_path, &resolved_macros);
        for umacro in macro_errors {
            errors.push(ValidationError::UndefinedMacro {
                skill: skill_name.clone(),
                harness: harness_id.clone(),
                macro_name: umacro.macro_name,
                template_path: umacro.template_path,
            });
        }
    }
}

fn has_error_for_skill(errors: &[ValidationError], skill: &str, harness: &str) -> bool {
    errors.iter().any(|e| match e {
        ValidationError::SyntaxError {
            skill: s,
            harness: h,
            ..
        }
        | ValidationError::UndefinedVariable {
            skill: s,
            harness: h,
            ..
        }
        | ValidationError::UndefinedMacro {
            skill: s,
            harness: h,
            ..
        }
        | ValidationError::TemplateRead {
            skill: s,
            harness: h,
            ..
        } => s == skill && h == harness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;
    use crate::resolver::HarnessResolver;
    use crate::types::SkillModel;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn test_skill(
        name: &str,
        template_content: &str,
        vars: BTreeMap<String, yaml_serde::Value>,
    ) -> SkillModel {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("validator")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let tmpl_path = dir.join("SKILL.md.j2");
        fs::write(&tmpl_path, template_content).unwrap();

        SkillModel {
            name: name.to_string(),
            directory_name: name.to_string(),
            description: String::new(),
            version: None,
            license: None,
            compatibility: None,
            metadata: BTreeMap::new(),
            allowed_tools: None,
            when_to_use: None,
            argument_hint: None,
            arguments: None,
            disable_model_invocation: None,
            user_invocable: None,
            disallowed_tools: None,
            model_override: None,
            effort: None,
            context_fork: false,
            agent: None,
            hooks: None,
            activation_paths: None,
            shell: None,
            required_capabilities: Vec::new(),
            variables: vars,
            template_path: tmpl_path,
            asset_dirs: Vec::new(),
            harness_overrides: BTreeMap::new(),
        }
    }

    fn resolve_pair(name: &str, harness_name: &str) -> ResolvedPair {
        let registry = HarnessRegistry::with_builtins();
        let skill = test_skill(name, "", BTreeMap::new());
        HarnessResolver::resolve_skill_harness(&skill, harness_name, &registry).unwrap()
    }

    #[test]
    fn all_skills_pass_validation() {
        let pairs = vec![
            resolve_pair("skill-a", "claude"),
            resolve_pair("skill-b", "opencode"),
        ];

        let outcome = Validator::validate(pairs);
        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.valid.len(), 2);
    }

    #[test]
    fn syntax_error_collected() {
        let registry = HarnessRegistry::with_builtins();
        let skill = test_skill("broken", "Hello {{ name }", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();

        let outcome = Validator::validate(vec![pair]);
        assert_eq!(outcome.errors.len(), 1);
        match &outcome.errors[0] {
            ValidationError::SyntaxError { skill, .. } => {
                assert_eq!(skill, "broken");
            }
            e @ (ValidationError::UndefinedVariable { .. }
            | ValidationError::UndefinedMacro { .. }
            | ValidationError::TemplateRead { .. }) => {
                panic!("expected SyntaxError, got {e:?}")
            }
        }
        assert!(outcome.valid.is_empty());
    }

    #[test]
    fn undefined_variable_collected() {
        let registry = HarnessRegistry::with_builtins();
        let skill = test_skill("missing-var", "Hello {{ unknown }}!", BTreeMap::new());
        let pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();

        let outcome = Validator::validate(vec![pair]);

        assert_eq!(
            outcome
                .errors
                .iter()
                .filter(|e| matches!(e, ValidationError::UndefinedVariable { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn collect_errors_from_multiple_skills() {
        let registry = HarnessRegistry::with_builtins();

        let good_skill = test_skill("good", "Hello {{ name }}!", {
            let mut v = BTreeMap::new();
            v.insert("name".to_string(), yaml_serde::Value::String("test".into()));
            v
        });
        let bad_skill = test_skill("bad", "Hello {{ unknown }}!", BTreeMap::new());

        let pairs = vec![
            HarnessResolver::resolve_skill_harness(&good_skill, "claude", &registry).unwrap(),
            HarnessResolver::resolve_skill_harness(&bad_skill, "claude", &registry).unwrap(),
        ];

        let outcome = Validator::validate(pairs);
        assert_eq!(outcome.valid.len(), 1);
        assert_eq!(outcome.valid[0].skill.name, "good");

        assert_eq!(
            outcome
                .errors
                .iter()
                .filter(|e| matches!(e, ValidationError::UndefinedVariable { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn template_read_error_handled() {
        let registry = HarnessRegistry::with_builtins();
        let skill = test_skill("no-file", "anything", BTreeMap::new());
        let mut pair = HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        pair.skill.template_path = Path::new("/nonexistent/template.j2").to_path_buf();

        let outcome = Validator::validate(vec![pair]);
        assert_eq!(outcome.errors.len(), 1);
        match &outcome.errors[0] {
            ValidationError::TemplateRead { .. } => {}
            e @ (ValidationError::SyntaxError { .. }
            | ValidationError::UndefinedVariable { .. }
            | ValidationError::UndefinedMacro { .. }) => {
                panic!("expected TemplateRead, got {e:?}")
            }
        }
    }

    #[test]
    fn harness_only_variable_not_flagged_undefined_for_its_own_harness() {
        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill("harness-var", "Hello {{ custom_var }}!", BTreeMap::new());
        let mut claude_vars = BTreeMap::new();
        claude_vars.insert(
            "custom_var".to_string(),
            yaml_serde::Value::String("hi".into()),
        );
        skill.harness_overrides.insert(
            "claude".to_string(),
            crate::types::HarnessOverride {
                variables: claude_vars,
                macros: BTreeMap::new(),
            },
        );

        let claude_pair =
            HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let opencode_pair =
            HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();

        let outcome = Validator::validate(vec![claude_pair, opencode_pair]);

        assert_eq!(outcome.valid.len(), 1);
        assert_eq!(outcome.valid[0].harness.id, "claude");
        assert_eq!(
            outcome
                .errors
                .iter()
                .filter(|e| matches!(e, ValidationError::UndefinedVariable { harness, .. } if harness == "opencode"))
                .count(),
            1,
            "opencode has no override for custom_var, so it should still be reported undefined there"
        );
    }

    #[test]
    fn harness_only_macro_not_flagged_undefined_for_its_own_harness() {
        let registry = HarnessRegistry::with_builtins();
        let mut skill = test_skill(
            "harness-macro",
            "{{ harness.custom_macro }}",
            BTreeMap::new(),
        );
        let mut claude_macros = BTreeMap::new();
        claude_macros.insert("custom_macro".to_string(), "Custom!".to_string());
        skill.harness_overrides.insert(
            "claude".to_string(),
            crate::types::HarnessOverride {
                variables: BTreeMap::new(),
                macros: claude_macros,
            },
        );

        let claude_pair =
            HarnessResolver::resolve_skill_harness(&skill, "claude", &registry).unwrap();
        let opencode_pair =
            HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry).unwrap();

        let outcome = Validator::validate(vec![claude_pair, opencode_pair]);

        assert_eq!(outcome.valid.len(), 1);
        assert_eq!(outcome.valid[0].harness.id, "claude");
        assert_eq!(
            outcome
                .errors
                .iter()
                .filter(|e| matches!(e, ValidationError::UndefinedMacro { harness, .. } if harness == "opencode"))
                .count(),
            1,
            "opencode has no override for custom_macro, so it should still be reported undefined there"
        );
    }

    #[test]
    fn valid_skills_included_with_partial_failures() {
        let registry = HarnessRegistry::with_builtins();

        // Distinct names from other tests in this file's `good`/`broken` pairs —
        // test_skill() writes to a path keyed only by name under the shared
        // std::env::temp_dir(), so reusing a name races other tests under parallel
        // execution (each test's setup removes and recreates the same directory).
        let good = test_skill("good-partial", "ok", BTreeMap::new());
        let broken = test_skill("broken-partial", "{{ broken", BTreeMap::new());

        let pairs = vec![
            HarnessResolver::resolve_skill_harness(&good, "claude", &registry).unwrap(),
            HarnessResolver::resolve_skill_harness(&broken, "claude", &registry).unwrap(),
        ];

        let outcome = Validator::validate(pairs);
        assert_eq!(outcome.valid.len(), 1);
        assert_eq!(outcome.valid[0].skill.name, "good-partial");
        assert_eq!(outcome.errors.len(), 1);
    }
}

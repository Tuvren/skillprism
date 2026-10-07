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

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use minijinja::Environment;

use crate::types::{HELPER_FUNCTIONS, SKILL_METADATA_FIELDS};

/// Checks that all template variables are defined in skill.yaml or built-in.
pub fn check_variables(
    template_content: &str,
    template_path: &Path,
    resolved_variables: &BTreeMap<String, yaml_serde::Value>,
) -> Vec<UndefinedVariable> {
    let mut env = Environment::new();
    let name = template_path.to_string_lossy();
    let Ok(()) = env.add_template(&name, template_content) else {
        return Vec::new();
    };
    let Ok(template) = env.get_template(&name) else {
        return Vec::new();
    };

    let undeclared: HashSet<String> = template.undeclared_variables(true);
    let known: HashSet<&str> = resolved_variables.keys().map(String::as_str).collect();
    // Derive builtins from MiniJinja so feature or version changes cannot stale a list.
    let builtin_globals: HashSet<&str> = env.globals().map(|(name, _)| name).collect();

    let mut errors = Vec::new();
    for var in &undeclared {
        if is_builtin(var, &builtin_globals) {
            continue;
        }
        if !known.contains(var.as_str()) {
            errors.push(UndefinedVariable {
                variable_name: var.clone(),
                template_path: template_path.to_string_lossy().to_string(),
            });
        }
    }

    errors
}

fn is_builtin(name: &str, builtin_globals: &HashSet<&str>) -> bool {
    let root = name.split('.').next().unwrap();
    builtin_globals.contains(root)
        || HELPER_FUNCTIONS.contains(&root)
        || matches!(
            root,
            "loop"
                | "self"
                | "kwargs"
                | "varargs"
                | "super"
                | "g"
                | "harness"
                | "_"
                | "skill_name"
                | "skill_description"
        )
        || SKILL_METADATA_FIELDS.contains(&root)
}

/// Checks that no skill.yaml variable name collides with a built-in field or helper.
///
/// `build_context` (`engine::context`) inserts `skill_name`, `skill_description`, and
/// every `SKILL_METADATA_FIELDS` entry before skill variables, then lets variables
/// overwrite them unconditionally — a variable named e.g. `version` or `license` would
/// otherwise silently shadow the skill's own declared metadata with no warning.
/// Variables also shadow helper functions, preventing templates from calling them.
pub fn check_reserved_names(
    resolved_variables: &BTreeMap<String, yaml_serde::Value>,
) -> Vec<String> {
    resolved_variables
        .keys()
        .filter(|name| is_reserved(name))
        .cloned()
        .collect()
}

fn is_reserved(name: &str) -> bool {
    matches!(name, "skill_name" | "skill_description")
        || SKILL_METADATA_FIELDS.contains(&name)
        || HELPER_FUNCTIONS.contains(&name)
}

/// A template variable that was used but not defined in skill.yaml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndefinedVariable {
    /// Name of the undefined variable.
    pub variable_name: String,
    /// Path to the template containing the undefined reference.
    pub template_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minijinja_globals_not_reported_as_undefined() {
        let errors = check_variables(
            "{% for i in range(2) %}{{ i }}{% endfor %} {{ dict(a=1).a }}",
            Path::new("t.j2"),
            &BTreeMap::new(),
        );
        assert!(
            errors.is_empty(),
            "MiniJinja globals should pass: {errors:?}"
        );
    }

    #[test]
    fn helper_function_not_reported_as_undefined() {
        let vars = BTreeMap::new();
        let errors = check_variables("Ref: {{ skill_ref(\"other\") }}", Path::new("t.j2"), &vars);
        let names: Vec<&str> = errors.iter().map(|e| e.variable_name.as_str()).collect();
        assert!(
            names.is_empty(),
            "registered helper functions must not be reported as undefined, got: {names:?}"
        );
    }

    #[test]
    fn undefined_variable_reported_alongside_helper_function() {
        let vars = BTreeMap::new();
        let errors = check_variables(
            "{{ skill_ref(name) }} {{ not_defined }}",
            Path::new("t.j2"),
            &vars,
        );
        let mut names: Vec<&str> = errors.iter().map(|e| e.variable_name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["name", "not_defined"]);
    }

    #[test]
    fn defined_variable_passes() {
        let mut vars = BTreeMap::new();
        vars.insert("name".to_string(), yaml_serde::Value::String("test".into()));
        let errors = check_variables("Hello {{ name }}!", Path::new("t.j2"), &vars);
        assert!(errors.is_empty());
    }

    #[test]
    fn undefined_variable_reported() {
        let vars = BTreeMap::new();
        let errors = check_variables("Hello {{ name }}!", Path::new("t.j2"), &vars);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].variable_name, "name");
        assert!(errors[0].template_path.contains("t.j2"));
    }

    #[test]
    fn multiple_undefined_variables() {
        let vars = BTreeMap::new();
        let errors = check_variables("{{ a }} {{ b }} {{ c }}", Path::new("t.j2"), &vars);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn harness_variable_not_reported_as_undefined() {
        let vars = BTreeMap::new();
        let errors = check_variables("{{ harness.name }}", Path::new("t.j2"), &vars);
        let names: Vec<&str> = errors.iter().map(|e| e.variable_name.as_str()).collect();
        assert!(
            names.is_empty() || !names.contains(&"harness"),
            "harness should not be reported as undefined, got: {names:?}"
        );
    }

    #[test]
    fn harness_filtered_from_simple_ref() {
        let vars = BTreeMap::new();
        let errors = check_variables("{{ harness }}", Path::new("t.j2"), &vars);
        assert!(errors.is_empty(), "harness alone should be builtin");
    }

    #[test]
    fn skill_name_and_description_not_reported() {
        let vars = BTreeMap::new();
        let errors = check_variables(
            "{{ skill_name }} {{ skill_description }}",
            Path::new("t.j2"),
            &vars,
        );
        assert!(errors.is_empty(), "engine-injected builtins should pass");
    }

    #[test]
    fn partial_variables_resolved() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "theme".to_string(),
            yaml_serde::Value::String("dark".into()),
        );
        let errors = check_variables("{{ theme }} {{ missing }}", Path::new("t.j2"), &vars);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].variable_name, "missing");
    }

    #[test]
    fn invalid_template_no_panic() {
        let vars = BTreeMap::new();
        let errors = check_variables("{{ broken", Path::new("t.j2"), &vars);
        assert!(errors.is_empty());
    }

    #[test]
    fn reserved_name_collision_reported() {
        let mut vars = BTreeMap::new();
        vars.insert("version".to_string(), yaml_serde::Value::String("2".into()));
        vars.insert(
            "theme".to_string(),
            yaml_serde::Value::String("dark".into()),
        );
        let reserved = check_reserved_names(&vars);
        assert_eq!(reserved, vec!["version".to_string()]);
    }

    #[test]
    fn no_reserved_name_collision_when_none_present() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "theme".to_string(),
            yaml_serde::Value::String("dark".into()),
        );
        assert!(check_reserved_names(&vars).is_empty());
    }

    #[test]
    fn skill_name_and_description_are_reserved() {
        let mut vars = BTreeMap::new();
        vars.insert(
            "skill_name".to_string(),
            yaml_serde::Value::String("x".into()),
        );
        vars.insert(
            "skill_description".to_string(),
            yaml_serde::Value::String("y".into()),
        );
        let reserved = check_reserved_names(&vars);
        assert_eq!(reserved.len(), 2);
    }
}

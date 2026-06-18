use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use minijinja::Environment;

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

    let mut errors = Vec::new();
    for var in &undeclared {
        if is_builtin(var) {
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

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "loop"
            | "self"
            | "kwargs"
            | "varargs"
            | "namespace"
            | "super"
            | "g"
            | "harness"
            | "_"
            | "skill_name"
            | "skill_description"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndefinedVariable {
    pub variable_name: String,
    pub template_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

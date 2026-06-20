use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use minijinja::Environment;

use crate::registry::MacroDef;

/// Checks that all `harness.<name>` references in the template are defined.
///
/// Uses `MiniJinja`'s `undeclared_variables(true)` to find ALL harness
/// references across all template constructs (output blocks `{{ }}`,
/// conditionals `{% if %}`, assignments `{% set %}`, loops `{% for %}`,
/// and filters).
pub fn check_macros(
    template_content: &str,
    template_path: &Path,
    macros: &BTreeMap<String, MacroDef>,
) -> Vec<UndefinedMacro> {
    let mut env = Environment::new();
    let name = template_path.to_string_lossy();
    let Ok(()) = env.add_template(&name, template_content) else {
        return Vec::new();
    };
    let Ok(template) = env.get_template(&name) else {
        return Vec::new();
    };

    let undeclared: HashSet<String> = template.undeclared_variables(true);

    let mut errors = Vec::new();
    for var in &undeclared {
        let Some(macro_name) = var.strip_prefix("harness.") else {
            continue;
        };
        let first_attr = macro_name.split('.').next().unwrap_or(macro_name);
        if !macros.contains_key(first_attr) && !is_harness_builtin(first_attr) {
            errors.push(UndefinedMacro {
                macro_name: first_attr.to_string(),
                template_path: template_path.to_string_lossy().to_string(),
            });
        }
    }

    errors
}

fn is_harness_builtin(name: &str) -> bool {
    matches!(name, "id" | "name" | "version" | "skill_ref_pattern")
}

/// A harness macro reference that was used but not defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndefinedMacro {
    /// Name of the undefined macro.
    pub macro_name: String,
    /// Path to the template containing the undefined reference.
    pub template_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_macros() -> BTreeMap<String, MacroDef> {
        BTreeMap::new()
    }

    #[test]
    fn defined_macro_passes() {
        let mut macros = BTreeMap::new();
        macros.insert(
            "header".to_string(),
            MacroDef::Inline("# Header".to_string()),
        );
        let errors = check_macros("{{ harness.header }}", Path::new("t.j2"), &macros);
        assert!(errors.is_empty());
    }

    #[test]
    fn undefined_macro_reported() {
        let errors = check_macros("{{ harness.missing }}", Path::new("t.j2"), &empty_macros());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].macro_name, "missing");
    }

    #[test]
    fn builtin_harness_fields_not_reported() {
        let errors = check_macros(
            "{{ harness.id }} {{ harness.name }}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn non_harness_refs_ignored() {
        let errors = check_macros(
            "{{ skill_name }} {{ items|length }}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn multiple_harness_refs_all_checked() {
        let mut macros = BTreeMap::new();
        macros.insert("header".to_string(), MacroDef::Inline("H".to_string()));
        let errors = check_macros(
            "{{ harness.header }} {{ harness.footer }} {{ harness.missing }}",
            Path::new("t.j2"),
            &macros,
        );
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn nested_harness_ref_reported_as_undefined() {
        let errors = check_macros(
            "{{ harness.capabilities.supports_subagent }}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].macro_name, "capabilities");
    }

    #[test]
    fn if_block_harness_missing_reported() {
        let errors = check_macros(
            "{% if harness.missing %}yes{% endif %}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].macro_name, "missing");
    }

    #[test]
    fn set_block_harness_missing_reported() {
        let errors = check_macros(
            "{% set x = harness.missing %}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].macro_name, "missing");
    }

    #[test]
    fn if_block_harness_builtin_not_reported() {
        let errors = check_macros(
            "{% if harness.id %}yes{% endif %}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn for_block_harness_missing_reported() {
        let errors = check_macros(
            "{% for item in harness.items %}x{% endfor %}",
            Path::new("t.j2"),
            &empty_macros(),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].macro_name, "items");
    }
}

use std::collections::BTreeMap;
use std::path::Path;

use crate::registry::MacroDef;

/// Checks that all `harness.<name>` references in the template are defined.
pub fn check_macros(
    template_content: &str,
    template_path: &Path,
    macros: &BTreeMap<String, MacroDef>,
) -> Vec<UndefinedMacro> {
    let mut errors = Vec::new();
    let mut pos = 0;

    while let Some(start) = template_content[pos..].find("{{") {
        let expr_start = pos + start + 2;
        let Some(end) = template_content[expr_start..].find("}}") else {
            break;
        };
        let expr = template_content[expr_start..expr_start + end].trim();

        if let Some(macro_name) = extract_harness_ref(expr) {
            if !macros.contains_key(macro_name) && !is_harness_builtin(macro_name) {
                errors.push(UndefinedMacro {
                    macro_name: macro_name.to_string(),
                    template_path: template_path.to_string_lossy().to_string(),
                });
            }
        }

        pos = expr_start + end + 2;
    }

    errors
}

fn extract_harness_ref(expr: &str) -> Option<&str> {
    let expr = expr.trim();
    if let Some(dot_pos) = expr.find('.') {
        let prefix = expr[..dot_pos].trim();
        if prefix == "harness" {
            let rest = expr[dot_pos + 1..].trim();
            let name = rest
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()?;
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
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
}

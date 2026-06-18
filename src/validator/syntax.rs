use minijinja::Environment;
use std::path::Path;

/// Validates that a template has valid Jinja2 syntax.
pub fn check_syntax(template_content: &str, template_path: &Path) -> Result<(), String> {
    let mut env = Environment::new();
    let name = template_path.to_string_lossy();
    env.add_template(&name, template_content)
        .map_err(|e| format!("{} — {e}", template_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_template_parses() {
        let result = check_syntax("Hello {{ name }}!", Path::new("test.j2"));
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_syntax_reports_error() {
        let result = check_syntax("Hello {{ name }", Path::new("test.j2"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("test.j2"));
    }

    #[test]
    fn template_with_blocks_parses() {
        let result = check_syntax(
            "{% for item in items %}{{ item }}{% endfor %}",
            Path::new("loop.j2"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn unterminated_block() {
        let result = check_syntax("{% if true", Path::new("bad.j2"));
        assert!(result.is_err());
    }

    #[test]
    fn empty_template_parses() {
        let result = check_syntax("", Path::new("empty.j2"));
        assert!(result.is_ok());
    }
}

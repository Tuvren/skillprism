use std::fs;
use std::io;
use std::path::Path;

/// Scaffolds a new skillprism project with directory structure and config.
///
/// When `harnesses` is empty, defaults to `["claude", "opencode"]`.
pub fn scaffold_project(dir: &Path, name: &str, harnesses: &[String]) -> io::Result<()> {
    fs::create_dir_all(dir)?;

    let harness_list = if harnesses.is_empty() {
        vec!["claude".to_string(), "opencode".to_string()]
    } else {
        harnesses.to_vec()
    };

    let yaml_lines: Vec<String> = harness_list.iter().map(|h| format!("  - {h}")).collect();

    fs::write(
        dir.join("skillprism.yaml"),
        format!(
            "name: {name}\nharnesses:\n{harness_list}\nskills_dir: skills\n",
            harness_list = yaml_lines.join("\n")
        ),
    )?;

    fs::create_dir_all(dir.join("skills"))?;
    fs::create_dir_all(dir.join("harnesses"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_expected_structure() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_project");
        let _ = fs::remove_dir_all(&dir);

        scaffold_project(&dir, "test-project", &[]).unwrap();
        assert!(dir.join("skillprism.yaml").exists());
        assert!(dir.join("skills").is_dir());
        assert!(dir.join("harnesses").is_dir());

        let content = fs::read_to_string(dir.join("skillprism.yaml")).unwrap();
        assert!(content.contains("test-project"));
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.contains(&"  - claude"));
        assert!(lines.contains(&"  - opencode"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scaffold_with_custom_harnesses() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_custom_harnesses");
        let _ = fs::remove_dir_all(&dir);

        let h = vec!["claude".to_string(), "codex".to_string(), "pi".to_string()];
        scaffold_project(&dir, "custom", &h).unwrap();

        let content = fs::read_to_string(dir.join("skillprism.yaml")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.contains(&"  - claude"));
        assert!(lines.contains(&"  - codex"));
        assert!(lines.contains(&"  - pi"));

        let _ = fs::remove_dir_all(&dir);
    }
}

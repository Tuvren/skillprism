use std::fs;
use std::io;
use std::path::Path;

/// Scaffolds a new skillprism project with directory structure, config, and sample skill.
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

    fs::create_dir_all(dir.join("harnesses"))?;

    let sample_dir = dir.join("skills/sample");
    fs::create_dir_all(&sample_dir)?;
    fs::write(
        sample_dir.join("skill.yaml"),
        "name: sample\ndescription: A sample skill to get started\nvariables:\n  greeting: Hello from sample\n",
    )?;
    fs::write(
        sample_dir.join("SKILL.md.j2"),
        "# {{ skill_name }}\n\n{{ skill_description }}\n\nHarness: {{ harness.id }} ({{ harness.name }})\n\n{{ greeting }}\n",
    )?;

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
        assert!(dir.join("skills/sample/skill.yaml").exists());
        assert!(dir.join("skills/sample/SKILL.md.j2").exists());

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

    #[test]
    fn scaffold_sample_skill_contains_variable_refs() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_sample_refs");
        let _ = fs::remove_dir_all(&dir);

        scaffold_project(&dir, "test", &[]).unwrap();

        let template = fs::read_to_string(dir.join("skills/sample/SKILL.md.j2")).unwrap();
        assert!(
            template.contains("{{ skill_name }}"),
            "template must reference skill_name"
        );
        assert!(
            template.contains("{{ skill_description }}"),
            "template must reference skill_description"
        );
        assert!(
            template.contains("{{ harness.id }}"),
            "template must reference harness.id"
        );
        assert!(
            template.contains("{{ harness.name }}"),
            "template must reference harness.name"
        );
        assert!(
            template.contains("{{ greeting }}"),
            "template must reference greeting variable"
        );

        let skill_yaml = fs::read_to_string(dir.join("skills/sample/skill.yaml")).unwrap();
        assert!(
            skill_yaml.contains("sample"),
            "skill.yaml must contain the skill name"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}

use std::fs;
use std::io;
use std::path::Path;

/// Scaffolds a new custom harness definition YAML in the harnesses/ directory.
pub fn scaffold_harness(project_root: &Path, name: &str) -> io::Result<()> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("harness name `{name}` must not contain path separators or '..'"),
        ));
    }

    let harnesses_dir = project_root.join("harnesses");
    fs::create_dir_all(&harnesses_dir)?;

    fs::write(
        harnesses_dir.join(format!("{name}.yaml")),
        format!(
            "# {name} Harness Definition\n# Edit the values below to configure your custom harness.\n\n\
            id: {name}\n\
            name: {name}\n\
            capabilities:\n  \
                supports_subagent: false\n  \
                requires_sidecar: false\n  \
                requires_manifest: false\n  \
                frontmatter_mode: lenient\n  \
                name_max_length: 64\n  \
                description_max_length: 1024\n\
            paths:\n  \
                project_scope_path: \".{name}/skills\"\n  \
                user_scope_path: \".{name}/skills\"\n  \
                skill_filename: SKILL.md\n",
        ),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_harness_yaml() {
        let project_root = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_harness");
        let _ = fs::remove_dir_all(&project_root);

        fs::create_dir_all(&project_root).unwrap();

        scaffold_harness(&project_root, "my-custom").unwrap();

        let harness_file = project_root.join("harnesses/my-custom.yaml");
        assert!(harness_file.exists());

        let content = fs::read_to_string(&harness_file).unwrap();
        assert!(content.contains("id: my-custom"));
        assert!(content.contains("name: my-custom"));
        assert!(content.contains("capabilities:"));
        assert!(content.contains("paths:"));

        // Verify placeholder values are present
        assert!(content.contains("supports_subagent: false"));
        assert!(content.contains("project_scope_path:"));

        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn scaffold_harness_creates_harnesses_dir() {
        let project_root = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_harness_dir");
        let _ = fs::remove_dir_all(&project_root);

        fs::create_dir_all(&project_root).unwrap();

        scaffold_harness(&project_root, "test-harness").unwrap();

        assert!(project_root.join("harnesses").is_dir());

        let _ = fs::remove_dir_all(&project_root);
    }
}

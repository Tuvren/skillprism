use std::fs;
use std::io;
use std::path::Path;

/// Scaffolds a new skill directory with a starter skill.yaml and SKILL.md.j2 template.
pub fn scaffold_skill(project_root: &Path, name: &str, _targets: &[String]) -> io::Result<()> {
    let skill_dir = project_root.join("skills").join(name);
    fs::create_dir_all(&skill_dir)?;

    fs::write(
        skill_dir.join("skill.yaml"),
        format!("name: {name}\ndescription: A new skill\n"),
    )?;

    fs::write(
        skill_dir.join("SKILL.md.j2"),
        format!("# {name}\n\n{{{{ skill_name }}}}\n"),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_skill_files() {
        let project_root = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_skill");
        let _ = fs::remove_dir_all(&project_root);

        fs::create_dir_all(project_root.join("skills")).unwrap();
        fs::write(
            project_root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(&project_root, "my-skill", &[]).unwrap();

        let skill_dir = project_root.join("skills/my-skill");
        assert!(skill_dir.join("skill.yaml").exists());
        assert!(skill_dir.join("SKILL.md.j2").exists());

        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(yaml.contains("my-skill"));

        let _ = fs::remove_dir_all(&project_root);
    }
}

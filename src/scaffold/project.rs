use std::fs;
use std::io;
use std::path::Path;

pub fn scaffold_project(dir: &Path, name: &str) -> io::Result<()> {
    fs::create_dir_all(dir)?;

    fs::write(
        dir.join("skillprism.yaml"),
        format!("name: {name}\nharnesses:\n  - claude\n  - opencode\nskills_dir: skills\n"),
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

        scaffold_project(&dir, "test-project").unwrap();
        assert!(dir.join("skillprism.yaml").exists());
        assert!(dir.join("skills").is_dir());
        assert!(dir.join("harnesses").is_dir());

        let content = fs::read_to_string(dir.join("skillprism.yaml")).unwrap();
        assert!(content.contains("test-project"));

        let _ = fs::remove_dir_all(&dir);
    }
}

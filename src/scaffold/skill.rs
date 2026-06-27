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

use std::fs;
use std::io;
use std::path::Path;

/// Scaffolds a new skill directory with a starter skill.yaml, SKILL.md.j2 template,
/// and standard asset directories (references/, scripts/).
pub fn scaffold_skill(project_root: &Path, name: &str, _harnesses: &[String]) -> io::Result<()> {
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

    fs::create_dir_all(skill_dir.join("references"))?;
    fs::create_dir_all(skill_dir.join("scripts"))?;

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
        assert!(skill_dir.join("references").is_dir());
        assert!(skill_dir.join("scripts").is_dir());

        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(yaml.contains("my-skill"));

        let _ = fs::remove_dir_all(&project_root);
    }
}

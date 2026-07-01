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

/// Scaffolds a new skill directory with a starter skill.yaml, SKILL.md template,
/// and standard asset directories (references/, scripts/).
pub fn scaffold_skill(project_root: &Path, name: &str, _harnesses: &[String]) -> io::Result<()> {
    let skill_dir = project_root.join("skills").join(name);
    fs::create_dir_all(&skill_dir)?;

    fs::write(
        skill_dir.join("skill.yaml"),
        format!(
            "name: {name}\n\
             description: A new skill\n\
             # Anything under `variables:` is available in SKILL.md as {{{{ variable_name }}}}.\n\
             # skill.yaml fields like `license`, `when_to_use`, and `metadata` are available\n\
             # too, under their own name — see .constitution/tech-spec/contracts/skill-schema.json\n\
             # for the full list.\n\
             variables:\n  \
             greeting: Hello from {name}\n"
        ),
    )?;

    // SKILL.md (not SKILL.md.j2) so editors apply Markdown syntax highlighting; it's
    // still a MiniJinja template underneath — rename to SKILL.md.j2 if you'd rather
    // have the extension say so explicitly. Both are accepted, never both at once.
    fs::write(
        skill_dir.join("SKILL.md"),
        "# {{ skill_name }}\n\n\
         {{ skill_description }}\n\n\
         {{ greeting }}\n\n\
         Built for {{ harness.name }} ({{ harness.id }}).\n",
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
        assert!(skill_dir.join("SKILL.md").exists());
        assert!(skill_dir.join("references").is_dir());
        assert!(skill_dir.join("scripts").is_dir());

        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(yaml.contains("my-skill"));

        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn scaffold_skill_contains_variable_refs() {
        let project_root = std::env::temp_dir()
            .join("skillprism_test")
            .join("scaffold_skill_variable_refs");
        let _ = fs::remove_dir_all(&project_root);

        fs::create_dir_all(project_root.join("skills")).unwrap();
        fs::write(
            project_root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(&project_root, "my-skill", &[]).unwrap();

        let skill_dir = project_root.join("skills/my-skill");

        // skill.yaml must round-trip a custom variable, matching the teaching shape of
        // `scaffold_project`'s sample skill — both entry points should demonstrate the
        // same skill.yaml <-> SKILL.md relationship.
        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(yaml.contains("variables:"));
        assert!(yaml.contains("greeting:"));

        let template = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(template.contains("{{ skill_name }}"));
        assert!(template.contains("{{ skill_description }}"));
        assert!(template.contains("{{ greeting }}"));
        assert!(template.contains("{{ harness.id }}"));
        assert!(template.contains("{{ harness.name }}"));

        let _ = fs::remove_dir_all(&project_root);
    }
}

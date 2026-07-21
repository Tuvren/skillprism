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

/// Scaffolds a new skill directory with a starter skill.yaml, a spec-compliant
/// SKILL.md template (YAML frontmatter + body), and standard asset directories
/// (references/, scripts/).
pub fn scaffold_skill(project_root: &Path, name: &str) -> io::Result<()> {
    let skill_dir = project_root.join("skills").join(name);
    fs::create_dir_all(&skill_dir)?;

    fs::write(
        skill_dir.join("skill.yaml"),
        format!(
            "name: {name}\n\
             description: >-\n  \
             TODO: Describe what this skill does AND when to use it. Include trigger\n  \
             keywords so agents can match this skill to relevant tasks.\n\
             # Optional fields — uncomment as needed:\n\
             # license: Apache-2.0\n\
             # compatibility: Requires git and access to the internet\n\
             # variables:        # custom template values, available in SKILL.md as {{{{ name }}}}\n\
             #   greeting: Hello from {name}\n"
        ),
    )?;

    // SKILL.md (not SKILL.md.j2) so editors apply Markdown syntax highlighting; it's
    // still a MiniJinja template underneath — rename to SKILL.md.j2 if you'd rather
    // have the extension say so explicitly. Both are accepted, never both at once.
    //
    // The YAML frontmatter (name/description) is REQUIRED by the Agent Skills spec —
    // without it no client can discover the skill. skillprism renders it once per
    // harness from the skill.yaml fields above.
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\n\
         name: {{ skill_name }}\n\
         description: {{ skill_description }}\n\
         ---\n\n\
         # {{ skill_name }}\n\n\
         {{ skill_description }}\n",
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
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::create_dir_all(project_root.join("skills")).unwrap();
        fs::write(
            project_root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(project_root, "my-skill").unwrap();

        let skill_dir = project_root.join("skills/my-skill");
        assert!(skill_dir.join("skill.yaml").exists());
        assert!(skill_dir.join("SKILL.md").exists());
        assert!(skill_dir.join("references").is_dir());
        assert!(skill_dir.join("scripts").is_dir());

        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(yaml.contains("my-skill"));
    }

    #[test]
    fn scaffold_skill_emits_spec_compliant_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::create_dir_all(project_root.join("skills")).unwrap();
        fs::write(
            project_root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(project_root, "my-skill").unwrap();

        let skill_dir = project_root.join("skills/my-skill");

        // The Agent Skills spec requires YAML frontmatter (name + description) at the
        // top of SKILL.md — without it no client can discover the skill. The scaffold
        // must emit it so a brand-new project's first `skillprism build` produces a
        // valid, discoverable skill.
        let template = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(
            template.starts_with("---\n"),
            "SKILL.md must start with YAML frontmatter, got: {template:?}"
        );
        assert!(
            template.contains("name: {{ skill_name }}"),
            "frontmatter must render the skill name"
        );
        assert!(
            template.contains("description: {{ skill_description }}"),
            "frontmatter must render the skill description"
        );

        // Built-ins are always available and always render — the scaffold should
        // demonstrate them so the template builds successfully with zero edits beyond
        // the description.
        assert!(template.contains("{{ skill_name }}"));
        assert!(template.contains("{{ skill_description }}"));
    }

    #[test]
    fn scaffold_skill_variables_are_optional_comment() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::create_dir_all(project_root.join("skills")).unwrap();
        fs::write(
            project_root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(project_root, "my-skill").unwrap();

        let skill_dir = project_root.join("skills/my-skill");

        // variables: must be a commented optional example, NOT a real field — most
        // skills don't need custom variables, and the scaffold should build cleanly
        // without forcing the author to understand them first.
        let yaml = fs::read_to_string(skill_dir.join("skill.yaml")).unwrap();
        assert!(
            !yaml.contains("variables:\n  greeting"),
            "variables should be commented out, not active by default"
        );
        assert!(
            yaml.contains("# variables:"),
            "variables should appear as a commented example"
        );

        // The template must NOT reference {{ greeting }} — it's not defined by default.
        let template = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
        assert!(
            !template.contains("{{ greeting }}"),
            "template should not reference an undefined variable"
        );
    }
}

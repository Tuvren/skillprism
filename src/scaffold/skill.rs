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
/// with deletable placeholder content (references/, scripts/, assets/).
pub fn scaffold_skill(project_root: &Path, name: &str) -> io::Result<()> {
    let config_path = project_root.join("skillprism.yaml");
    let rel_skills_dir = if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: crate::types::ProjectConfig = yaml_serde::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse {}: {e}", config_path.display()),
            )
        })?;
        config.skills_dir
    } else {
        std::path::PathBuf::from("skills")
    };

    if rel_skills_dir.is_absolute()
        || rel_skills_dir
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "skills_dir `{}` must be a relative path without '..' components",
                rel_skills_dir.display()
            ),
        ));
    }

    let target_dir = project_root.join(rel_skills_dir).join(name);
    fs::create_dir_all(&target_dir)?;

    fs::write(
        target_dir.join("skill.yaml"),
        format!(
            "skillprism: '1'\n\
             name: {name}\n\
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
        target_dir.join("SKILL.md"),
        "---\n\
         name: {{ skill_name }}\n\
         description: {{ skill_description }}\n\
         ---\n\n\
         # {{ skill_name }}\n\n\
         {{ skill_description }}\n",
    )?;

    let refs_dir = target_dir.join("references");
    fs::create_dir_all(&refs_dir)?;
    fs::write(
        refs_dir.join("example.md"),
        "# Reference Documentation\n\nAdd reference material here (e.g. domain docs, API specs, guidelines).\nLink to this file from SKILL.md.\n\nDelete this placeholder file if unused.\n",
    )?;

    let scripts_dir = target_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;
    fs::write(
        scripts_dir.join("example.sh"),
        "#!/usr/bin/env bash\n# Executable script placeholder.\n# Place scripts here that handle repetitive or complex tasks.\n# Delete this placeholder file if unused.\n",
    )?;

    let assets_dir = target_dir.join("assets");
    fs::create_dir_all(&assets_dir)?;
    fs::write(
        assets_dir.join("example.txt"),
        "# Static Assets\n\nPlace static assets (templates, fonts, images) here.\nDelete this placeholder file if unused.\n",
    )?;

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
        assert!(yaml.contains("skillprism: '1'"));
        assert!(yaml.contains("my-skill"));

        assert!(skill_dir.join("references/example.md").exists());
        assert!(skill_dir.join("scripts/example.sh").exists());
        assert!(skill_dir.join("assets/example.txt").exists());
    }

    #[test]
    fn scaffold_skill_respects_custom_skills_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::write(
            project_root.join("skillprism.yaml"),
            "skills_dir: custom_skills\nharnesses:\n  - claude\n",
        )
        .unwrap();

        scaffold_skill(project_root, "custom-dir-skill").unwrap();

        let skill_dir = project_root.join("custom_skills/custom-dir-skill");
        assert!(skill_dir.join("skill.yaml").exists());
        assert!(skill_dir.join("SKILL.md").exists());
    }

    #[test]
    fn scaffold_skill_rejects_unsafe_skills_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::write(
            project_root.join("skillprism.yaml"),
            "skills_dir: ../outside\nharnesses:\n  - claude\n",
        )
        .unwrap();

        let res = scaffold_skill(project_root, "bad-skill");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn scaffold_skill_fails_on_malformed_config() {
        let dir = tempfile::tempdir().unwrap();
        let project_root = dir.path();

        fs::write(
            project_root.join("skillprism.yaml"),
            "skills_dir: [invalid\n",
        )
        .unwrap();

        let res = scaffold_skill(project_root, "bad-skill");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
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

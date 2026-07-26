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

/// Scaffolds a new skillprism project with directory structure, config, sample skill,
/// and a project-level .gitignore so generated harness output isn't committed.
///
/// When `harnesses` is empty, defaults to `["claude", "opencode"]`.
pub fn scaffold_project(dir: &Path, name: &str, harnesses: &[String]) -> io::Result<()> {
    fs::create_dir_all(dir)?;

    let yaml_lines: Vec<String> = harnesses.iter().map(|h| format!("  - {h}")).collect();

    fs::write(
        dir.join("skillprism.yaml"),
        format!(
            "harnesses:\n{harness_list}\nskills_dir: skills\n",
            harness_list = yaml_lines.join("\n")
        ),
    )?;

    // Sample skill follows Anthropic skill-creator layout with skillprism: '1'
    crate::scaffold::skill::scaffold_skill(dir, "sample")?;

    // Keep generated harness output out of version control — these directories are
    // written by `skillprism build` and shouldn't be committed alongside source.
    fs::write(
        dir.join(".gitignore"),
        ".claude/\n.opencode/\n.agents/\n.factory/\n.pi/\ndist/\n",
    )?;

    fs::write(
        dir.join("README.md"),
        format!(
            "# {name}\n\nAgent Skills project managed with [skillprism](https://tuvren.github.io/skillprism/).\n\n## Usage\n\n```bash\nskillprism build       # compile skills to all configured harnesses\nskillprism validate    # check templates without writing output\nskillprism init skill <name>   # add a new skill\n```\n\nSee `skills/` for your skills and `skillprism.yaml` for project configuration.\n"
        ),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_creates_expected_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let harnesses = vec!["claude".to_string(), "opencode".to_string()];
        scaffold_project(dir, "test-project", &harnesses).unwrap();
        assert!(dir.join("skillprism.yaml").exists());
        assert!(dir.join("skills").is_dir());
        assert!(!dir.join("harnesses").exists());
        assert!(dir.join("skills/sample/skill.yaml").exists());
        assert!(dir.join("skills/sample/SKILL.md").exists());
        assert!(dir.join(".gitignore").exists());
        assert!(dir.join("README.md").exists());

        let content = fs::read_to_string(dir.join("skillprism.yaml")).unwrap();
        let model = crate::loader::ProjectLoader::load(dir).unwrap();
        assert_eq!(model.skills.len(), 1);
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.contains(&"  - claude"));
        assert!(lines.contains(&"  - opencode"));
    }

    #[test]
    fn scaffold_with_custom_harnesses() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let h = vec!["claude".to_string(), "codex".to_string(), "pi".to_string()];
        scaffold_project(dir, "custom", &h).unwrap();

        let content = fs::read_to_string(dir.join("skillprism.yaml")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.contains(&"  - claude"));
        assert!(lines.contains(&"  - codex"));
        assert!(lines.contains(&"  - pi"));
    }

    #[test]
    fn scaffold_sample_skill_contains_frontmatter_and_builtins() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        scaffold_project(dir, "test", &["claude".to_string()]).unwrap();

        let template = fs::read_to_string(dir.join("skills/sample/SKILL.md")).unwrap();
        // The spec requires frontmatter — the scaffold must emit it.
        assert!(
            template.starts_with("---\n"),
            "sample SKILL.md must start with YAML frontmatter, got: {template:?}"
        );
        assert!(
            template.contains("name: {{ skill_name }}"),
            "template must render skill_name in frontmatter"
        );
        assert!(
            template.contains("description: {{ skill_description }}"),
            "template must render skill_description in frontmatter"
        );
        assert!(
            template.contains("{{ skill_name }}"),
            "template must reference skill_name"
        );
        assert!(
            template.contains("{{ skill_description }}"),
            "template must reference skill_description"
        );

        let skill_yaml = fs::read_to_string(dir.join("skills/sample/skill.yaml")).unwrap();
        assert!(
            skill_yaml.contains("skillprism: '1'"),
            "skill.yaml must declare skillprism: '1'"
        );
        assert!(
            skill_yaml.contains("sample"),
            "skill.yaml must contain the skill name"
        );

        assert!(dir.join("skills/sample/references/example.md").exists());
        assert!(dir.join("skills/sample/scripts/example.sh").exists());
        assert!(dir.join("skills/sample/assets/example.txt").exists());
    }

    #[test]
    fn scaffold_writes_gitignore_for_harness_output() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        scaffold_project(dir, "test", &["claude".to_string()]).unwrap();

        let gitignore = fs::read_to_string(dir.join(".gitignore")).unwrap();
        assert!(gitignore.contains(".claude/"));
        assert!(gitignore.contains(".opencode/"));
        assert!(gitignore.contains("dist/"));
    }
}

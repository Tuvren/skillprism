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

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::{HarnessOverride, ProjectConfig, ProjectError, ProjectModel, SkillModel};

/// Loads a skillprism project from disk, discovering all skills.
pub struct ProjectLoader;

impl ProjectLoader {
    /// Loads the project configuration and discovers all skills from the given root.
    pub fn load(project_root: &Path) -> Result<ProjectModel, ProjectError> {
        let config_path = project_root.join("skillprism.yaml");
        let config = Self::load_config(&config_path)?;
        let skills = Self::discover_skills(project_root, &config.skills_dir)?;

        Ok(ProjectModel {
            config,
            skills,
            project_root: project_root.to_path_buf(),
        })
    }

    fn load_config(path: &Path) -> Result<ProjectConfig, ProjectError> {
        let content = std::fs::read_to_string(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ProjectError::ConfigNotFound {
                path: path.to_string_lossy().to_string(),
            },
            _ => ProjectError::ConfigRead {
                path: path.to_string_lossy().to_string(),
                source: e,
            },
        })?;

        yaml_serde::from_str(&content).map_err(|e| {
            let loc = e.location().map_or(0, |l| l.line());
            ProjectError::YamlParse {
                path: path.to_string_lossy().to_string(),
                line: loc,
                message: e.to_string(),
            }
        })
    }

    fn discover_skills(
        project_root: &Path,
        skills_dir: &Path,
    ) -> Result<Vec<SkillModel>, ProjectError> {
        let skills_path = project_root.join(skills_dir);
        if !skills_path.exists() {
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();
        Self::walk_directory(&skills_path, &BTreeMap::new(), &mut skills)?;
        Ok(skills)
    }

    fn walk_directory(
        dir: &Path,
        parent_variables: &BTreeMap<String, yaml_serde::Value>,
        skills: &mut Vec<SkillModel>,
    ) -> Result<(), ProjectError> {
        let config_path = dir.join("skill.yaml");
        let local_variables = if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).map_err(|e| ProjectError::ConfigRead {
                    path: config_path.to_string_lossy().to_string(),
                    source: e,
                })?;

            let skill_config: SkillYamlRaw = yaml_serde::from_str(&content).map_err(|e| {
                let loc = e.location().map_or(0, |l| l.line());
                ProjectError::YamlParse {
                    path: config_path.to_string_lossy().to_string(),
                    line: loc,
                    message: e.to_string(),
                }
            })?;

            skill_config.variables.unwrap_or_default()
        } else {
            BTreeMap::new()
        };

        let merged = merge_variables(parent_variables, &local_variables);

        for entry in read_dir_entries(dir)? {
            let path = entry.path();

            if path.is_dir() {
                if let Some(template_path) = Self::find_template_path(&path)? {
                    let skill = Self::load_skill(&path, &template_path, &merged)?;
                    skills.push(skill);
                } else if path.join("skill.yaml").exists() || Self::has_skill_dirs(&path) {
                    Self::walk_directory(&path, &merged, skills)?;
                }
            }
        }

        Ok(())
    }

    /// A skill's template may be authored as `SKILL.md.j2` or as bare `SKILL.md` — the
    /// latter exists purely so editors apply Markdown syntax highlighting to a file that
    /// is otherwise identical (still `MiniJinja` syntax, still rendered the same way).
    /// Having both in one directory is rejected rather than silently preferring one.
    pub(crate) fn find_template_path(
        dir: &Path,
    ) -> Result<Option<std::path::PathBuf>, ProjectError> {
        let j2 = dir.join("SKILL.md.j2");
        let bare = dir.join("SKILL.md");
        match (j2.exists(), bare.exists()) {
            (true, true) => Err(ProjectError::AmbiguousTemplate {
                dir: dir.to_string_lossy().to_string(),
            }),
            (true, false) => Ok(Some(j2)),
            (false, true) => Ok(Some(bare)),
            (false, false) => Ok(None),
        }
    }

    fn has_skill_dirs(dir: &Path) -> bool {
        read_dir_entries(dir).is_ok_and(|entries| {
            entries.iter().any(|e| {
                let p = e.path();
                p.is_dir() && (p.join("SKILL.md.j2").exists() || p.join("SKILL.md").exists())
            })
        })
    }

    fn load_skill(
        dir: &Path,
        template_path: &Path,
        merged_variables: &BTreeMap<String, yaml_serde::Value>,
    ) -> Result<SkillModel, ProjectError> {
        let config_path = dir.join("skill.yaml");
        let directory_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut skill = SkillModel {
            name: directory_name.clone(),
            directory_name,
            description: String::new(),
            version: None,
            license: None,
            compatibility: None,
            metadata: BTreeMap::new(),
            allowed_tools: None,
            when_to_use: None,
            argument_hint: None,
            arguments: None,
            disable_model_invocation: None,
            user_invocable: None,
            disallowed_tools: None,
            model_override: None,
            effort: None,
            context_fork: false,
            agent: None,
            hooks: None,
            activation_paths: None,
            shell: None,
            required_capabilities: Vec::new(),
            variables: merged_variables.clone(),
            template_path: template_path.to_path_buf(),
            asset_dirs: Vec::new(),
            harness_overrides: BTreeMap::new(),
        };

        if config_path.exists() {
            let content =
                std::fs::read_to_string(&config_path).map_err(|e| ProjectError::ConfigRead {
                    path: config_path.to_string_lossy().to_string(),
                    source: e,
                })?;

            let skill_config: SkillYamlRaw = yaml_serde::from_str(&content).map_err(|e| {
                let loc = e.location().map_or(0, |l| l.line());
                ProjectError::YamlParse {
                    path: config_path.to_string_lossy().to_string(),
                    line: loc,
                    message: e.to_string(),
                }
            })?;

            if let Some(name) = skill_config.name {
                skill.name = name;
            }
            skill.description = skill_config.description.unwrap_or_default();
            skill.version = skill_config.version;
            skill.license = skill_config.license;
            skill.compatibility = skill_config.compatibility;
            skill.metadata = skill_config.metadata.unwrap_or_default();
            skill.allowed_tools = skill_config.allowed_tools;
            skill.when_to_use = skill_config.when_to_use;
            skill.argument_hint = skill_config.argument_hint;
            skill.arguments = skill_config.arguments;
            skill.disable_model_invocation = skill_config.disable_model_invocation;
            skill.user_invocable = skill_config.user_invocable;
            skill.disallowed_tools = skill_config.disallowed_tools;
            skill.model_override = skill_config.model;
            skill.effort = skill_config.effort;
            skill.context_fork = skill_config.context.is_some_and(|c| c == "fork");
            skill.agent = skill_config.agent;
            skill.hooks = skill_config.hooks;
            skill.activation_paths = skill_config.paths;
            skill.shell = skill_config.shell;
            skill.required_capabilities = skill_config.required_capabilities.unwrap_or_default();

            if let Some(vars) = skill_config.variables {
                for (k, v) in vars {
                    skill.variables.insert(k, v);
                }
            }

            if let Some(harnesses) = skill_config.harnesses {
                skill.harness_overrides = harnesses
                    .into_iter()
                    .map(|(harness_id, raw)| {
                        (
                            harness_id,
                            HarnessOverride {
                                variables: raw.variables,
                                macros: raw.macros,
                            },
                        )
                    })
                    .collect();
            }
        }

        skill.asset_dirs = Self::discover_asset_dirs(dir)?;

        Ok(skill)
    }

    /// Every direct subdirectory of a skill's own directory is an asset directory to
    /// copy verbatim, regardless of name (`references/`, `scripts/`, or anything else
    /// an author uses) — `walk_directory` never recurses into a skill's own directory
    /// looking for nested skills/groups once SKILL.md.j2 has been found, so nothing
    /// here can be mistaken for one. Dot-directories (`.venv/`, `.git/`,
    /// `.ipynb_checkpoints/`, ...) are excluded — they're tooling/VCS artifacts, never
    /// content an author intends to ship alongside the skill.
    pub(crate) fn discover_asset_dirs(dir: &Path) -> Result<Vec<std::path::PathBuf>, ProjectError> {
        let mut asset_dirs = read_dir_entries(dir)?
            .into_iter()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .filter(|path| {
                !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
            })
            .collect::<Vec<_>>();
        asset_dirs.sort();
        Ok(asset_dirs)
    }
}

// Crate-internal free-function aliases for the distribution commands. The
// `loader` module is private (declared `mod loader;`), so these are effectively
// crate-scoped; they stay `pub` rather than `pub(crate)` because the crate
// enforces `deny(clippy::nursery)`, whose `redundant_pub_crate` lint rejects
// `pub(crate)` inside a private module.
pub fn find_template_path(dir: &Path) -> Result<Option<std::path::PathBuf>, ProjectError> {
    ProjectLoader::find_template_path(dir)
}

pub fn discover_asset_dirs(dir: &Path) -> Result<Vec<std::path::PathBuf>, ProjectError> {
    ProjectLoader::discover_asset_dirs(dir)
}

fn read_dir_entries(dir: &Path) -> Result<Vec<std::fs::DirEntry>, ProjectError> {
    let entries: Result<Vec<_>, _> = std::fs::read_dir(dir)
        .map_err(|e| ProjectError::ConfigRead {
            path: dir.to_string_lossy().to_string(),
            source: e,
        })?
        .collect();
    entries.map_err(|e| ProjectError::ConfigRead {
        path: dir.to_string_lossy().to_string(),
        source: e,
    })
}

fn merge_variables(
    parent: &BTreeMap<String, yaml_serde::Value>,
    child: &BTreeMap<String, yaml_serde::Value>,
) -> BTreeMap<String, yaml_serde::Value> {
    let mut merged = parent.clone();
    for (k, v) in child {
        merged.insert(k.clone(), v.clone());
    }
    merged
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SkillYamlRaw {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
    license: Option<String>,
    compatibility: Option<String>,
    metadata: Option<BTreeMap<String, String>>,
    #[serde(rename = "allowed-tools")]
    allowed_tools: Option<String>,
    when_to_use: Option<String>,
    #[serde(rename = "argument-hint")]
    argument_hint: Option<String>,
    arguments: Option<Vec<String>>,
    #[serde(rename = "disable-model-invocation")]
    disable_model_invocation: Option<bool>,
    #[serde(rename = "user-invocable")]
    user_invocable: Option<bool>,
    #[serde(rename = "disallowed-tools")]
    disallowed_tools: Option<Vec<String>>,
    model: Option<String>,
    effort: Option<String>,
    context: Option<String>,
    agent: Option<String>,
    hooks: Option<BTreeMap<String, yaml_serde::Value>>,
    paths: Option<Vec<String>>,
    shell: Option<String>,
    #[serde(rename = "required-capabilities")]
    required_capabilities: Option<Vec<String>>,
    variables: Option<BTreeMap<String, yaml_serde::Value>>,
    harnesses: Option<BTreeMap<String, HarnessOverrideRaw>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HarnessOverrideRaw {
    #[serde(default)]
    variables: BTreeMap<String, yaml_serde::Value>,
    #[serde(default)]
    macros: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn load_valid_project() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill/references")).unwrap();

        fs::write(
            root.join("skillprism.yaml"),
            "harnesses:\n  - claude\n  - opencode\nskills_dir: skills\n",
        )
        .unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: A test skill\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        assert_eq!(
            model.config.harnesses,
            vec!["claude".to_string(), "opencode".to_string()]
        );
        assert_eq!(model.skills.len(), 1);
        assert_eq!(model.skills[0].name, "my-skill");
        assert_eq!(model.skills[0].description, "A test skill");
    }

    #[test]
    fn dot_directories_excluded_from_asset_dirs() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill/references")).unwrap();
        fs::create_dir_all(root.join("skills/my-skill/.venv")).unwrap();
        fs::create_dir_all(root.join("skills/my-skill/.git")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: A test skill\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        assert_eq!(model.skills.len(), 1);
        let asset_names: Vec<String> = model.skills[0]
            .asset_dirs
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .collect();
        assert_eq!(asset_names, vec!["references".to_string()]);
    }

    #[test]
    fn load_valid_project_with_bare_skill_md() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: A test skill\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md"), "# {{ name }}\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        assert_eq!(model.skills.len(), 1);
        assert_eq!(
            model.skills[0].template_path,
            root.join("skills/my-skill/SKILL.md")
        );
    }

    #[test]
    fn ambiguous_template_both_extensions_present() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: A test skill\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md"), "# {{ name }}\n").unwrap();

        let result = ProjectLoader::load(root);
        match result.unwrap_err() {
            ProjectError::AmbiguousTemplate { dir } => {
                assert!(dir.contains("my-skill"));
            }
            e => panic!("expected AmbiguousTemplate error, got {e:?}"),
        }
    }

    #[test]
    fn missing_skillprism_yaml() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        let result = ProjectLoader::load(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ConfigNotFound { .. } => {}
            _ => panic!("expected ConfigNotFound error"),
        }
    }

    #[test]
    fn invalid_yaml_syntax() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses: [invalid\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::YamlParse { .. } => {}
            e => panic!("expected YamlParse error, got {e:?}"),
        }
    }

    #[test]
    fn invalid_skill_yaml() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/bad-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(root.join("skills/bad-skill/skill.yaml"), "name: 'broken\n").unwrap();
        fs::write(root.join("skills/bad-skill/SKILL.md.j2"), "content\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::YamlParse { .. } => {}
            e => panic!("expected YamlParse error, got {e:?}"),
        }
    }

    #[test]
    fn typo_in_harness_override_field_rejected() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: A test skill\nharnesses:\n  claude:\n    variabels:\n      greeting: hi\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(
            result.is_err(),
            "a typo'd override field should not be silently dropped"
        );
        match result.unwrap_err() {
            ProjectError::YamlParse { .. } => {}
            e => panic!("expected YamlParse error, got {e:?}"),
        }
    }

    #[test]
    fn harness_overrides_parsed_from_skill_yaml() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\n\
             description: test\n\
             variables:\n  \
             greeting: hello\n\
             harnesses:\n  \
             claude:\n    \
             variables:\n      \
             greeting: hello-claude\n    \
             macros:\n      \
             extra_note: Claude-only note\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# test\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        let skill = &model.skills[0];

        // The top-level default is untouched by the override.
        assert_eq!(
            skill.variables.get("greeting").and_then(|v| v.as_str()),
            Some("hello")
        );

        let claude_override = skill.harness_overrides.get("claude").unwrap();
        assert_eq!(
            claude_override
                .variables
                .get("greeting")
                .and_then(|v| v.as_str()),
            Some("hello-claude")
        );
        assert_eq!(
            claude_override.macros.get("extra_note").map(String::as_str),
            Some("Claude-only note")
        );
    }

    #[test]
    fn group_variable_merge_child_wins() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/group/child")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();

        fs::write(
            root.join("skills/group/skill.yaml"),
            "variables:\n  theme: dark\n  lang: en\n",
        )
        .unwrap();

        fs::write(
            root.join("skills/group/child/skill.yaml"),
            "variables:\n  lang: fr\n",
        )
        .unwrap();
        fs::write(root.join("skills/group/child/SKILL.md.j2"), "# test\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        assert_eq!(model.skills.len(), 1);

        let vars = &model.skills[0].variables;

        let theme = vars.get("theme").and_then(|v| v.as_str()).unwrap();
        assert_eq!(theme, "dark");

        let lang = vars.get("lang").and_then(|v| v.as_str()).unwrap();
        assert_eq!(lang, "fr");
    }
}

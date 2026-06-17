#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::{ProjectConfig, ProjectError, ProjectModel, SkillModel};

pub struct ProjectLoader;

impl ProjectLoader {
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
                let template_path = path.join("SKILL.md.j2");
                if template_path.exists() {
                    let skill = Self::load_skill(&path, &merged)?;
                    skills.push(skill);
                } else if path.join("skill.yaml").exists() || Self::has_skill_dirs(&path) {
                    Self::walk_directory(&path, &merged, skills)?;
                }
            }
        }

        Ok(())
    }

    fn has_skill_dirs(dir: &Path) -> bool {
        read_dir_entries(dir).is_ok_and(|entries| {
            entries
                .iter()
                .any(|e| e.path().is_dir() && e.path().join("SKILL.md.j2").exists())
        })
    }

    fn load_skill(
        dir: &Path,
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
            template_path: dir.join("SKILL.md.j2"),
            asset_dirs: Vec::new(),
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
        }

        let refs_dir = dir.join("references");
        if refs_dir.exists() {
            skill.asset_dirs.push(refs_dir);
        }
        let scripts_dir = dir.join("scripts");
        if scripts_dir.exists() {
            skill.asset_dirs.push(scripts_dir);
        }

        Ok(skill)
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("skillprism_test").join(name);
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn load_valid_project() {
        let root = setup_test_dir("valid_project");
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

        let model = ProjectLoader::load(&root).unwrap();
        assert_eq!(
            model.config.harnesses,
            vec!["claude".to_string(), "opencode".to_string()]
        );
        assert_eq!(model.skills.len(), 1);
        assert_eq!(model.skills[0].name, "my-skill");
        assert_eq!(model.skills[0].description, "A test skill");
    }

    #[test]
    fn missing_skillprism_yaml() {
        let root = setup_test_dir("missing_config");
        fs::create_dir_all(&root).unwrap();

        let result = ProjectLoader::load(&root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ConfigNotFound { .. } => {}
            _ => panic!("expected ConfigNotFound error"),
        }
    }

    #[test]
    fn invalid_yaml_syntax() {
        let root = setup_test_dir("invalid_yaml");
        fs::create_dir_all(&root).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses: [invalid\n").unwrap();

        let result = ProjectLoader::load(&root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::YamlParse { .. } => {}
            e => panic!("expected YamlParse error, got {e:?}"),
        }
    }

    #[test]
    fn invalid_skill_yaml() {
        let root = setup_test_dir("invalid_skill_yaml");
        fs::create_dir_all(root.join("skills/bad-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(root.join("skills/bad-skill/skill.yaml"), "name: 'broken\n").unwrap();
        fs::write(root.join("skills/bad-skill/SKILL.md.j2"), "content\n").unwrap();

        let result = ProjectLoader::load(&root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::YamlParse { .. } => {}
            e => panic!("expected YamlParse error, got {e:?}"),
        }
    }

    #[test]
    fn group_variable_merge_child_wins() {
        let root = setup_test_dir("var_merge");
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

        let model = ProjectLoader::load(&root).unwrap();
        assert_eq!(model.skills.len(), 1);

        let vars = &model.skills[0].variables;

        let theme = vars.get("theme").and_then(|v| v.as_str()).unwrap();
        assert_eq!(theme, "dark");

        let lang = vars.get("lang").and_then(|v| v.as_str()).unwrap();
        assert_eq!(lang, "fr");
    }
}

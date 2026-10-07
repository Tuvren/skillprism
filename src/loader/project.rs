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

        super::yaml::deserialize(&content, path)
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

            let skill_config = parse_skill_config(&content, &config_path)?;
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
            Self::apply_skill_config(&config_path, &mut skill)?;
        }

        skill.asset_dirs = Self::discover_asset_dirs(dir)?;

        Ok(skill)
    }

    fn apply_skill_config(config_path: &Path, skill: &mut SkillModel) -> Result<(), ProjectError> {
        let content =
            std::fs::read_to_string(config_path).map_err(|e| ProjectError::ConfigRead {
                path: config_path.to_string_lossy().to_string(),
                source: e,
            })?;

        let skill_config = parse_skill_config(&content, config_path)?;

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

        if let Some(overrides) = skill_config.overrides {
            skill.harness_overrides = overrides
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

        Ok(())
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

fn parse_skill_config(content: &str, path: &Path) -> Result<SkillYamlRaw, ProjectError> {
    let raw = super::yaml::deserialize(content, path)?;
    validate_skillprism_manifest_version(&raw, path, content)?;
    super::yaml::from_value(raw, content, path)
}

fn validate_skillprism_manifest_version(
    raw: &yaml_serde::Value,
    path: &Path,
    content: &str,
) -> Result<(), ProjectError> {
    let yaml_serde::Value::Mapping(map) = raw else {
        return Err(ProjectError::config_schema(
            path,
            content,
            "skill.yaml must contain a top-level YAML mapping".to_owned(),
            super::yaml::root_location(content),
        ));
    };
    if map.contains_key("harnesses") {
        let message = "the `harnesses:` block in skill.yaml has been renamed to `overrides:` \
                       in skillprism 0.2.0; please update your skill.yaml to use `overrides:`";
        return Err(ProjectError::config_schema(
            path,
            content,
            message.to_owned(),
            super::yaml::legacy_harnesses_location(content),
        ));
    }
    let message = match map.get("skillprism") {
        None => {
            let message = "missing required field `skillprism`; either add `skillprism: '1'` \
                           to declare skillprism-format, or remove skill.yaml to declare plain-format.";
            return Err(ProjectError::config_schema(
                path,
                content,
                message.to_owned(),
                None,
            ));
        }
        Some(yaml_serde::Value::String(s)) if s == "1" => return Ok(()),
        Some(yaml_serde::Value::Number(n)) if n.as_i64() == Some(1) => return Ok(()),
        Some(yaml_serde::Value::String(s)) if s.is_empty() => {
            "the `skillprism:` field must not be empty".to_owned()
        }
        Some(yaml_serde::Value::String(other)) => format!(
            "unsupported `skillprism:` value `{other}`; only `skillprism: '1'` is supported"
        ),
        Some(yaml_serde::Value::Number(other)) => format!(
            "unsupported `skillprism:` value `{other}`; only `skillprism: '1'` is supported"
        ),
        Some(_) => "the `skillprism:` field must be a quoted string or integer".to_owned(),
    };
    Err(ProjectError::config_schema(
        path,
        content,
        message,
        super::yaml::manifest_version_location(content),
    ))
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillYamlRaw {
    #[allow(dead_code)]
    skillprism: Option<yaml_serde::Value>,
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
    overrides: Option<BTreeMap<String, HarnessOverrideRaw>>,
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
    fn config_diagnostics_loader_scenarios() {
        let cases = [
            (
                "skillprism.yaml",
                "name: my-skills\nharnesses: [claude]\n",
                "unknown field `name`",
                Some("line 1 column 1"),
            ),
            (
                "skills/demo/skill.yaml",
                "name: demo\ndescription: test\n",
                "missing required field `skillprism`",
                None,
            ),
            (
                "skills/demo/skill.yaml",
                "skillprism: '1'\nname: demo\nvariables: text\n",
                "variables",
                Some("line 3 column 12"),
            ),
            (
                "skillprism.yaml",
                "# project\n# broken sequence\nharnesses: [claude",
                "YAML does not parse",
                Some("line 4 column 1"),
            ),
        ];
        for (file, content, reason, location) in cases {
            let tmp = setup_test_dir();
            fs::create_dir_all(tmp.path().join("skills/demo")).unwrap();
            fs::write(tmp.path().join("skillprism.yaml"), "harnesses: [claude]\n").unwrap();
            fs::write(tmp.path().join("skills/demo/SKILL.md"), "# Demo\n").unwrap();
            fs::write(tmp.path().join(file), content).unwrap();
            let error = ProjectLoader::load(tmp.path()).unwrap_err().to_string();
            assert!(error.contains(reason), "{error}");
            assert!(error.contains(file), "{error}");
            assert!(!error.contains("Invalid YAML"), "{error}");
            if let Some(location) = location {
                assert!(error.contains(location), "{error}");
            } else {
                assert!(!error.contains("line 1"), "{error}");
            }
        }
    }

    #[test]
    fn config_diagnostics_skill_unknown_field_and_syntax_have_spans() {
        use miette::Diagnostic;

        let cases = [
            (
                "skillprism: '1'\n# metadata\nnam: demo\n",
                "unknown field `nam`",
                "nam",
            ),
            (
                "skillprism: '1'\n# metadata\nvariables: [text",
                "YAML does not parse",
                "",
            ),
        ];
        for (content, reason, text) in cases {
            let tmp = setup_test_dir();
            fs::create_dir_all(tmp.path().join("skills/demo")).unwrap();
            fs::write(tmp.path().join("skillprism.yaml"), "harnesses: [claude]\n").unwrap();
            fs::write(tmp.path().join("skills/demo/SKILL.md"), "# Demo\n").unwrap();
            fs::write(tmp.path().join("skills/demo/skill.yaml"), content).unwrap();
            let error = ProjectLoader::load(tmp.path()).unwrap_err();
            assert!(error.to_string().contains(reason), "{error}");
            assert!(error.to_string().contains("line 3 column"), "{error}");
            let label = error.labels().unwrap().next().unwrap();
            assert_eq!(&content[label.offset()..label.offset() + label.len()], text);
            assert!(error.source_code().is_some());
        }
    }

    #[test]
    fn config_diagnostics_project_wrong_type_and_defaults() {
        let tmp = setup_test_dir();
        fs::write(
            tmp.path().join("skillprism.yaml"),
            "# config\nharnesses: claude\n",
        )
        .unwrap();
        let error = ProjectLoader::load(tmp.path()).unwrap_err();
        assert!(matches!(error, ProjectError::ConfigSchema { .. }));
        let message = error.to_string();
        assert!(message.contains("harnesses"), "{message}");
        assert!(message.contains("expected a sequence"), "{message}");
        assert!(message.contains("line 2 column 12"), "{message}");

        // ProjectConfig has no required fields; preserve its defaults.
        fs::write(tmp.path().join("skillprism.yaml"), "{}\n").unwrap();
        let project = ProjectLoader::load(tmp.path()).unwrap();
        assert!(project.config.harnesses.is_empty());
        assert_eq!(project.config.skills_dir, Path::new("skills"));
    }

    #[test]
    fn config_diagnostics_version_checks_use_parser_locations() {
        use miette::Diagnostic;

        for value in ["''", "'2'", "2", "null", "false", "[]", "{}"] {
            let content = format!("# manifest\nname: demo\nskillprism: {value}\n");
            let error = parse_skill_config(&content, Path::new("skill.yaml")).unwrap_err();
            assert!(matches!(error, ProjectError::ConfigSchema { .. }));
            let message = error.to_string();
            assert!(message.contains("skillprism:"), "{message}");
            assert!(message.contains("line 3 column 13"), "{message}");
            let label = error.labels().unwrap().next().unwrap();
            assert_eq!(
                label.offset(),
                content.find(value).unwrap(),
                "{value}: {message}"
            );
        }
    }

    #[test]
    fn config_diagnostics_value_conversion_keeps_file_level_when_direct_parse_coerces() {
        use miette::Diagnostic;

        for content in [
            "skillprism: '1'\nname: 42\n",
            "skillprism: '1'\nname: 42\nvariables: text\n",
        ] {
            let error = parse_skill_config(content, Path::new("skill.yaml")).unwrap_err();
            assert!(matches!(error, ProjectError::ConfigSchema { .. }));
            let message = error.to_string();
            assert!(
                message.contains("invalid type: integer `42`, expected a string"),
                "{message}"
            );
            assert!(!message.contains("line"), "{message}");
            assert!(error.labels().unwrap().next().is_none());
        }
    }

    #[test]
    fn config_diagnostics_spans_use_bytes_for_utf8_and_crlf() {
        use miette::Diagnostic;

        for content in [
            "skillprism: '1'\nname: café\nvariables: text\n",
            "skillprism: '1'\r\nname: café\r\nvariables: text\r\n",
            "{skillprism: '1', name: café, variables: text}",
        ] {
            let error = parse_skill_config(content, Path::new("skill.yaml")).unwrap_err();
            let label = error.labels().unwrap().next().unwrap();
            assert_eq!(label.offset(), content.find("text").unwrap());
            assert_eq!(
                &content[label.offset()..label.offset() + label.len()],
                "text"
            );
        }
    }

    #[test]
    fn config_diagnostics_root_shape_and_legacy_migration_locations() {
        use miette::Diagnostic;

        for value in ["text", "[]", "null"] {
            let content = format!("# manifest\n{value}\n");
            let error = parse_skill_config(&content, Path::new("skill.yaml")).unwrap_err();
            assert!(error.to_string().contains("top-level YAML mapping"));
            assert!(error.to_string().contains("line 2 column 1"), "{error}");
            assert_eq!(
                error.labels().unwrap().next().unwrap().offset(),
                content.find(value).unwrap()
            );
        }
        let content = "skillprism: '2'\n# legacy config\nharnesses: {}\n";
        let error = parse_skill_config(content, Path::new("skill.yaml")).unwrap_err();
        // The migration check retains its wording and precedence over version errors.
        assert!(error.to_string().contains("renamed to `overrides:`"));
        assert!(error.to_string().contains("line 3 column 12"), "{error}");
        assert_eq!(
            error.labels().unwrap().next().unwrap().offset(),
            content.find('{').unwrap()
        );

        let error = parse_skill_config("name: demo\ndescription: test\n", Path::new("skill.yaml"))
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("missing required field `skillprism`")
        );
        assert!(!error.to_string().contains("line"));
        assert!(error.labels().unwrap().next().is_none());
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
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\n",
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
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\n",
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
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\n",
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
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\n",
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
            ProjectError::YamlSyntax { .. } => {}
            e => panic!("expected YamlSyntax error, got {e:?}"),
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
            ProjectError::YamlSyntax { .. } => {}
            e => panic!("expected YamlSyntax error, got {e:?}"),
        }
    }

    #[test]
    fn missing_skillprism_version_in_skill_yaml_fails() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "name: my-skill\ndescription: test\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "content\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ConfigSchema { message, .. } => {
                assert!(message.contains("missing required field `skillprism`"));
            }
            e => panic!("expected ConfigSchema error for missing skillprism field, got {e:?}"),
        }
    }

    #[test]
    fn integer_skillprism_version_accepted() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "skillprism: 1\nname: my-skill\ndescription: test\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "content\n").unwrap();

        let model = ProjectLoader::load(root).unwrap();
        assert_eq!(model.skills.len(), 1);
    }

    #[test]
    fn typo_in_harness_override_field_rejected() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\noverrides:\n  claude:\n    variabels:\n      greeting: hi\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(
            result.is_err(),
            "a typo'd override field should not be silently dropped"
        );
        match result.unwrap_err() {
            ProjectError::ConfigSchema { .. } => {}
            e => panic!("expected ConfigSchema error, got {e:?}"),
        }
    }

    #[test]
    fn legacy_harnesses_key_in_skill_yaml_rejected_with_help() {
        let tmp = setup_test_dir();
        let root = tmp.path();
        fs::create_dir_all(root.join("skills/my-skill")).unwrap();

        fs::write(root.join("skillprism.yaml"), "harnesses:\n  - claude\n").unwrap();
        fs::write(
            root.join("skills/my-skill/skill.yaml"),
            "skillprism: '1'\nname: my-skill\ndescription: A test skill\nharnesses:\n  claude:\n    variables:\n      greeting: hi\n",
        )
        .unwrap();
        fs::write(root.join("skills/my-skill/SKILL.md.j2"), "# {{ name }}\n").unwrap();

        let result = ProjectLoader::load(root);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProjectError::ConfigSchema { message, .. } => {
                assert!(message.contains("renamed to `overrides:`"));
            }
            e => panic!("expected ConfigSchema with migration help, got {e:?}"),
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
            "skillprism: '1'\n\
             name: my-skill\n\
             description: test\n\
             variables:\n  \
             greeting: hello\n\
             overrides:\n  \
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
            "skillprism: '1'\nvariables:\n  theme: dark\n  lang: en\n",
        )
        .unwrap();

        fs::write(
            root.join("skills/group/child/skill.yaml"),
            "skillprism: '1'\nvariables:\n  lang: fr\n",
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

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

//! `skillprism list` command implementation.

use crate::state::{InstallScope, InstalledSkill, SkillFormat, StateStore};

use super::add::InstallScopeArg;

/// Runs the `list` command.
pub fn run_list(
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
) -> Result<(), miette::Report> {
    let store = StateStore::open().map_err(miette::Report::new)?;
    let skills = filter_skills(store.skills(), target, harnesses);

    if skills.is_empty() {
        println!("No skills installed");
        return Ok(());
    }

    for skill in skills {
        print_skill(&skill);
    }

    Ok(())
}

fn filter_skills(
    skills: &[InstalledSkill],
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
) -> Vec<InstalledSkill> {
    skills
        .iter()
        .filter(|s| target.is_none_or(|t| InstallScope::from(t) == s.scope))
        .filter(|s| {
            harnesses.is_none_or(|h| {
                let wanted: Vec<_> = h
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .filter(|x| !x.is_empty())
                    .collect();
                wanted.is_empty()
                    || s.harnesses
                        .iter()
                        .any(|installed| wanted.contains(installed))
            })
        })
        .cloned()
        .collect()
}

fn print_skill(skill: &InstalledSkill) {
    let r#ref = skill.r#ref.clone().unwrap_or_else(|| "-".to_string());
    let harnesses = skill.harnesses.join(", ");
    let format = match skill.format {
        SkillFormat::Skillprism => "skillprism",
        SkillFormat::Plain => "plain",
    };
    let scope = match skill.scope {
        InstallScope::Project => "project",
        InstallScope::User => "user",
    };
    println!(
        "{name}\t{source}\t{ref}\t{format}\t{scope}\t{harnesses}",
        name = skill.name,
        source = skill.source,
        ref = r#ref,
        format = format,
        scope = scope,
        harnesses = harnesses
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{InstalledFile, SkillFormat, SourceType, now_rfc3339};

    fn sample_skill(name: &str, scope: InstallScope, harnesses: &[&str]) -> InstalledSkill {
        InstalledSkill {
            name: name.to_string(),
            source: format!("owner/{name}"),
            source_url: format!("https://github.com/owner/{name}.git"),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            scope,
            harnesses: harnesses.iter().map(|h| (*h).to_string()).collect(),
            format: SkillFormat::Skillprism,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![InstalledFile {
                path: format!("{name}.md"),
                hash: "sha256:abc".to_string(),
            }],
        }
    }

    #[test]
    fn filter_by_scope() {
        let skills = vec![
            sample_skill("alpha", InstallScope::Project, &["claude"]),
            sample_skill("beta", InstallScope::User, &["opencode"]),
        ];
        let filtered = filter_skills(&skills, Some(InstallScopeArg::User), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "beta");
    }

    #[test]
    fn filter_by_harness() {
        let skills = vec![
            sample_skill("alpha", InstallScope::Project, &["claude", "opencode"]),
            sample_skill("beta", InstallScope::Project, &["opencode"]),
        ];
        let filtered = filter_skills(&skills, None, Some(&"claude".to_string()));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "alpha");
    }

    #[test]
    fn filter_by_scope_and_harness() {
        let skills = vec![
            sample_skill("alpha", InstallScope::Project, &["claude"]),
            sample_skill("beta", InstallScope::User, &["claude"]),
            sample_skill("gamma", InstallScope::User, &["opencode"]),
        ];
        let filtered = filter_skills(
            &skills,
            Some(InstallScopeArg::User),
            Some(&"claude".to_string()),
        );
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "beta");
    }
}

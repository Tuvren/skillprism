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

use crate::state::{InstalledSkill, SkillFormat, StateStore};

use super::add::InstallScopeArg;
use super::source::mask_credentials;

/// Runs the `list` command.
pub fn run_list(
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
    verbose: bool,
) -> Result<(), miette::Report> {
    let store = StateStore::open().map_err(miette::Report::new)?;
    if verbose {
        eprintln!("[list] loaded {} state record(s)", store.skills().len());
    }
    let skills = filter_skills(store.skills(), target, harnesses);

    if skills.is_empty() {
        // Status message, not table data: stdout stays clean for piping
        // (guidelines.md stdout-discipline; mirrors update's empty-state notice).
        eprintln!("No installed skills");
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
        .filter(|s| super::scope_harness_matches(s, target, harnesses))
        .cloned()
        .collect()
}

fn print_skill(skill: &InstalledSkill) {
    // DIST-I003 column contract: show a short SHA for commit-pinned refs, but
    // branch/tag names verbatim.
    let r#ref = skill.r#ref.as_deref().map_or_else(
        || "-".to_string(),
        |r| {
            if super::network::is_sha_ref(r) && r.len() > 7 {
                r[..7].to_string()
            } else {
                r.to_string()
            }
        },
    );
    let harnesses = skill.harnesses.join(", ");
    let format = match skill.format {
        SkillFormat::Skillprism => "skillprism",
        SkillFormat::Plain => "plain",
    };
    let scope = skill.scope.as_str();
    println!(
        "{name}\t{source}\t{ref}\t{format}\t{scope}\t{harnesses}",
        name = skill.name,
        source = mask_credentials(&skill.source),
        ref = r#ref,
        format = format,
        scope = scope,
        harnesses = harnesses
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{InstallScope, InstalledFile, SkillFormat, SourceType, now_rfc3339};

    fn sample_skill(name: &str, scope: InstallScope, harnesses: &[&str]) -> InstalledSkill {
        InstalledSkill {
            name: name.to_string(),
            source: format!("owner/{name}"),
            source_url: format!("https://github.com/owner/{name}.git"),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: None,
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

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

use crate::state::{InstallScope, StateStore};

use super::add::InstallScopeArg;

/// Runs the `list` command.
pub fn run_list(
    target: Option<InstallScopeArg>,
    harnesses: Option<&String>,
) -> Result<(), miette::Report> {
    let store = StateStore::open().map_err(miette::Report::new)?;
    let skills: Vec<_> = store
        .skills()
        .iter()
        .filter(|s| target.is_none_or(|t| InstallScope::from(t) == s.scope))
        .filter(|s| {
            harnesses.is_none_or(|h| {
                let wanted: Vec<_> = h.split(',').map(|x| x.trim().to_string()).collect();
                s.harnesses
                    .iter()
                    .any(|installed| wanted.contains(installed))
            })
        })
        .cloned()
        .collect();

    if skills.is_empty() {
        println!("No skills installed");
        return Ok(());
    }

    for skill in skills {
        let r#ref = skill.r#ref.unwrap_or_else(|| "-".to_string());
        let harnesses = skill.harnesses.join(", ");
        let scope = match skill.scope {
            InstallScope::Project => "project",
            InstallScope::User => "user",
        };
        println!(
            "{name}\t{source}\t{ref}\t{scope}\t{harnesses}",
            name = skill.name,
            source = skill.source,
            ref = r#ref,
            scope = scope,
            harnesses = harnesses
        );
    }

    Ok(())
}

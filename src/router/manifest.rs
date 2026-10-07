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

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use super::{ManifestEntry, RouterError};

/// Aggregates and validates every manifest before any output is written or diffed.
pub(super) fn aggregate_manifests(
    entries: &[ManifestEntry],
) -> Result<BTreeMap<PathBuf, String>, RouterError> {
    for entry in entries {
        serde_json::from_str::<serde_json::Value>(&entry.content).map_err(|source| {
            RouterError::ManifestJson {
                skill: entry.skill.clone(),
                harness: entry.harness.clone(),
                path: entry.path.to_string_lossy().into_owned(),
                source,
            }
        })?;
    }
    group_manifest_entries(entries)
        .into_iter()
        .map(|(path, group)| {
            let content = aggregate_json_entries(&group);
            serde_json::from_str::<serde_json::Value>(&content).map_err(|source| {
                let skills: BTreeSet<_> = group.iter().map(|entry| entry.skill.as_str()).collect();
                let harnesses: BTreeSet<_> =
                    group.iter().map(|entry| entry.harness.as_str()).collect();
                RouterError::ManifestJson {
                    skill: skills.into_iter().collect::<Vec<_>>().join(", "),
                    harness: harnesses.into_iter().collect::<Vec<_>>().join(", "),
                    path: path.to_string_lossy().into_owned(),
                    source,
                }
            })?;
            Ok((path, content))
        })
        .collect()
}

/// Groups manifest entries by their resolved file path.
fn group_manifest_entries(entries: &[ManifestEntry]) -> BTreeMap<PathBuf, Vec<&ManifestEntry>> {
    let mut grouped: BTreeMap<PathBuf, Vec<&ManifestEntry>> = BTreeMap::new();
    for entry in entries {
        grouped.entry(entry.path.clone()).or_default().push(entry);
    }
    grouped
}

/// Aggregates manifest entries into a JSON array.
///
/// Each entry is expected to be a JSON object string.
/// The result is a JSON array containing all entries.
fn aggregate_json_entries(entries: &[&ManifestEntry]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push_str(",\n");
        }
        for line in entry.content.lines() {
            result.push_str("  ");
            result.push_str(line);
            result.push('\n');
        }
    }
    result.push(']');
    result
}

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
use std::path::PathBuf;

/// Groups manifest entries by their resolved file path.
pub(super) fn group_manifest_entries(
    entries: &[crate::router::ManifestEntry],
) -> BTreeMap<PathBuf, Vec<String>> {
    let mut grouped: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.path.clone())
            .or_default()
            .push(entry.content.clone());
    }
    grouped
}

/// Aggregates manifest entries into a JSON array.
///
/// Each entry is expected to be a JSON object string.
/// The result is a JSON array containing all entries.
pub(super) fn aggregate_json_entries(entries: &[String]) -> String {
    if entries.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[\n");
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            result.push_str(",\n");
        }
        for line in entry.lines() {
            result.push_str("  ");
            result.push_str(line);
            result.push('\n');
        }
    }
    result.push(']');
    result
}

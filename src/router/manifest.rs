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

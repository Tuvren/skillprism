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

use std::path::Path;

use serde::{Deserialize, de::DeserializeOwned};

use crate::types::{ConfigKind, ProjectError};

/// Preserve direct deserialization semantics, classifying only rejected input.
pub fn deserialize<T: DeserializeOwned>(
    content: &str,
    path: &Path,
    kind: ConfigKind,
) -> Result<T, ProjectError> {
    yaml_serde::from_str(content).map_err(|error| {
        // Validate syntax across the document stream. A valid multi-document
        // stream is a schema error because configs require a single document.
        for document in yaml_serde::Deserializer::from_str(content) {
            if let Err(syntax) = yaml_serde::Value::deserialize(document) {
                return ProjectError::yaml_syntax(path, content, &syntax);
            }
        }
        ProjectError::config_schema(kind, path, content, error.to_string(), error.location())
    })
}

/// Value conversion remains authoritative; a second parse supplies diagnostic metadata only.
pub(super) fn from_value<T: DeserializeOwned>(
    value: &yaml_serde::Value,
    content: &str,
    path: &Path,
    kind: ConfigKind,
    field_path: impl FnOnce(&yaml_serde::Value, &str) -> Option<String>,
) -> Result<T, ProjectError> {
    yaml_serde::from_value(value.clone()).map_err(|error| {
        // Direct parsing can coerce scalars that Value conversion rejects. Never
        // replace the authoritative reason with an unrelated later failure.
        let reason = error.to_string();
        let located_error = yaml_serde::from_str::<T>(content)
            .err()
            .filter(|located| located.to_string().contains(&reason));
        let (message, position) = located_error.map_or_else(
            || {
                let message = field_path(value, &reason)
                    .map_or_else(|| reason.clone(), |field| format!("{field}: {reason}"));
                (message, None)
            },
            |located| (located.to_string(), located.location()),
        );
        ProjectError::config_schema(kind, path, content, message, position)
    })
}

pub(super) fn failing_field<'a, T: serde::de::DeserializeOwned>(
    value: &'a yaml_serde::Value,
    reason: &str,
) -> Option<(&'a str, &'a yaml_serde::Value)> {
    let map = value.as_mapping()?;
    for (key, value) in map {
        // Reuse the actual struct's field types only after rejection, so these
        // probes cannot change acceptance or replace the first schema failure.
        let probe =
            yaml_serde::Value::Mapping(std::iter::once((key.clone(), value.clone())).collect());
        if yaml_serde::from_value::<T>(probe)
            .err()
            .is_some_and(|error| error.to_string() == reason)
        {
            return Some((key.as_str()?, value));
        }
    }
    None
}

pub(super) fn failing_entry<'a, T: serde::de::DeserializeOwned>(
    value: &'a yaml_serde::Value,
    reason: &str,
) -> Option<(&'a yaml_serde::Value, &'a yaml_serde::Value)> {
    value.as_mapping()?.iter().find_map(|(key, value)| {
        // Check the key before its value, just as the authoritative map does.
        let probe =
            yaml_serde::Value::Mapping(std::iter::once((key.clone(), value.clone())).collect());
        yaml_serde::from_value::<std::collections::BTreeMap<String, T>>(probe)
            .err()
            .filter(|error| error.to_string() == reason)
            .map(|_| (key, value))
    })
}

pub(super) fn entry_key_path(key: &yaml_serde::Value) -> String {
    yaml_serde::from_value::<String>(key.clone()).map_or_else(
        |_| {
            let key = yaml_serde::to_string(key).unwrap_or_else(|_| format!("{key:?}"));
            format!(": key `{}`", key.trim())
        },
        |key| format!(".{key}"),
    )
}

pub(super) fn string_entry_path(value: &yaml_serde::Value, reason: &str) -> Option<String> {
    value.as_sequence().map_or_else(
        || failing_entry::<String>(value, reason).map(|(key, _)| entry_key_path(key)),
        |sequence| {
            sequence.iter().enumerate().find_map(|(index, value)| {
                yaml_serde::from_value::<String>(value.clone())
                    .err()
                    .filter(|error| error.to_string() == reason)
                    .map(|_| format!("[{index}]"))
            })
        },
    )
}

/// Locate a manifest version value using the parser, including quoted and flow-style keys.
pub(super) fn manifest_version_location(content: &str) -> Option<yaml_serde::Location> {
    #[derive(serde::Deserialize)]
    struct Probe {
        #[serde(rename = "skillprism")]
        _version: RejectValue,
    }
    yaml_serde::from_str::<Probe>(content)
        .err()
        .filter(is_probe_rejection)
        .and_then(|error| error.location())
}

/// Locate the legacy migration field while ignoring the other manifest fields.
pub(super) fn legacy_harnesses_location(content: &str) -> Option<yaml_serde::Location> {
    #[derive(serde::Deserialize)]
    struct Probe {
        #[serde(rename = "harnesses")]
        _legacy: RejectValue,
    }
    yaml_serde::from_str::<Probe>(content)
        .err()
        .filter(is_probe_rejection)
        .and_then(|error| error.location())
}

/// Locate the root node when a manifest is not a mapping.
pub(super) fn root_location(content: &str) -> Option<yaml_serde::Location> {
    yaml_serde::from_str::<RejectValue>(content)
        .err()
        .filter(is_probe_rejection)
        .and_then(|error| error.location())
}

fn is_probe_rejection(error: &yaml_serde::Error) -> bool {
    // Earlier failures (for example complex mapping keys) did not reach our value.
    error
        .to_string()
        .contains("expected a valid manifest value")
}

struct RejectValue;

impl<'de> serde::Deserialize<'de> for RejectValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = RejectValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a valid manifest value")
            }
        }

        // The default visitor rejects every value at its parser-reported mark.
        // Only that mark is used; these probes never decide which inputs load.
        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProjectConfig;

    #[test]
    fn config_diagnostics_value_conversion_preserves_config_kind() {
        for kind in [ConfigKind::Project, ConfigKind::Skill, ConfigKind::Harness] {
            let error = from_value::<String>(
                &yaml_serde::Value::from(1),
                "1\n",
                Path::new("config.yaml"),
                kind,
                |_, _| None,
            )
            .unwrap_err();
            let ProjectError::ConfigSchema { kind: actual, .. } = error else {
                panic!("expected a config schema error");
            };
            assert_eq!(actual.to_string(), kind.to_string());
        }
    }

    #[test]
    fn config_diagnostics_both_variants_have_help_and_short_labels() {
        use miette::Diagnostic;

        for (content, syntax, suggestion) in [
            (
                "harnesses: [claude",
                true,
                "Fix the YAML syntax at the reported location",
            ),
            ("name: demo\n", false, "Check field names and value types"),
        ] {
            let error = deserialize::<ProjectConfig>(
                content,
                Path::new("skillprism.yaml"),
                ConfigKind::Project,
            )
            .unwrap_err();
            assert_eq!(matches!(error, ProjectError::YamlSyntax { .. }), syntax);
            assert_eq!(matches!(error, ProjectError::ConfigSchema { .. }), !syntax);
            let help = error.help().expect("config diagnostics must provide help");
            assert!(help.to_string().contains(suggestion), "{help}");
            assert_eq!(
                error.labels().unwrap().next().unwrap().label(),
                Some("here")
            );
        }
    }

    #[test]
    fn config_diagnostics_probes_ignore_errors_before_requested_field() {
        let prefix = "? [complex, key]\n: hello\n";
        assert!(
            manifest_version_location(&format!("{prefix}skillprism: '2'\n")).is_none(),
            "a complex key must not label the manifest version"
        );
        assert!(
            legacy_harnesses_location(&format!("{prefix}harnesses: {{}}\n")).is_none(),
            "a complex key must not label the legacy field"
        );
        assert!(manifest_version_location("name: demo\n").is_none());
        assert!(legacy_harnesses_location("skillprism: '1'\n").is_none());
        assert_eq!(
            manifest_version_location("skillprism: '2'\n")
                .unwrap()
                .column(),
            13
        );
        assert_eq!(
            legacy_harnesses_location("skillprism: '1'\nharnesses: {}\n")
                .unwrap()
                .line(),
            2
        );
    }

    #[test]
    fn config_diagnostics_duplicate_mapping_keys_are_yaml_syntax_errors() {
        let content = "harnesses: [claude]\nharnesses: [codex]\n";
        let error = deserialize::<ProjectConfig>(
            content,
            Path::new("skillprism.yaml"),
            ConfigKind::Project,
        )
        .unwrap_err();
        assert!(matches!(error, ProjectError::YamlSyntax { .. }), "{error}");
        assert!(
            error
                .to_string()
                .contains("duplicate entry with key \"harnesses\""),
            "{error}"
        );
    }

    #[test]
    fn config_diagnostics_multiple_documents_are_file_level_schema_errors() {
        use miette::Diagnostic;

        let content = "harnesses: [claude]\n---\nharnesses: [codex]\n";
        let error = deserialize::<ProjectConfig>(
            content,
            Path::new("skillprism.yaml"),
            ConfigKind::Project,
        )
        .unwrap_err();
        assert!(
            matches!(error, ProjectError::ConfigSchema { .. }),
            "{error}"
        );
        assert!(error.to_string().contains("more than one document"));
        assert!(!error.to_string().contains("line"));
        assert!(error.labels().unwrap().next().is_none());
    }
}

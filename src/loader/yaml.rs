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

use crate::types::ProjectError;

/// Preserve direct deserialization semantics, classifying only rejected input.
pub fn deserialize<T: DeserializeOwned>(content: &str, path: &Path) -> Result<T, ProjectError> {
    yaml_serde::from_str(content).map_err(|error| {
        // Validate syntax across the document stream. A valid multi-document
        // stream is a schema error because configs require a single document.
        for document in yaml_serde::Deserializer::from_str(content) {
            if let Err(syntax) = yaml_serde::Value::deserialize(document) {
                return ProjectError::yaml_syntax(path, content, &syntax);
            }
        }
        ProjectError::config_schema(path, content, error.to_string(), error.location())
    })
}

/// Value conversion remains authoritative; a second parse supplies diagnostic metadata only.
pub(super) fn from_value<T: DeserializeOwned>(
    value: yaml_serde::Value,
    content: &str,
    path: &Path,
) -> Result<T, ProjectError> {
    yaml_serde::from_value(value).map_err(|error| {
        // Direct parsing can coerce scalars that Value conversion rejects. Never
        // replace the authoritative reason with an unrelated later failure.
        let located_error = yaml_serde::from_str::<T>(content)
            .err()
            .filter(|located| located.to_string().contains(&error.to_string()))
            .unwrap_or(error);
        ProjectError::config_schema(
            path,
            content,
            located_error.to_string(),
            located_error.location(),
        )
    })
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
        .and_then(|error| error.location())
}

/// Locate the root node when a manifest is not a mapping.
pub(super) fn root_location(content: &str) -> Option<yaml_serde::Location> {
    yaml_serde::from_str::<RejectValue>(content)
        .err()
        .and_then(|error| error.location())
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
    fn config_diagnostics_multiple_documents_are_file_level_schema_errors() {
        use miette::Diagnostic;

        let content = "harnesses: [claude]\n---\nharnesses: [codex]\n";
        let error =
            deserialize::<ProjectConfig>(content, Path::new("skillprism.yaml")).unwrap_err();
        assert!(
            matches!(error, ProjectError::ConfigSchema { .. }),
            "{error}"
        );
        assert!(error.to_string().contains("more than one document"));
        assert!(!error.to_string().contains("line"));
        assert!(error.labels().unwrap().next().is_none());
    }
}

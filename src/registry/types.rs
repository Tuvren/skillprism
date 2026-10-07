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

// reason: HarnessCapabilities has 8 boolean fields by design; extracting a bools struct
// would add indirection without clarity benefit. Lint kept at module level intentionally.
#![allow(clippy::struct_excessive_bools)]

use std::collections::BTreeMap;

use serde::Deserialize;

/// Complete definition of a harness including its capabilities, paths, and templates.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessDefinition {
    /// Unique identifier for the harness (e.g. "claude", "opencode").
    pub id: String,
    /// Human-readable display name (e.g. "Claude Code").
    pub name: String,
    /// Optional version string.
    pub version: Option<String>,
    /// Capabilities supported by this harness.
    pub capabilities: HarnessCapabilities,
    /// File system paths for skill and manifest output.
    pub paths: HarnessPaths,
    /// Named macros available to templates as `harness.<name>`.
    #[serde(default)]
    pub macros: BTreeMap<String, MacroDef>,
    /// Sidecar file definitions produced alongside skills.
    #[serde(default)]
    pub sidecars: Vec<SidecarDef>,
    /// Optional manifest definition for plugin metadata.
    pub manifest: Option<ManifestDef>,
    /// Pattern for referencing skills in this harness (e.g. "/{name}").
    pub skill_ref_pattern: Option<String>,
}

/// Feature flags and constraints for a harness.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessCapabilities {
    /// Whether the harness supports sub-agent invocation.
    pub supports_subagent: bool,
    /// Whether the harness requires sidecar files.
    #[serde(default)]
    pub requires_sidecar: bool,
    /// Whether the harness requires a manifest file.
    #[serde(default)]
    pub requires_manifest: bool,
    /// Maximum allowed skill name length.
    #[serde(default = "default_name_max")]
    pub name_max_length: usize,
    /// Maximum allowed description length.
    #[serde(default = "default_desc_max")]
    pub description_max_length: usize,
    /// Whether `allowed-tools` is supported in skill config.
    #[serde(default)]
    pub supports_allowed_tools: bool,
    /// Whether `disable-model-invocation` is supported in skill config.
    #[serde(default)]
    pub supports_disable_model_invocation: bool,
    /// Whether `user-invocable` is supported in skill config.
    #[serde(default)]
    pub supports_user_invocable_flag: bool,
}

const fn default_name_max() -> usize {
    64
}

const fn default_desc_max() -> usize {
    1024
}

/// File system paths where skills and manifests are written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessPaths {
    /// Directory for project-scoped skill output (e.g. ".claude/skills").
    pub project_scope_path: String,
    /// Directory for user-scoped skill output (e.g. "~/.claude/skills").
    pub user_scope_path: String,
    /// Filename for the rendered skill file (e.g. "SKILL.md").
    pub skill_filename: String,
    /// Directory for manifest output (e.g. ".claude").
    pub manifest_scope_path: Option<String>,
    /// Filename for the manifest file (e.g. "plugin.json").
    pub manifest_filename: Option<String>,
}

/// A harness macro definition: either inline text or a function body.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MacroDef {
    /// An inline string macro.
    Inline(String),
    /// A function-like macro with body content.
    Function {
        /// The macro body.
        content: String,
    },
}

/// Definition of a sidecar file produced alongside the main skill.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidecarDef {
    /// Filename for the sidecar output.
    pub filename: String,
    /// Jinja2 template for the sidecar content.
    pub template: String,
    /// Optional subdirectory within the skill output directory.
    pub output_dir: Option<String>,
}

/// Definition of a manifest file produced for the harness.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestDef {
    /// Jinja2 template for the manifest content.
    pub template: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::schema_contract::{
        assert_loader_contract, assert_struct_contract, yaml_accepts,
    };
    use serde_json::{Value, json};

    fn schema() -> Value {
        serde_json::from_str(include_str!("../../schemas/harness-schema.json")).unwrap()
    }

    #[test]
    fn harness_definition_schema_field_coverage() {
        assert_struct_contract::<HarnessDefinition>(
            &schema(),
            &json!({
                "id": "demo", "name": "Demo", "version": "1",
                "capabilities": {"supports_subagent": false},
                "paths": {"project_scope_path": ".demo/skills", "user_scope_path": ".demo/skills", "skill_filename": "SKILL.md"},
                "macros": {}, "sidecars": [], "manifest": {"template": "{}"}, "skill_ref_pattern": "/{name}"
            }),
            yaml_accepts::<HarnessDefinition>,
        );
    }

    #[test]
    fn harness_capabilities_schema_field_coverage() {
        assert_struct_contract::<HarnessCapabilities>(
            &schema()["properties"]["capabilities"],
            &json!({
                "supports_subagent": true, "requires_sidecar": false, "requires_manifest": true,
                "name_max_length": 64, "description_max_length": 1024, "supports_allowed_tools": true,
                "supports_disable_model_invocation": true, "supports_user_invocable_flag": true
            }),
            yaml_accepts::<HarnessCapabilities>,
        );
    }

    #[test]
    fn harness_paths_schema_field_coverage() {
        assert_struct_contract::<HarnessPaths>(
            &schema()["properties"]["paths"],
            &json!({
                "project_scope_path": ".demo/skills", "user_scope_path": ".demo/skills", "skill_filename": "SKILL.md",
                "manifest_scope_path": ".demo", "manifest_filename": "plugin.json"
            }),
            yaml_accepts::<HarnessPaths>,
        );
    }

    #[test]
    fn harness_sidecar_schema_field_coverage() {
        assert_struct_contract::<SidecarDef>(
            &schema()["properties"]["sidecars"]["items"],
            &json!({"filename": "demo.json", "template": "{}", "output_dir": "data"}),
            yaml_accepts::<SidecarDef>,
        );
    }

    #[test]
    fn harness_manifest_schema_field_coverage() {
        assert_struct_contract::<ManifestDef>(
            &schema()["properties"]["manifest"],
            &json!({"template": "{}"}),
            yaml_accepts::<ManifestDef>,
        );
    }

    #[test]
    fn harness_macro_schema_field_coverage() {
        let schema = schema();
        let inline = &schema["properties"]["macros"]["additionalProperties"]["oneOf"][0];
        assert!(
            jsonschema::draft202012::new(inline)
                .unwrap()
                .is_valid(&json!("body"))
        );
        assert!(
            matches!(yaml_serde::from_str::<MacroDef>("body").unwrap(), MacroDef::Inline(body) if body == "body")
        );

        let function = &schema["properties"]["macros"]["additionalProperties"]["oneOf"][1];
        let sample = json!({"content": "body"});
        // The untagged enum does not expose deserialize_struct's field list.
        // An exhaustive pattern guards the variant's fields; deserialization
        // verifies its serde name, and omission verifies that it is required.
        match yaml_serde::from_str::<MacroDef>(&yaml_serde::to_string(&sample).unwrap()).unwrap() {
            MacroDef::Function { content } => assert_eq!(content, "body"),
            MacroDef::Inline(_) => panic!("expected function macro"),
        }
        let fields = std::iter::once("content").collect();
        assert_eq!(
            function["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            fields
        );
        assert_loader_contract(function, &sample, &fields, yaml_accepts::<MacroDef>);
        // Macro function objects are open: the loader ignores extra keys.
        let extra_key = json!({"content": "body", "unknown_property": "ignored"});
        assert!(
            jsonschema::draft202012::new(function)
                .unwrap()
                .is_valid(&extra_key)
        );
        assert!(yaml_accepts::<MacroDef>(&extra_key));
    }

    #[test]
    fn harness_schema_documents_integral_float_length_limitation() {
        let schema = schema();
        let validator = jsonschema::draft202012::new(&schema).unwrap();
        for field in ["name_max_length", "description_max_length"] {
            let content = format!(
                "id: demo\nname: Demo\ncapabilities:\n  supports_subagent: false\n  {field}: 64.0\n\
                 paths:\n  project_scope_path: .demo\n  user_scope_path: .demo\n  skill_filename: SKILL.md\n"
            );
            let value: Value = yaml_serde::from_str(&content).unwrap();
            assert!(validator.is_valid(&value), "integral float for {field}");
            assert!(yaml_serde::from_str::<HarnessDefinition>(&content).is_err());
            assert!(
                schema["properties"]["capabilities"]["properties"][field]["description"]
                    .as_str()
                    .unwrap()
                    .contains("integral floats")
            );
        }
    }
}

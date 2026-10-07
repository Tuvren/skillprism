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

use serde::Deserialize;

/// Top-level project configuration deserialized from skillprism.yaml.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Harness IDs this project targets (e.g. `claude`, `opencode`).
    #[serde(default)]
    pub harnesses: Vec<String>,

    /// Directory containing skill definitions, relative to project root.
    #[serde(default = "default_skills_dir")]
    pub skills_dir: PathBuf,
}

fn default_skills_dir() -> PathBuf {
    PathBuf::from("skills")
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            harnesses: Vec::new(),
            skills_dir: default_skills_dir(),
        }
    }
}

/// A skill loaded from disk with all its configuration and metadata.
#[derive(Debug, Clone)]
pub struct SkillModel {
    /// Skill name, either from skill.yaml or the directory name.
    pub name: String,
    /// The directory name on disk.
    #[allow(dead_code)]
    pub directory_name: String,
    /// Human-readable description of the skill.
    pub description: String,
    /// Optional semantic version.
    pub version: Option<String>,
    /// Optional license identifier.
    pub license: Option<String>,
    /// Optional compatibility note.
    pub compatibility: Option<String>,
    /// Arbitrary key-value metadata.
    pub metadata: BTreeMap<String, String>,
    /// Comma-separated list of allowed tools.
    pub allowed_tools: Option<String>,
    /// Description of when the skill should be used.
    pub when_to_use: Option<String>,
    /// Hint shown to users for skill arguments.
    pub argument_hint: Option<String>,
    /// List of argument names.
    pub arguments: Option<Vec<String>>,
    /// Whether to suppress default model invocation.
    pub disable_model_invocation: Option<bool>,
    /// Whether the skill is user-invocable.
    pub user_invocable: Option<bool>,
    /// Tools explicitly disallowed for this skill.
    pub disallowed_tools: Option<Vec<String>>,
    /// Override the default model for this skill.
    pub model_override: Option<String>,
    /// Effort level hint for the model.
    pub effort: Option<String>,
    /// Whether the skill uses context forking.
    pub context_fork: bool,
    /// Optional agent identifier for sub-agent invocation.
    pub agent: Option<String>,
    /// Optional lifecycle hooks.
    pub hooks: Option<BTreeMap<String, yaml_serde::Value>>,
    /// Optional file paths that trigger this skill.
    pub activation_paths: Option<Vec<String>>,
    /// Optional shell command to execute.
    pub shell: Option<String>,
    /// Capabilities required from the harness.
    pub required_capabilities: Vec<String>,
    /// Template variables defined in skill.yaml (inherited from parent groups).
    pub variables: BTreeMap<String, yaml_serde::Value>,
    /// Path to the Jinja2 template file.
    pub template_path: PathBuf,
    /// Every direct subdirectory of the skill's own directory (e.g. `references/`,
    /// `scripts/`, or any other name an author uses), copied verbatim alongside it.
    pub asset_dirs: Vec<PathBuf>,
    /// Per-harness overrides from skill.yaml's `harnesses:` block, keyed by harness ID.
    pub harness_overrides: BTreeMap<String, HarnessOverride>,
}

impl SkillModel {
    /// Resolves this skill's variables for rendering against a specific harness: the
    /// top-level `variables:` map with that harness's `harnesses.<id>.variables`
    /// overrides merged in on top (harness wins), per the schema's documented
    /// "merged with top-level variables, harness wins" semantics.
    pub fn variables_for_harness(&self, harness_id: &str) -> BTreeMap<String, yaml_serde::Value> {
        let mut merged = self.variables.clone();
        if let Some(override_) = self.harness_overrides.get(harness_id) {
            for (k, v) in &override_.variables {
                merged.insert(k.clone(), v.clone());
            }
        }
        merged
    }
}

/// Per-harness overrides for a single skill, from skill.yaml's `harnesses:` block.
#[derive(Debug, Clone, Default)]
pub struct HarnessOverride {
    /// Variable overrides merged over top-level `variables`, harness wins.
    pub variables: BTreeMap<String, yaml_serde::Value>,
    /// Macro overrides scoped to this skill only — harness wins over that harness's
    /// own builtin macro of the same name, if any.
    pub macros: BTreeMap<String, String>,
}

/// Names of `SkillModel` metadata fields exposed as built-in template variables,
/// one-to-one with the fields below `description`. Shared by
/// `engine::context::build_context` (which inserts them into the render context) and
/// `validator::variables::is_builtin` (which exempts them from undefined-variable
/// checks) so the two can't silently drift apart — `engine::context::tests` asserts
/// every name here is actually present in a built context.
pub const SKILL_METADATA_FIELDS: &[&str] = &[
    "version",
    "license",
    "compatibility",
    "metadata",
    "allowed_tools",
    "when_to_use",
    "argument_hint",
    "arguments",
    "disable_model_invocation",
    "user_invocable",
    "disallowed_tools",
    "model",
    "model_override",
    "effort",
    "context",
    "context_fork",
    "agent",
    "hooks",
    "paths",
    "activation_paths",
    "shell",
    "required_capabilities",
];

/// The complete loaded model of a skillprism project.
#[derive(Debug, Clone)]
pub struct ProjectModel {
    /// Project-level configuration from skillprism.yaml.
    pub config: ProjectConfig,
    /// All discovered skills in the project.
    pub skills: Vec<SkillModel>,
    /// Absolute path to the project root directory.
    #[allow(dead_code)]
    pub project_root: PathBuf,
}

/// Test-only introspection of serde's actual field list, including field renames.
#[cfg(test)]
pub mod schema_contract {
    use std::collections::BTreeSet;

    use serde::de::{DeserializeOwned, Error, Visitor};
    use serde::{Deserializer, forward_to_deserialize_any};
    use serde_json::{Value, json};

    struct Fields<'a>(&'a mut Option<&'static [&'static str]>);

    impl<'de> Deserializer<'de> for Fields<'_> {
        type Error = serde::de::value::Error;

        fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
            Err(Self::Error::custom("field introspection only"))
        }

        fn deserialize_struct<V: Visitor<'de>>(
            self,
            _name: &'static str,
            fields: &'static [&'static str],
            _visitor: V,
        ) -> Result<V::Value, Self::Error> {
            *self.0 = Some(fields);
            Err(Self::Error::custom("field introspection only"))
        }

        forward_to_deserialize_any! {
            bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
            byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
            map enum identifier ignored_any
        }
    }

    pub fn assert_struct_contract<T: DeserializeOwned>(
        schema: &Value,
        sample: &Value,
        accepts: impl Fn(&Value) -> bool,
    ) {
        let mut fields = None;
        assert!(T::deserialize(Fields(&mut fields)).is_err());
        let fields: BTreeSet<_> = fields
            .expect("serde struct field list")
            .iter()
            .copied()
            .collect();
        assert_eq!(
            fields,
            schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect(),
            "schema properties must equal serde fields in both directions"
        );
        assert_loader_contract(schema, sample, &fields, &accepts);
        let mut unknown = sample.clone();
        unknown["unknown_property"] = json!("unexpected");
        assert!(
            !jsonschema::draft202012::new(schema)
                .unwrap()
                .is_valid(&unknown),
            "schema must reject unknown properties in a closed object"
        );
        assert!(
            !accepts(&unknown),
            "loader must reject unknown properties in a closed object"
        );
    }

    pub fn assert_loader_contract(
        schema: &Value,
        sample: &Value,
        fields: &BTreeSet<&str>,
        accepts: impl Fn(&Value) -> bool,
    ) {
        assert_eq!(
            *fields,
            sample
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect(),
            "the sample must exercise every field"
        );
        assert!(accepts(sample), "loader must accept the complete sample");
        let validator = jsonschema::draft202012::new(schema).unwrap();
        assert!(
            validator.is_valid(sample),
            "schema must accept the complete sample"
        );
        let mut required = BTreeSet::new();
        for field in fields {
            let mut omitted = sample.clone();
            omitted.as_object_mut().unwrap().remove(*field);
            if !accepts(&omitted) {
                required.insert(*field);
            }
        }
        let declared: BTreeSet<_> = schema.get("required").map_or_else(BTreeSet::new, |value| {
            value
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect()
        });
        assert_eq!(
            required, declared,
            "required fields must match the full loader contract"
        );
        assert_schema_soundness(schema, sample, fields, &declared, &accepts);
    }

    fn assert_schema_soundness(
        schema: &Value,
        sample: &Value,
        fields: &BTreeSet<&str>,
        required: &BTreeSet<&str>,
        accepts: impl Fn(&Value) -> bool,
    ) {
        let validator = jsonschema::draft202012::new(schema).unwrap();
        let mut minimal = sample.clone();
        minimal
            .as_object_mut()
            .unwrap()
            .retain(|field, _| required.contains(field.as_str()));
        assert!(validator.is_valid(&minimal));
        assert!(accepts(&minimal), "schema-valid minimal sample must load");
        for field in fields {
            let mut optional = minimal.clone();
            optional[*field] = sample[*field].clone();
            assert!(validator.is_valid(&optional));
            assert!(accepts(&optional), "schema-valid field {field} must load");

            let mut null = sample.clone();
            null[*field] = Value::Null;
            if validator.is_valid(&null) {
                assert!(accepts(&null), "schema-valid null for {field} must load");
                optional[*field] = Value::Null;
                assert!(validator.is_valid(&optional));
                assert!(accepts(&optional), "schema-valid optional null must load");
            }

            // Maps fail string coercion too, unlike plain numbers or booleans.
            let property = jsonschema::draft202012::new(&schema["properties"][*field]).unwrap();
            let wrong_type = [
                json!({"unexpected": "value"}),
                json!([]),
                json!("wrong-type"),
            ]
            .into_iter()
            .find(|value| !property.is_valid(value))
            .expect("a representative wrong non-null type");
            let mut invalid = sample.clone();
            invalid[*field] = wrong_type;
            assert!(
                !validator.is_valid(&invalid),
                "schema must reject wrong type for {field}"
            );
            assert!(
                !accepts(&invalid),
                "loader must reject wrong type for {field}"
            );
        }
    }

    pub fn yaml_accepts<T: DeserializeOwned>(value: &Value) -> bool {
        yaml_serde::from_str::<T>(&yaml_serde::to_string(value).unwrap()).is_ok()
    }

    #[test]
    fn project_config_schema_field_coverage() {
        let schema =
            serde_json::from_str(include_str!("../../schemas/project-config-schema.json")).unwrap();
        assert_struct_contract::<super::ProjectConfig>(
            &schema,
            &json!({"harnesses": ["claude"], "skills_dir": "skills"}),
            yaml_accepts::<super::ProjectConfig>,
        );
    }
}

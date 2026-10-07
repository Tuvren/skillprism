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

//! File-level checks for the hand-written draft 2020-12 contracts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use jsonschema::Validator;
use serde_json::{Value, json};

// Classification is per file: the project config in the missing-version fixture
// is valid. Template errors (such as dist-undefined) do not invalidate metadata.
const EXPECTED_INVALID: &[&str] = &[
    "tests/fixtures/invalid-unknown-project-field/skillprism.yaml",
    "tests/fixtures/invalid-missing-skillprism-field/skills/demo/skill.yaml",
    "tests/fixtures/invalid-syntax-error/skillprism.yaml",
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn validator(name: &str) -> Validator {
    let schema: Value =
        serde_json::from_str(&fs::read_to_string(root().join("schemas").join(name)).unwrap())
            .unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    jsonschema::draft202012::meta::validate(&schema).unwrap();
    jsonschema::draft202012::new(&schema).unwrap()
}

fn yaml(path: &Path) -> Result<Value, yaml_serde::Error> {
    // Deserialize into JSON directly: these configuration files have string keys.
    yaml_serde::from_str(&fs::read_to_string(path).unwrap())
}

fn assert_valid(path: &Path, schema: &Validator) {
    let instance = yaml(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let errors: Vec<_> = schema
        .iter_errors(&instance)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{}: {errors:?}", path.display());
}

fn assert_modeline(path: &Path, schema_name: &str) {
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(root().join("schemas").join(schema_name)).unwrap(),
    )
    .unwrap();
    let expected = format!(
        "# yaml-language-server: $schema={}",
        schema["$id"].as_str().unwrap()
    );
    let content = fs::read_to_string(path).unwrap();
    assert_eq!(
        content.lines().next(),
        Some(expected.as_str()),
        "{}",
        path.display()
    );
}

fn config_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            files.extend(config_files(&entry.path()));
        } else if matches!(
            entry.file_name().to_str(),
            Some("skillprism.yaml" | "skill.yaml")
        ) {
            files.push(entry.path());
        }
    }
    files.sort();
    files
}

#[test]
fn schemas_have_absolute_versioned_published_ids() {
    for (name, expected_id) in [
        (
            "project-config-schema.json",
            "https://tuvren.github.io/skillprism/schema/v1/skillprism.json",
        ),
        (
            "skill-schema.json",
            "https://tuvren.github.io/skillprism/schema/v1/skill.json",
        ),
        (
            "harness-schema.json",
            "https://tuvren.github.io/skillprism/schema/v1/harness.json",
        ),
    ] {
        let schema: Value =
            serde_json::from_str(&fs::read_to_string(root().join("schemas").join(name)).unwrap())
                .unwrap();
        assert_eq!(schema["$id"], expected_id, "{name}");
    }
}

#[test]
fn hugo_schema_mounts_match_published_ids() {
    let config = fs::read_to_string(root().join("site/hugo.toml")).unwrap();
    let mut mounts = Vec::<BTreeMap<&str, &str>>::new();
    let mut in_mount = false;
    // Only module.mounts source/target pairs are needed; no TOML dependency.
    for line in config.lines().map(str::trim) {
        if line.starts_with('[') {
            in_mount = line == "[[module.mounts]]";
            if in_mount {
                mounts.push(BTreeMap::new());
            }
        } else if in_mount {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                if matches!(key, "source" | "target") {
                    let value = value.trim().strip_prefix('"').unwrap();
                    let value = value.strip_suffix('"').unwrap();
                    assert!(mounts.last_mut().unwrap().insert(key, value).is_none());
                }
            }
        }
    }

    let mut mounted = BTreeSet::new();
    for mount in &mounts {
        let source = mount.get("source").unwrap();
        let Some(file) = source.strip_prefix("../schemas/") else {
            continue;
        };
        assert!(file.ends_with(".json"));
        assert!(mounted.insert(file), "duplicate schema mount: {file}");
        let schema: Value =
            serde_json::from_str(&fs::read_to_string(root().join("schemas").join(file)).unwrap())
                .unwrap();
        let id = schema["$id"].as_str().unwrap();
        let (_, url_path) = id
            .strip_prefix("https://")
            .unwrap()
            .split_once('/')
            .unwrap();
        let name = url_path.strip_prefix("skillprism/schema/v1/").unwrap();
        assert!(!name.contains('/'));
        assert!(name.ends_with(".json"));
        assert_eq!(mount["target"], format!("static/schema/v1/{name}"));
    }
    assert_eq!(
        mounted,
        BTreeSet::from([
            "project-config-schema.json",
            "skill-schema.json",
            "harness-schema.json",
        ])
    );
}

#[test]
fn examples_and_classified_fixtures_match_schemas() {
    let project = validator("project-config-schema.json");
    let skill = validator("skill-schema.json");
    let mut rejected = BTreeSet::new();
    for directory in ["examples", "tests/fixtures"] {
        let files = config_files(&root().join(directory));
        assert!(!files.is_empty(), "no config files in {directory}");
        for path in files {
            let relative = path.strip_prefix(root()).unwrap().to_str().unwrap();
            let schema = if path.file_name().unwrap() == "skillprism.yaml" {
                &project
            } else {
                &skill
            };
            if EXPECTED_INVALID.contains(&relative) {
                assert!(
                    yaml(&path).map_or(true, |value| !schema.is_valid(&value)),
                    "expected-invalid fixture now passes: {relative}"
                );
                rejected.insert(relative.to_owned());
            } else {
                assert_valid(&path, schema);
            }
        }
    }
    assert_eq!(
        rejected,
        EXPECTED_INVALID
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
    );
}

#[test]
fn builtin_harnesses_match_schema() {
    let schema = validator("harness-schema.json");
    for id in ["claude", "codex", "opencode", "factory", "pi"] {
        assert_valid(
            &root().join(format!("src/builtin_harnesses/{id}.yaml")),
            &schema,
        );
    }
}

fn init(dir: &Path, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_skillprism"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "init {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scaffold_output_matches_schemas() {
    let temp = tempfile::tempdir().unwrap();
    init(
        temp.path(),
        &[
            "init",
            "project",
            "schema-project",
            "--harnesses",
            "claude,codex,opencode,factory,pi",
        ],
    );
    let project = temp.path().join("schema-project");
    assert_modeline(
        &project.join("skillprism.yaml"),
        "project-config-schema.json",
    );
    assert_valid(
        &project.join("skillprism.yaml"),
        &validator("project-config-schema.json"),
    );
    let skill = validator("skill-schema.json");
    assert_modeline(
        &project.join("skills/sample/skill.yaml"),
        "skill-schema.json",
    );
    assert_valid(&project.join("skills/sample/skill.yaml"), &skill);
    init(&project, &["init", "skill", "schema-skill"]);
    assert_modeline(
        &project.join("skills/schema-skill/skill.yaml"),
        "skill-schema.json",
    );
    assert_valid(&project.join("skills/schema-skill/skill.yaml"), &skill);
    init(&project, &["init", "harness", "schema-harness"]);
    assert_modeline(
        &project.join("harnesses/schema-harness.yaml"),
        "harness-schema.json",
    );
    assert_valid(
        &project.join("harnesses/schema-harness.yaml"),
        &validator("harness-schema.json"),
    );
}

#[test]
fn schemas_reject_unknown_project_name_and_missing_skillprism() {
    assert!(
        !validator("project-config-schema.json")
            .is_valid(&json!({"name": "my-skills", "harnesses": ["claude"]}))
    );
    assert!(
        !validator("skill-schema.json")
            .is_valid(&json!({"name": "demo", "description": "Example skill"}))
    );
}

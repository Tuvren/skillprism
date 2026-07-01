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

use crate::registry::{HarnessDefinition, MacroDef};
use crate::resolver::ResolvedPair;

/// Builds the template rendering context from a resolved skill-harness pair.
///
/// Populates `skill_name`, `skill_description`, every other documented `skill.yaml`
/// metadata field (see `types::SKILL_METADATA_FIELDS`), skill variables, and the
/// `harness` object (including macros) for use in Jinja2 templates. Variables and
/// macros are resolved against this pair's specific harness first, falling back to the
/// skill's top-level `variables`/the harness's own macros — see skill.yaml's
/// `harnesses:` block (`types::HarnessOverride`).
pub fn build_context(pair: &ResolvedPair) -> BTreeMap<String, minijinja::Value> {
    let mut ctx = BTreeMap::new();
    let skill = &pair.skill;

    ctx.insert("skill_name".to_string(), skill.name.clone().into());
    ctx.insert(
        "skill_description".to_string(),
        skill.description.clone().into(),
    );

    ctx.insert(
        "version".to_string(),
        minijinja::Value::from_serialize(&skill.version),
    );
    ctx.insert(
        "license".to_string(),
        minijinja::Value::from_serialize(&skill.license),
    );
    ctx.insert(
        "compatibility".to_string(),
        minijinja::Value::from_serialize(&skill.compatibility),
    );
    ctx.insert(
        "metadata".to_string(),
        minijinja::Value::from_serialize(&skill.metadata),
    );
    ctx.insert(
        "allowed_tools".to_string(),
        minijinja::Value::from_serialize(&skill.allowed_tools),
    );
    ctx.insert(
        "when_to_use".to_string(),
        minijinja::Value::from_serialize(&skill.when_to_use),
    );
    ctx.insert(
        "argument_hint".to_string(),
        minijinja::Value::from_serialize(&skill.argument_hint),
    );
    ctx.insert(
        "arguments".to_string(),
        minijinja::Value::from_serialize(&skill.arguments),
    );
    ctx.insert(
        "disable_model_invocation".to_string(),
        minijinja::Value::from_serialize(skill.disable_model_invocation),
    );
    ctx.insert(
        "user_invocable".to_string(),
        minijinja::Value::from_serialize(skill.user_invocable),
    );
    ctx.insert(
        "disallowed_tools".to_string(),
        minijinja::Value::from_serialize(&skill.disallowed_tools),
    );
    ctx.insert(
        "model_override".to_string(),
        minijinja::Value::from_serialize(&skill.model_override),
    );
    ctx.insert(
        "effort".to_string(),
        minijinja::Value::from_serialize(&skill.effort),
    );
    ctx.insert(
        "context_fork".to_string(),
        minijinja::Value::from_serialize(skill.context_fork),
    );
    ctx.insert(
        "agent".to_string(),
        minijinja::Value::from_serialize(&skill.agent),
    );
    ctx.insert(
        "hooks".to_string(),
        minijinja::Value::from_serialize(&skill.hooks),
    );
    ctx.insert(
        "activation_paths".to_string(),
        minijinja::Value::from_serialize(&skill.activation_paths),
    );
    ctx.insert(
        "shell".to_string(),
        minijinja::Value::from_serialize(&skill.shell),
    );
    ctx.insert(
        "required_capabilities".to_string(),
        minijinja::Value::from_serialize(&skill.required_capabilities),
    );

    for (k, v) in skill.variables_for_harness(&pair.harness.id) {
        ctx.insert(k, minijinja::Value::from_serialize(&v));
    }

    ctx.insert(
        "harness".to_string(),
        build_harness_value(&pair.harness, skill.harness_overrides.get(&pair.harness.id)),
    );

    ctx
}

fn build_harness_value(
    harness: &HarnessDefinition,
    override_: Option<&crate::types::HarnessOverride>,
) -> minijinja::Value {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert("id".to_string(), harness.id.clone());
    map.insert("name".to_string(), harness.name.clone());
    if let Some(ref v) = harness.version {
        map.insert("version".to_string(), v.clone());
    }
    if let Some(ref v) = harness.skill_ref_pattern {
        map.insert("skill_ref_pattern".to_string(), v.clone());
    }

    for (name, def) in &harness.macros {
        let content = match def {
            MacroDef::Inline(s) => s.clone(),
            MacroDef::Function { content, .. } => content.clone(),
        };
        map.insert(name.clone(), content);
    }

    // Skill-scoped macro overrides win over the harness's own builtin macro of the
    // same name — these only apply to this one skill, not the harness definition
    // globally (schema: "Harness-specific macro overrides for this skill only").
    if let Some(override_) = override_ {
        for (name, content) in &override_.macros {
            map.insert(name.clone(), content.clone());
        }
    }

    minijinja::Value::from_serialize(&map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;

    #[test]
    fn context_includes_skill_name() {
        let registry = HarnessRegistry::with_builtins();
        let skill = crate::resolver::tests::test_skill("my-agent", vec![]);
        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        assert_eq!(
            ctx.get("skill_name").and_then(|v| v.as_str()),
            Some("my-agent")
        );
    }

    #[test]
    fn context_includes_every_skill_metadata_field() {
        let registry = HarnessRegistry::with_builtins();
        let skill = crate::resolver::tests::test_skill("my-agent", vec![]);
        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        for field in crate::types::SKILL_METADATA_FIELDS {
            assert!(
                ctx.contains_key(*field),
                "build_context() is missing key {field:?} listed in SKILL_METADATA_FIELDS"
            );
        }
    }

    #[test]
    fn context_includes_skill_metadata_values() {
        let registry = HarnessRegistry::with_builtins();
        let mut skill = crate::resolver::tests::test_skill("my-agent", vec![]);
        skill.license = Some("Apache-2.0".to_string());
        skill.when_to_use = Some("when testing things".to_string());
        skill.allowed_tools = Some("Read, Grep".to_string());
        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        assert_eq!(
            ctx.get("license").and_then(minijinja::Value::as_str),
            Some("Apache-2.0")
        );
        assert_eq!(
            ctx.get("when_to_use").and_then(minijinja::Value::as_str),
            Some("when testing things")
        );
        assert_eq!(
            ctx.get("allowed_tools").and_then(minijinja::Value::as_str),
            Some("Read, Grep")
        );
    }

    #[test]
    fn context_includes_harness_object() {
        let registry = HarnessRegistry::with_builtins();
        let skill = crate::resolver::tests::test_skill("test", vec![]);
        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        let harness = ctx.get("harness").unwrap();
        let id_key = minijinja::Value::from("id");
        let name_key = minijinja::Value::from("name");
        assert_eq!(
            harness.get_item(&id_key).unwrap().as_str(),
            Some("opencode")
        );
        assert_eq!(
            harness.get_item(&name_key).unwrap().as_str(),
            Some("OpenCode")
        );
    }

    #[test]
    fn context_applies_harness_variable_override() {
        use crate::types::HarnessOverride;
        use std::collections::BTreeMap as Map;

        let registry = HarnessRegistry::with_builtins();
        let mut skill = crate::resolver::tests::test_skill("greeter", vec![]);
        skill.variables.insert(
            "greeting".to_string(),
            yaml_serde::Value::String("hi".into()),
        );
        let mut claude_vars = Map::new();
        claude_vars.insert(
            "greeting".to_string(),
            yaml_serde::Value::String("hi-claude".into()),
        );
        skill.harness_overrides.insert(
            "claude".to_string(),
            HarnessOverride {
                variables: claude_vars,
                macros: Map::new(),
            },
        );

        let claude_pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let claude_ctx = build_context(&claude_pair);
        assert_eq!(
            claude_ctx.get("greeting").and_then(|v| v.as_str()),
            Some("hi-claude"),
            "claude has an override — should win over the top-level default"
        );

        let opencode_pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry)
                .unwrap();
        let opencode_ctx = build_context(&opencode_pair);
        assert_eq!(
            opencode_ctx.get("greeting").and_then(|v| v.as_str()),
            Some("hi"),
            "opencode has no override — should fall back to the top-level default"
        );
    }

    #[test]
    fn context_applies_harness_macro_override() {
        use crate::types::HarnessOverride;
        use std::collections::BTreeMap as Map;

        let registry = HarnessRegistry::with_builtins();
        let mut skill = crate::resolver::tests::test_skill("noted", vec![]);
        let mut claude_macros = Map::new();
        claude_macros.insert("extra_note".to_string(), "Claude-only note".to_string());
        skill.harness_overrides.insert(
            "claude".to_string(),
            HarnessOverride {
                variables: Map::new(),
                macros: claude_macros,
            },
        );

        let claude_pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let claude_ctx = build_context(&claude_pair);
        let harness = claude_ctx.get("harness").unwrap();
        let key = minijinja::Value::from("extra_note");
        assert_eq!(
            harness.get_item(&key).unwrap().as_str(),
            Some("Claude-only note")
        );

        let opencode_pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "opencode", &registry)
                .unwrap();
        let opencode_ctx = build_context(&opencode_pair);
        let opencode_harness = opencode_ctx.get("harness").unwrap();
        assert!(
            opencode_harness.get_item(&key).unwrap().is_undefined(),
            "the override is scoped to claude only — opencode shouldn't see extra_note"
        );
    }

    #[test]
    fn context_harness_macro_override_wins_over_builtin() {
        use crate::types::HarnessOverride;
        use std::collections::BTreeMap as Map;

        let registry = HarnessRegistry::with_builtins();
        let mut skill = crate::resolver::tests::test_skill("overridden", vec![]);
        let mut claude_macros = Map::new();
        claude_macros.insert(
            "subagent_guide".to_string(),
            "Custom subagent guidance for this skill only.".to_string(),
        );
        skill.harness_overrides.insert(
            "claude".to_string(),
            HarnessOverride {
                variables: Map::new(),
                macros: claude_macros,
            },
        );

        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        let harness = ctx.get("harness").unwrap();
        let key = minijinja::Value::from("subagent_guide");
        assert_eq!(
            harness.get_item(&key).unwrap().as_str(),
            Some("Custom subagent guidance for this skill only.")
        );
    }

    #[test]
    fn context_includes_skill_variables() {
        use std::collections::BTreeMap;
        let registry = HarnessRegistry::with_builtins();
        let mut vars = BTreeMap::new();
        vars.insert(
            "theme".to_string(),
            yaml_serde::Value::String("dark".into()),
        );
        let mut skill = crate::resolver::tests::test_skill("styled", vec![]);
        skill.variables = vars;
        let pair =
            crate::resolver::HarnessResolver::resolve_skill_harness(&skill, "claude", &registry)
                .unwrap();
        let ctx = build_context(&pair);
        assert_eq!(ctx.get("theme").and_then(|v| v.as_str()), Some("dark"));
    }
}

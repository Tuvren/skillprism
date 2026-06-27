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
/// Populates `skill_name`, `skill_description`, skill variables, and the
/// `harness` object (including macros) for use in Jinja2 templates.
pub fn build_context(pair: &ResolvedPair) -> BTreeMap<String, minijinja::Value> {
    let mut ctx = BTreeMap::new();

    ctx.insert("skill_name".to_string(), pair.skill.name.clone().into());
    ctx.insert(
        "skill_description".to_string(),
        pair.skill.description.clone().into(),
    );

    for (k, v) in &pair.skill.variables {
        ctx.insert(k.clone(), minijinja::Value::from_serialize(v));
    }

    ctx.insert("harness".to_string(), build_harness_value(&pair.harness));

    ctx
}

fn build_harness_value(harness: &HarnessDefinition) -> minijinja::Value {
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

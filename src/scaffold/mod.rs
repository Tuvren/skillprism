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

pub mod harness;
pub mod project;
pub mod skill;

const PROJECT_SCHEMA_URL: &str = "https://tuvren.github.io/skillprism/schema/v1/skillprism.json";
const SKILL_SCHEMA_URL: &str = "https://tuvren.github.io/skillprism/schema/v1/skill.json";
const HARNESS_SCHEMA_URL: &str = "https://tuvren.github.io/skillprism/schema/v1/harness.json";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_urls_match_published_ids() {
        for (url, source) in [
            (
                PROJECT_SCHEMA_URL,
                include_str!("../../schemas/project-config-schema.json"),
            ),
            (
                SKILL_SCHEMA_URL,
                include_str!("../../schemas/skill-schema.json"),
            ),
            (
                HARNESS_SCHEMA_URL,
                include_str!("../../schemas/harness-schema.json"),
            ),
        ] {
            let schema: serde_json::Value = serde_json::from_str(source).unwrap();
            assert_eq!(schema["$id"], url);
        }
    }
}

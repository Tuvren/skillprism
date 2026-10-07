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

use std::fmt;

use miette::{Diagnostic, NamedSource, SourceSpan};
use serde::de::{DeserializeSeed, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use yaml_serde::Value;

use crate::resolver::ResolvedPair;

/// Invalid frontmatter in a rendered skill, with locations in the generated text.
#[derive(Debug, Diagnostic, Error)]
#[error(
    "[{skill}] {harness}: Invalid rendered SKILL.md frontmatter{field} from template `{template}` at frontmatter line {line} (rendered line {rendered_line}): {detail}"
)]
#[diagnostic(help(
    "Apply the | yaml_str filter to frontmatter string values, for example: description: {{{{ skill_description | yaml_str }}}}. The frontmatter must be a YAML mapping between --- fences."
))]
pub struct FrontmatterError {
    skill: String,
    harness: String,
    template: String,
    field: String,
    line: usize,
    rendered_line: usize,
    detail: String,
    #[source_code]
    src: NamedSource<String>,
    #[label("invalid rendered frontmatter value")]
    span: Option<SourceSpan>,
}

pub(super) fn check(pair: &ResolvedPair, rendered: &str) -> Result<(), Box<FrontmatterError>> {
    let without_bom = rendered.strip_prefix('\u{feff}').unwrap_or(rendered);
    let mut lines = without_bom.split_inclusive('\n');
    let Some(opening) = lines.next().filter(|line| is_fence(line)) else {
        return Ok(());
    };
    // The BOM adds bytes to source spans, but does not add a rendered line.
    let start = rendered.len() - without_bom.len() + opening.len();
    let mut end = start;
    let mut closed = false;
    for line in lines {
        if is_fence(line) {
            closed = true;
            break;
        }
        end += line.len();
    }
    let frontmatter = &rendered[start..end];
    if !closed {
        return Err(diagnostic(
            pair,
            rendered,
            start,
            1,
            None,
            "Missing closing --- frontmatter fence".to_owned(),
        ));
    }

    let value: Value = yaml_serde::from_str(frontmatter).map_err(|error| {
        let location = error.location();
        let line = location.as_ref().map_or(1, yaml_serde::Location::line);
        let field = block_field_at(frontmatter, line);
        diagnostic(
            pair,
            rendered,
            start + location.as_ref().map_or(0, yaml_serde::Location::index),
            line,
            field.as_deref(),
            error.to_string(),
        )
    })?;
    if !value.is_mapping() {
        return Err(diagnostic(
            pair,
            rendered,
            start,
            1,
            None,
            "Frontmatter must be a YAML mapping".to_owned(),
        ));
    }

    // Value is authoritative for YAML scalar types. This second parse retains
    // parser locations and field paths while rejecting scalar-to-string coercion.
    yaml_serde::from_str::<StringFields>(frontmatter).map_err(|error| {
        let detail = error.to_string();
        let field = ["name", "description"]
            .into_iter()
            .find(|field| detail.starts_with(&format!("{field}:")));
        let location = error.location();
        diagnostic(
            pair,
            rendered,
            start + location.as_ref().map_or(0, yaml_serde::Location::index),
            location.as_ref().map_or(1, yaml_serde::Location::line),
            field,
            detail,
        )
    })?;
    Ok(())
}

fn is_fence(line: &str) -> bool {
    line.trim_end_matches(['\r', '\n']) == "---"
}

fn diagnostic(
    pair: &ResolvedPair,
    rendered: &str,
    offset: usize,
    line: usize,
    field: Option<&str>,
    detail: String,
) -> Box<FrontmatterError> {
    let span = rendered
        .get(offset..)
        .map(|tail| (offset, tail.chars().next().map_or(0, char::len_utf8)).into());
    Box::new(FrontmatterError {
        skill: pair.skill.name.clone(),
        harness: pair.harness.id.clone(),
        template: pair.skill.template_path.to_string_lossy().into_owned(),
        field: field.map_or_else(String::new, |field| format!(" field `{field}`")),
        line,
        rendered_line: line + 1,
        detail,
        src: NamedSource::new(
            format!("rendered {}/SKILL.md", pair.harness.id),
            rendered.to_owned(),
        ),
        span,
    })
}

// Syntax errors have no deserialized field path. Identify an enclosing block
// field only when its key parses as a string; other forms keep a file location.
fn block_field_at(frontmatter: &str, line: usize) -> Option<String> {
    frontmatter
        .lines()
        .take(line)
        .filter_map(|text| {
            if text.starts_with(char::is_whitespace) {
                return None;
            }
            text.match_indices(':').find_map(|(offset, _)| {
                if !text[offset + 1..].starts_with(char::is_whitespace) && offset + 1 != text.len()
                {
                    return None;
                }
                yaml_serde::from_str::<Value>(&text[..offset])
                    .ok()?
                    .as_str()
                    .map(str::to_owned)
            })
        })
        .last()
}

struct StringFields;

impl<'de> Deserialize<'de> for StringFields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(Self)
    }
}

impl<'de> Visitor<'de> for StringFields {
    type Value = Self;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML mapping")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self, A::Error> {
        while let Some(key) = map.next_key::<Value>()? {
            if matches!(key.as_str(), Some("name" | "description")) {
                map.next_value_seed(StrictString)?;
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(Self)
    }
}

struct StrictString;

impl<'de> DeserializeSeed<'de> for StrictString {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl Visitor<'_> for StrictString {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML string")
    }

    fn visit_str<E: serde::de::Error>(self, _value: &str) -> Result<(), E> {
        Ok(())
    }
}

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

use std::fmt::Write;

use minijinja::{Environment, Value};

/// Registers custom Jinja2 helper functions and filters into the rendering environment,
/// and configures rendering options shared by every render call site.
///
/// `skill_ref_pattern` is the current harness's `skill_ref_pattern` (e.g. `"/{name}"`).
/// The `skill_ref` helper substitutes `{name}` in that pattern; when the harness has no
/// pattern, it falls back to `"/{name}"`.
pub fn register_helpers(env: &mut Environment, skill_ref_pattern: Option<&str>) {
    // MiniJinja defaults to Jinja2's `keep_trailing_newline=False`, silently dropping
    // the final newline of every rendered file. Markdown/source files are
    // conventionally newline-terminated, so without this every skillprism build would
    // strip the trailing newline its own source template ended with.
    env.set_keep_trailing_newline(true);
    env.add_function("skill_ref", make_skill_ref(skill_ref_pattern));
    env.add_filter("yaml_str", yaml_str);
}

/// Stringifies a value using `MiniJinja`'s display rules and double-quotes it as a YAML scalar.
fn yaml_str(value: &Value) -> Value {
    let text = value.to_string();
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for ch in text.chars() {
        match ch {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            // Escape Unicode line separators to avoid parser-specific line folding,
            // and the two BMP noncharacters excluded from YAML's printable set.
            ch if ch.is_control()
                || matches!(ch, '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}') =>
            {
                write!(quoted, "\\u{:04X}", u32::from(ch))
                    .expect("writing to a String cannot fail");
            }
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    // YAML/JSON template filenames enable MiniJinja's automatic JSON escaping.
    // This scalar is already escaped; suppress a second round of quoting.
    Value::from_safe_string(quoted)
}

/// Placeholder token inside a harness's `skill_ref_pattern` that the helper replaces
/// with the referenced skill's name.
const NAME_PLACEHOLDER: &str = "{name}";

/// Builds the `skill_ref` helper bound to one harness's `skill_ref_pattern`.
fn make_skill_ref(
    skill_ref_pattern: Option<&str>,
) -> impl Fn(&str) -> String + Send + Sync + 'static {
    let pattern = skill_ref_pattern.unwrap_or("/{name}").to_string();
    move |name: &str| pattern.replace(NAME_PLACEHOLDER, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HELPER_FUNCTIONS;
    use minijinja::Environment;
    use std::collections::BTreeSet;

    #[test]
    fn skill_ref_uses_default_pattern_when_unset() {
        let mut env = Environment::new();
        register_helpers(&mut env, None);
        env.add_template("t.j2", "{{ skill_ref(name) }}").unwrap();
        let tmpl = env.get_template("t.j2").unwrap();
        let result = tmpl
            .render(minijinja::context! { name => "my-agent" })
            .unwrap();
        assert_eq!(result, "/my-agent");
    }

    #[test]
    fn skill_ref_uses_harness_pattern() {
        let mut env = Environment::new();
        register_helpers(&mut env, Some("@{name}"));
        env.add_template("t.j2", "{{ skill_ref(name) }}").unwrap();
        let tmpl = env.get_template("t.j2").unwrap();
        let result = tmpl
            .render(minijinja::context! { name => "other" })
            .unwrap();
        assert_eq!(result, "@other");
    }

    #[test]
    fn helper_functions_constant_matches_registration() {
        let builtin_env = Environment::new();
        let builtins: BTreeSet<&str> = builtin_env.globals().map(|(name, _)| name).collect();
        let mut env = Environment::new();
        register_helpers(&mut env, None);
        let registered: BTreeSet<&str> = env.globals().map(|(name, _)| name).collect();
        // Exclude MiniJinja's own globals so extra custom helpers also fail this check.
        let helpers: BTreeSet<&str> = registered.difference(&builtins).copied().collect();
        let expected: BTreeSet<&str> = HELPER_FUNCTIONS.iter().copied().collect();
        assert_eq!(helpers, expected);
    }

    #[test]
    fn trailing_newline_in_source_template_is_preserved() {
        let mut env = Environment::new();
        register_helpers(&mut env, None);
        env.add_template("t.j2", "# {{ name }}\n").unwrap();
        let tmpl = env.get_template("t.j2").unwrap();
        let result = tmpl.render(minijinja::context! { name => "test" }).unwrap();
        assert_eq!(result, "# test\n");
    }
}

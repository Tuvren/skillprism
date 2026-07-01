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

use minijinja::Environment;

/// Registers custom Jinja2 helper functions into the rendering environment, and
/// configures rendering options shared by every render call site.
pub fn register_helpers(env: &mut Environment) {
    // MiniJinja defaults to Jinja2's `keep_trailing_newline=False`, silently dropping
    // the final newline of every rendered file. Markdown/source files are
    // conventionally newline-terminated, so without this every skillprism build would
    // strip the trailing newline its own source template ended with.
    env.set_keep_trailing_newline(true);
    env.add_function("skill_ref", skill_ref);
}

fn skill_ref(name: &str) -> String {
    format!("/{name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use minijinja::Environment;

    #[test]
    fn skill_ref_formats_correctly() {
        assert_eq!(skill_ref("my-agent"), "/my-agent");
        assert_eq!(skill_ref("test"), "/test");
    }

    #[test]
    fn skill_ref_works_in_template() {
        let mut env = Environment::new();
        register_helpers(&mut env);
        env.add_template("t.j2", "{{ skill_ref(name) }}").unwrap();
        let tmpl = env.get_template("t.j2").unwrap();
        let result = tmpl
            .render(minijinja::context! { name => "my-agent" })
            .unwrap();
        assert_eq!(result, "/my-agent");
    }

    #[test]
    fn trailing_newline_in_source_template_is_preserved() {
        let mut env = Environment::new();
        register_helpers(&mut env);
        env.add_template("t.j2", "# {{ name }}\n").unwrap();
        let tmpl = env.get_template("t.j2").unwrap();
        let result = tmpl.render(minijinja::context! { name => "test" }).unwrap();
        assert_eq!(result, "# test\n");
    }
}

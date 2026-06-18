use minijinja::Environment;

pub fn register_helpers(env: &mut Environment) {
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
}

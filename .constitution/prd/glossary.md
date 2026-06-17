# Glossary

| Term | Definition | Do Not Use |
| :--- | :--- | :--- |
| Harness | An agent platform target (e.g., Claude Code, Codex, OpenCode) that skillprism generates output for | target, platform, runtime, agent |
| Skill | A portable agent capability definition conforming to the open Agent Skills specification (SKILL.md with YAML frontmatter) | plugin, command, tool, action |
| Template | A SKILL.md.j2 file containing MiniJinja syntax and harness-aware macro references that skillprism compiles into harness-specific SKILL.md files | source, blueprint, pattern |
| Macro | A named content block defined in a harness definition YAML file that is referenced in templates to emit harness-specific content | snippet, fragment, partial |
| Sidecar | A harness-specific companion file generated alongside SKILL.md (e.g., Codex agents/openai.yaml) | metadata file, extra file, supplement |
| Plugin manifest | A harness-specific discovery configuration file that registers skills with an agent (e.g., Claude's marketplace.json) | registry, index, catalog |
| Build | The act of compiling templates into harness-specific output files via skillprism's `build` command | compile, generate, render |
| Harness definition | A YAML file describing one harness's identity, capabilities, installation paths, macros, and sidecar templates | config, spec, profile |
| Variable | A named value defined in skill.yaml that templates can reference via MiniJinja syntax for harness-aware rendering | parameter, option, setting |
| Group-level variable | A variable defined in a parent directory's skill.yaml that is inherited by all skills in that group | shared variable, global variable, inherited value |

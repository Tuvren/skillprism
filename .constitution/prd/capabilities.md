# Capabilities

## Epic: Template Compilation

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P0 | TC-1 | Compile a template into a harness-specific SKILL.md file, resolving macro references and variable substitutions from the harness definition and skill configuration. | Core function of the tool — every other capability depends on this. |
| P0 | TC-2 | Copy shared assets (references/, scripts/) from the skill source to each harness output directory unchanged. | Skills depend on these files; they must not be duplicated or modified. |
| P0 | TC-3 | Merge group-level variables with per-skill variables (skill wins) before rendering each template. | Enables shared configuration for skill groups while allowing individual overrides. |
| P0 | TC-4 | Generate harness-specific sidecar files from inline templates defined in the harness definition. | Harnesses like Codex require companion metadata files alongside each skill. |
| P0 | TC-5 | Generate harness-specific plugin manifests that register skills with the agent's discovery system. | Harnesses require manifests (e.g., marketplace.json) for skills to be discoverable. |
| P0 | TC-6 | Fail the build on any missing macro reference or undefined template variable. | Prevents deploying broken skills; catches errors at build time, not at agent invocation. |

## Epic: Harness Support

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P0 | HS-1 | Ship built-in harness definitions for Claude Code, Codex, OpenCode, Factory, and Pi that cover skill format, installation paths, subagent API patterns, invocation syntax, frontmatter fields, sidecar requirements, and validation strictness. | Covers the five most common agent platforms out of the box based on real-world validation. |
| P0 | HS-2 | Allow users to override a built-in harness definition by placing a harnesses/{name}.yaml file in the project root. | Power users can customize harness behavior without forking the tool. |
| P1 | HS-3 | Allow users to define entirely new harnesses (not overriding an existing one) by placing harnesses/{name}.yaml in the project root. | Enables support for niche or future agent platforms without waiting for the tool to ship support. |

## Epic: Build & Deployment

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P0 | BD-1 | Write all generated output to a dist/{harness}/ directory by default, with each subdirectory mirroring the exact layout that harness expects. | Enables safe inspection and diff before deployment; avoids overwriting existing installations. |
| P0 | BD-2 | Accept a --target flag (project | user) that deploys generated output to the agent's installation path instead of dist/. | One-command deployment to the correct directory for the selected scope. |

## Epic: Validation

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P0 | VA-1 | Provide a validate command that checks template syntax, variable definitions, and macro references without writing output. | Enables CI integration and authoring-time verification without side effects. |

## Epic: Scaffolding

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P1 | SC-1 | Provide an init command that scaffolds a new skillprism project: skillprism.yaml, a sample skill template, and a default harnesses/ directory. | Reduces time-to-first-build for new users; establishes the correct project layout. |
| P1 | SC-2 | Provide an init command variant that scaffolds a single skill within an existing project. | Enables incremental adoption — add skills one at a time to an existing skills repository. |

## Epic: Observability

| Priority | ID | Capability | Rationale |
| :--- | :--- | :--- | :--- |
| P0 | OB-1 | Generate build output that can be diffed against the currently installed state before the user runs --target deploy. | Prevents accidental overwrites; enables review workflow for team environments. |

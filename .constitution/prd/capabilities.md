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
| P0 | BD-1 | Write all generated output directly to harness-specific agent paths (project scope by default), with each target directory mirroring the exact layout that harness expects. | Skills are immediately usable after build — no separate deploy step. Matches the install-to-path model established by the skills CLI. |
| P0 | BD-2 | Accept a --target flag (project | user | dist) that reroutes output to project-level agent paths, user-level (global) agent paths, or a dist/ directory for inspection. | One-command targeting for the correct scope; dist/ enables CI artifact inspection without touching live agent paths. |

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
| P0 | OB-1 | Generate a diff preview that compares rendered output against whatever currently exists at the target paths, without writing anything. | Prevents accidental overwrites; enables review workflow for team environments. |

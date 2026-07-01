# Examples

Three skills, one project, meant to show off skillprism's own mechanics — this isn't a
ready-to-use skill library, and none of them should be copied verbatim into a real
project. All three are a real `skillprism build` target and a real `cargo test`
fixture (`tests/examples.rs`), not documentation-only snippets.

## The skills

- **`skills/quickstart/`** — small and entirely synthetic, meant to be read
  end-to-end in one sitting. It doesn't do anything; it exists purely to exercise
  every skillprism-specific mechanism at least once in one short file: the built-ins
  (`skill_name`, `skill_description`, `harness.id`, `harness.name`), a custom
  `variables:` entry, a builtin harness macro (`{{ harness.subagent_guide }}`), and
  `skill.yaml`'s per-skill `harnesses:` block overriding both a variable (for
  `opencode` only) and a macro (for `codex` only) — the one mechanism `mcp-builder`
  and `webapp-testing` below don't need and so leave undemonstrated. Start here if
  you're new to skillprism.
- **`skills/mcp-builder/`** — a guide for building MCP (Model Context Protocol)
  servers, ported from Anthropic's public `mcp-builder` Agent Skill. It genuinely
  needs `required-capabilities: [subagent, allowed-tools]`: the four-phase build is
  long enough to warrant running forked rather than crowding the primary
  conversation, and it only needs read/write/bash/fetch access, not a blank check.
  Since only `claude` supports `allowed-tools` among this project's three targeted
  harnesses, this skill resolves for `claude` only — `opencode`/`codex` are skipped
  with a warning, not a build failure (see "Issues found and fixed" #2 below). Its
  real upstream asset folder, `reference/` (singular), is kept exactly as-is.
- **`skills/webapp-testing/`** — a Playwright-based toolkit for testing local web
  apps, ported from Anthropic's public `webapp-testing` Agent Skill. No
  required-capabilities, so it resolves for all three harnesses. Its real upstream
  asset folder, `examples/`, is also kept exactly as-is.

`mcp-builder` and `webapp-testing` don't hand-write per-harness body text. The only
thing that varies across `claude`/`opencode`/`codex` in their rendered output is
`{{ harness.subagent_guide }}` — a builtin macro every harness defines for itself
(`src/builtin_harnesses/*.yaml`), not something invented for this example. That's
deliberate: most of what actually differs between harnesses (capability gating,
output paths, manifest aggregation, frontmatter support) is handled by skillprism
itself, not by branching inside your prose. A template only needs
`{% if harness.id == ... %}` for genuinely harness-specific instructions — neither of
these two skills needs that (`quickstart` does, once, deliberately, to show what it
looks like).

## Building

```bash
cd examples
skillprism build --target dist
find dist -type f | sort
```

Use `--target dist` here, **not** plain `skillprism build`. The default `--target
project` writes live `.claude/`, `.opencode/`, and `.agents/` directories straight into
`examples/` — those are not gitignored (only `dist/` is, via the repo's root
`.gitignore`), so a plain build would leave generated output sitting in your working
tree as untracked files.

Expect a `[resolve] skipped: ...` warning on stderr for each of the two
`mcp-builder`/`opencode` and `mcp-builder`/`codex` pairs, then a successful build with
7 rendered `SKILL.md` files (`mcp-builder` × claude only; `webapp-testing` and
`quickstart` × all three), the asset copies under each skill's output directory, and
two aggregated manifests: `dist/claude/.claude/plugin.json` (references all three
skills) and `dist/codex/.agents/marketplace.json` (references `webapp-testing` and
`quickstart`, not the skipped `mcp-builder`). `opencode` gets no manifest at all
(`requires_manifest: false` in its harness definition). Note the asymmetry: rendered
skill files land at `dist/<harness>/<skill>/SKILL.md` (`project_scope_path` is dropped
entirely under `--target dist` — `src/router/paths.rs::resolve_skill_path`), while
manifests land at `dist/<harness>/<manifest_scope_path>/<filename>` (the scope path
*is* applied — `src/router/paths.rs::resolve_manifest_path`). That's pre-existing,
deliberate `Dist` behavior, not a bug in this example.

## Attribution

`skills/quickstart/` is original content written for this repository — nothing to
attribute, nothing ported.

`skills/mcp-builder/` and `skills/webapp-testing/` are adapted from Anthropic's public
Agent Skills repository: <https://github.com/anthropics/skills>, pinned at commit
`35414756ca55738e050562e272a6bbc6273aa926`. Both source skills are licensed under the
Apache License, Version 2.0, © Anthropic, PBC — see each skill's `metadata.source` /
`metadata.upstream_license` field in its `skill.yaml`.

Changes made when porting:
- Each skill's single `SKILL.md` was split into skillprism's canonical
  `skill.yaml` (metadata) + `SKILL.md` (Jinja2 template) source format.
- `mcp-builder`'s body was condensed from the upstream four-phase guide; some
  reference material was trimmed — only `reference/mcp_best_practices.md` was kept as
  a full verbatim copy.
- Harness-conditional sections (`{% if harness.id == ... %}`) were added; they don't
  exist upstream, since the original skills only ever targeted Claude.
- Both upstream asset-folder names were kept as-is (`reference/`, `examples/`) rather
  than renamed to fit any particular convention — see #3 below for why that's safe.

(`anthropics/skills` also publishes `pdf`, `docx`, `pptx`, and `xlsx` — explicitly
**source-available, not open source**, with a `LICENSE.txt` that forbids reproduction
and derivative works. Those were deliberately excluded from this port.)

## Issues found and fixed while porting these skills

Authoring `mcp-builder` and `webapp-testing` the way the schema documents (not the
minimal way the existing `tests/fixtures/valid` fixture or `skillprism init skill`
scaffold do) surfaced three gaps between skillprism's documented contract and its
implementation (#1–#3 below). A fourth (#4) surfaced afterward, from a review question
about why these examples used `variables:` at all — the answer led to checking whether
the schema's "variables that genuinely differ by harness" mechanism was actually
implemented; it wasn't. All four were fixed as part of the same overall change that
added this directory; `quickstart` was added later specifically to demonstrate #4,
which otherwise had no example anywhere in this directory. `tests/examples.rs` and
unit tests in `src/loader`, `src/engine`, and `src/validator` regression-test all four
fixes, so this section stays honest as the code evolves.

1. **Most `skill.yaml` metadata fields never reached rendered output — fixed.**
   `src/loader/project.rs` parses `license`, `compatibility`, `metadata`,
   `allowed-tools`, `when_to_use`, `argument-hint`, `arguments`,
   `disable-model-invocation`, `user-invocable`, `disallowed-tools`, `model`, `effort`,
   `context`, `agent`, `hooks`, `paths`, and `shell` into `SkillModel`. The schema
   itself says several of them "map to SKILL.md frontmatter"
   (`.constitution/tech-spec/contracts/skill-schema.json`), but
   `src/engine/context.rs::build_context` only ever inserted `skill_name`,
   `skill_description`, each `variables` entry, and `harness` into the template
   context — none of the rest. **Fix:** `build_context` now inserts all of them under
   their `SkillModel` field names (`license`, `allowed_tools`, `when_to_use`,
   `metadata`, `version`, etc. — see `types::SKILL_METADATA_FIELDS`, the single list
   shared by `build_context` and `validator::variables::is_builtin` so the two can't
   drift apart; `engine::context::tests::context_includes_every_skill_metadata_field`
   guards against that). Both skills reference `{{ license }}` and `{{ when_to_use }}`
   directly in their `SKILL.md` — `mcp-builder` also uses `{{ allowed_tools }}` in
   its frontmatter, with no workaround needed. Note that not every field a skill
   declares has to be echoed into the rendered body to be "used" — `version` and
   `metadata.*` reach the context too (proven by the unit test above) but exist here
   purely as packaging/attribution metadata; a real skill body shouldn't recite its own
   version number just to prove a schema field works.
   `tests/examples.rs::examples_skill_metadata_fields_render_correctly` asserts the
   fields these skills do use render with the configured values.

2. **One project-wide harness list; any capability mismatch aborted the whole build —
   fixed.** `src/resolver/mod.rs::resolve_project` loops every skill × every
   project-configured harness; it used to collect *all* errors (including capability
   mismatches) into one bucket and abort the *entire* build if that bucket was
   non-empty — not just the one incompatible pair. **Fix:** `resolve_project` now
   returns a `ResolveOutcome { resolved, skipped, fatal }`, mirroring
   `Validator::validate`'s existing accumulate-don't-abort pattern. An unknown harness
   name in `skillprism.yaml` (a real project misconfiguration) is still fatal and
   aborts the build. A capability mismatch is now non-fatal: that one skill-harness
   pair is skipped, a `[resolve] skipped: ...` warning is printed, and every other
   pair still builds. `mcp-builder` demonstrates this directly: it requires
   `allowed-tools`, which only `claude` supports among this project's three targeted
   harnesses, so it resolves for `claude` only — `opencode`/`codex` are skipped with a
   warning instead of failing the whole build. (Separately, the schema also documents a
   per-skill `harnesses:` block for per-harness `variables`/`macros` overrides — not
   the same mechanism as this fix and not what makes `mcp-builder`'s skip work, but
   worth noting because it's a different kind of harness-scoping. It was unimplemented
   when this finding was first written up; see #4 below.)
   `tests/examples.rs::examples_build_succeeds_and_skips_incompatible_pairs` and
   `examples_manifests_reflect_resolved_pairs_only` assert the new behavior.

3. **Asset-directory discovery was hardcoded to exactly two names, silently — fixed.**
   `src/loader/project.rs::load_skill` used to only check for a sibling directory
   named exactly `references/` or `scripts/`; nothing else under a skill directory was
   inspected at all, with no warning if you got the name wrong. Both upstream skills
   reproduced this naturally: `mcp-builder`'s real asset folder is `reference/`
   (**singular**) and `webapp-testing`'s is `examples/` — neither matched the
   hardcoded plural `references/`. **Fix:** `load_skill` now treats every direct
   subdirectory of a skill's own directory as an asset directory to copy verbatim,
   regardless of name — `walk_directory` never recurses into a skill's own directory
   looking for nested skills/groups once its template (`SKILL.md` or `SKILL.md.j2`)
   has been found, so nothing there can be mistaken for one. Both upstream
   asset-folder names (`reference/`,
   `examples/`) are now kept exactly as the original skills shipped them, with no
   renaming needed to make them work.
   `tests/examples.rs::examples_asset_copy_matches_skillprism_convention` asserts both
   are copied byte-for-byte.

4. **`variables:` couldn't actually differ by harness, despite the schema documenting a
   mechanism for it — fixed.** A review pass on this directory caught both skills using
   `variables:` for content that never genuinely needed to differ by harness
   (`recommended_language`, `default_dev_port`, etc.) — and that was the smell that
   surfaced this: when asked to justify it, there was no justification, because
   `src/engine/context.rs::build_context` only ever inserted `skill.variables` as one
   flat, harness-invariant map (confirmed by grep — no harness branching anywhere near
   it). The schema documents exactly the missing mechanism: a per-skill `harnesses:`
   block (`.constitution/tech-spec/contracts/skill-schema.json`) where
   `harnesses.<id>.variables` is "merged with top-level variables, harness wins," and
   `harnesses.<id>.macros` overrides a harness's builtin macro *for this skill only*.
   `SkillYamlRaw` had no `harnesses` field at all — silently dropped, not even a parse
   error. **Fix:** `SkillYamlRaw` now parses `harnesses:`, populating
   `SkillModel::harness_overrides`; `SkillModel::variables_for_harness(harness_id)`
   merges the per-harness override over the top-level default (`types/project.rs`);
   `build_context` resolves both variables and `harness.*` macros against this specific
   pair's harness before falling back to the harness-invariant defaults
   (`engine::context::tests::context_applies_harness_variable_override`,
   `context_applies_harness_macro_override`,
   `context_harness_macro_override_wins_over_builtin`); and `Validator::validate_pair`
   was updated to validate against the same per-harness-resolved set, fixing a latent
   correctness bug where a variable or macro defined *only* via an override would have
   been incorrectly flagged as undefined even for the harness it was actually defined
   for (`validator::tests::harness_only_variable_not_flagged_undefined_for_its_own_harness`,
   `harness_only_macro_not_flagged_undefined_for_its_own_harness`). Neither
   `mcp-builder` nor `webapp-testing` uses this feature: their content genuinely doesn't
   need to differ by harness, and forcing a demonstration in would have repeated the
   exact mistake findings #1–#3 already corrected — implementing unused machinery one
   layer up. `skills/quickstart/skill.yaml`'s `harnesses:` block (an `opencode`
   variable override and a `codex` macro override) demonstrates the mechanism
   directly, once it existed to demonstrate — see
   `tests/examples.rs::examples_quickstart_demonstrates_harness_variable_override` and
   `examples_quickstart_demonstrates_harness_macro_override`.

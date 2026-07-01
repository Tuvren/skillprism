---
name: {{ skill_name }}
description: {{ skill_description }}
license: {{ license }}
allowed-tools: {{ allowed_tools }}
---

# {{ skill_name }}

{{ skill_description }}

Ported from Anthropic's public `mcp-builder` Agent Skill — see
`reference/mcp_best_practices.md` for the full naming/transport/security checklist and
`scripts/requirements.txt` for the Python evaluation tooling dependencies.

## When to use

{{ when_to_use }}

## Recommended stack

- **Language**: TypeScript (best SDK support, and models are reliably good at
  generating it).
- **Transport**: Streamable HTTP for remote/multi-client servers; stdio for local,
  single-session integrations.

## Workflow

### Phase 1 — Research and plan

Study the MCP specification (`https://modelcontextprotocol.io/sitemap.xml`, fetch pages
with a `.md` suffix for markdown), the relevant SDK's README, and the target service's
API docs. Decide tool naming (`{service}_{action}_{resource}`, snake_case, prefixed so
it doesn't collide with other MCP servers) and prioritize comprehensive endpoint
coverage over a handful of bespoke workflow tools.

### Phase 2 — Implement

Build shared infrastructure first (authenticated API client, error handling, response
formatting, pagination), then implement each tool with a validated input schema (Zod or
Pydantic), a defined output schema, a concise unambiguous description, and behavior
annotations (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`). See
`reference/mcp_best_practices.md` for the full naming and security checklist.

### Phase 3 — Review and test

Check for duplicated code, consistent error handling, and full type coverage. Build the
server and exercise it with the MCP Inspector (`npx @modelcontextprotocol/inspector`)
before treating it as done.

### Phase 4 — Evaluate

Write 10 independent, read-only, realistic evaluation questions that require multiple
tool calls to answer, each with a single verifiable answer. `scripts/requirements.txt`
pins the Python dependencies (`anthropic`, `mcp`) used to run evaluations like these.

{{ harness.subagent_guide }}

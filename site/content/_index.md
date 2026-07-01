---
title: "skillprism"
description: "Build once, ship spec-compliant Agent Skills to every harness from one source."
---

<section class="hero">
<h1>Build once, ship everywhere</h1>
<p class="tagline">skillprism is a build-time compiler that transforms canonical skill sources into harness-specific agent files. Write one <code>skill.yaml</code> + <code>SKILL.md</code> template, compile to Claude Code, Codex, OpenCode, Factory, and Pi — all from a single command.</p>
<div class="hero-cta">
<a href="{{% relref "docs/quickstart" %}}" class="btn btn-primary">Quickstart</a>
<a href="{{% relref "docs" %}}" class="btn btn-secondary">Read the docs</a>
</div>
</section>

## How it works

<div class="how-it-works">
<div class="step">
<div class="step-num">1</div>
<h3>Init</h3>
<p><code>skillprism init project my-skills</code> scaffolds a project with a sample skill and config.</p>
</div>
<div class="step">
<div class="step-num">2</div>
<h3>Author</h3>
<p>Write <code>skill.yaml</code> (metadata) + <code>SKILL.md</code> (MiniJinja template). One source, no per-harness copies.</p>
</div>
<div class="step">
<div class="step-num">3</div>
<h3>Build</h3>
<p><code>skillprism build</code> renders each skill once per configured harness, writing to each harness's expected paths.</p>
</div>
</div>

## Supported harnesses

<div class="harness-grid">
<div class="harness-card"><code>claude</code><p>Claude Code<br><code>.claude/skills/</code></p></div>
<div class="harness-card"><code>codex</code><p>OpenAI Codex<br><code>.agents/skills/</code></p></div>
<div class="harness-card"><code>opencode</code><p>OpenCode<br><code>.opencode/skills/</code></p></div>
<div class="harness-card"><code>factory</code><p>Factory<br><code>.factory/skills/</code></p></div>
<div class="harness-card"><code>pi</code><p>Pi<br><code>.pi/skills/</code></p></div>
</div>

## Spec compliant

Every rendered `SKILL.md` includes the YAML frontmatter (`name` + `description`) that the [Agent Skills specification](https://agentskills.io/specification) requires. `skillprism validate` enforces spec constraints — name format, length caps, description non-empty — so a successful build produces skills that load in any compatible client.

## Install

```bash
cargo install --path .
```

Or with [devenv](https://devenv.sh/):

```bash
devenv shell
```

## Get started

```bash
skillprism init project my-skills
cd my-skills
skillprism build
```

That's it — your skills are now compiled to every configured harness's directory, ready to be discovered by any compatible agent.

---
title: "skillprism"
description: "Build once, ship spec-compliant Agent Skills to every harness from one source."
---

<section class="prism-hero">
<div class="prism-hero-inner">
<h1>One source,<br><span class="amber">five harnesses.</span></h1>
<p class="tagline">skillprism is a build-time compiler. Write one <code>skill.yaml</code> + <code>SKILL.md</code> template, compile to Claude Code, Codex, OpenCode, Factory, and Pi — all from a single command. No per-harness copies, no drift.</p>
<div class="hero-cta">
<a href="/skillprism/docs/quickstart/" class="btn btn-primary">Quickstart →</a>
<a href="/skillprism/docs/" class="btn btn-secondary">Read the docs</a>
</div>

<div class="prism-diagram" aria-hidden="true">
<div class="prism-source">
<span class="source-label">SKILL.md</span><br>
skill.yaml
</div>
<div class="prism-body"></div>
<div class="prism-beam-in"></div>
<div class="prism-beams">
<div class="beam beam-1" data-label="claude"></div>
<div class="beam beam-2" data-label="codex"></div>
<div class="beam beam-3" data-label="opencode"></div>
<div class="beam beam-4" data-label="factory"></div>
<div class="beam beam-5" data-label="pi"></div>
</div>
</div>
</div>
</section>

## How it works

<div class="pipeline">
<div class="pipeline-step">
<div class="step-glyph">$ init</div>
<h3>Scaffold</h3>
<p><code>skillprism init project my-skills</code> creates a project with a sample skill and config.</p>
</div>
<div class="pipeline-step">
<div class="step-glyph">$ author</div>
<h3>Write</h3>
<p>Edit <code>skill.yaml</code> (metadata) + <code>SKILL.md</code> (MiniJinja template). One source — no per-harness copies.</p>
</div>
<div class="pipeline-step">
<div class="step-glyph">$ build</div>
<h3>Compile</h3>
<p><code>skillprism build</code> renders each skill once per configured harness, writing to each harness's expected path.</p>
</div>
</div>

## Supported harnesses

<div class="harness-grid">
<div class="harness-card"><div class="harness-dot" style="background:#d97757"></div><code>claude</code><p>Claude Code<br><code>.claude/skills/</code></p></div>
<div class="harness-card"><div class="harness-dot" style="background:#10a37f"></div><code>codex</code><p>OpenAI Codex<br><code>.agents/skills/</code></p></div>
<div class="harness-card"><div class="harness-dot" style="background:#6c8ae6"></div><code>opencode</code><p>OpenCode<br><code>.opencode/skills/</code></p></div>
<div class="harness-card"><div class="harness-dot" style="background:#a78bfa"></div><code>factory</code><p>Factory<br><code>.factory/skills/</code></p></div>
<div class="harness-card"><div class="harness-dot" style="background:#e8a838"></div><code>pi</code><p>Pi<br><code>.pi/skills/</code></p></div>
</div>

## Spec compliant

<div class="callout">
<p>Every rendered <code>SKILL.md</code> includes the YAML frontmatter (<code>name</code> + <code>description</code>) that the <a href="https://agentskills.io/specification">Agent Skills specification</a> requires. <code>skillprism validate</code> enforces spec constraints — name format, length caps, non-empty description — so a successful build produces skills that load in any compatible client.</p>
</div>

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

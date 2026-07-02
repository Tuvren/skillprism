# Architecture Strategy

**Version:** v0.2.2 (see [changelog.md](./changelog.md))

> **PRD alignment:** BD-1/BD-2 were updated in PRD v0.1.1 to reflect deploy-first (project scope default). See `.constitution/prd/capabilities.md`.

## Architectural Pattern

**Local-first compilation pipeline** (pipe-and-filter within a single static binary).

The system is a sequential pipeline of independent processing stages. Each stage transforms its input and passes it to the next. Two pipeline variants share the first three stages:

```
Full build:    Load → Resolve → Validate → Render → Route/Write
Validate only: Load → Resolve → Validate (no side effects)
```

The CLI entrypoint dispatches to the appropriate pipeline variant or directly to the Scaffolder.

## Why this pattern fits

1. **Natural decomposition** — A build-time compiler is inherently sequential: you must load configuration before you can validate it, validate before you render, render before you write. Pipe-and-filter makes the dependency chain explicit.
2. **Validate reuses build stages** — The validate command (`VA-1`) shares Load → Resolve → Validate with the build command, reducing duplication by construction.
3. **Single-binary core with distribution exception** — The `build`, `validate`, `init`, and `completions` commands run all filters in-process and have no network, no daemon, no IPC. The `add` and `update` distribution commands (Epic I) are the **single exception**: they perform network access by shelling out to `git` for shallow clones. This is the only network surface in skillprism, and it makes no persistent connections, no daemons, and no IPC. The pipeline becomes a function composition chain for the in-process subset; the distribution commands add a thin network shim that does not violate the static-binary guarantee of the rest.
4. **Collect-all-errors** — The Validate stage is a batch accumulator that inspects every skill before reporting, then passes the validated model to Render. This satisfies the operator's choice of collect-all-errors over fail-fast.

## Trade-offs Accepted

| Sacrifice | Why it's acceptable |
| :--- | :--- |
| **No concurrent rendering** — Skills render sequentially in the pipeline. | Typical projects have 5-20 skills with sub-second render times. Concurrency adds coordination complexity for no measurable gain. |
| **No hot-reload** — Harness definitions are compiled into the binary. Adding or modifying a harness requires a rebuild. | PRD constraint requires harness definitions to ship with the tool and follow its release cycle. The compiled-in approach keeps the binary self-contained. |
| **No pluggable renderers** — Only one template engine is supported. | The PRD's template engine choice is settled. Supporting multiple template engines would add abstraction overhead without demand. |
| **Synchronous pipeline** — Each stage blocks until the previous completes. | CLI tool expected to run in <1s. Async would add complexity without perceptible benefit. |

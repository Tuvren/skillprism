# Out of Scope: Runtime Skill Loading

**Context:** Determined non-goal during initial scope definition.

**Reasoning:** skillprism is a build-time tool. It generates files at build time and does not interact with agents at runtime. A runtime loader would be a fundamentally different product with different safety, performance, and observability requirements.

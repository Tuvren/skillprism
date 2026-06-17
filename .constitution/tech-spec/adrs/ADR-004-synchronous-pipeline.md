# ADR-004: Synchronous Pipeline (No Async Runtime)

**Status:** Accepted

## Context

The architecture's pipe-and-filter pipeline is sequential by design: Load → Resolve → Validate → Render → Route/Write. Each stage depends on the previous. The PRD expects sub-second build times for 5–20 skills. Concurrency would add Tokio dependency overhead.

## Decision

Keep the pipeline fully synchronous. No async runtime (Tokio, async-std, smol) is used. Each stage is a blocking function call that transforms its input and returns a Result.

## Consequences

- **Positive:** No runtime overhead (~10MB+ stripped binary savings). Simpler error handling with `?` in synchronous code. Faster compile times. Easier to debug (linear stack traces).
- **Negative:** Cannot parallelize validation across skills without adding threads manually.
- **Mitigation:** If 200+ skill projects become common, validation can be parallelized with `std::thread` scoped threads or `rayon` without adopting a full async runtime. The Validator's collect-all-errors accumulator is already thread-safe by design (Vec<SkillError>).

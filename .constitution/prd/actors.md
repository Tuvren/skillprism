# Actors

## Primary: Skill Author (Solo)

**Operating context:** An individual developer — often a solo founder, power user, or early-stage agent adopter — who maintains a personal collection of skills across 2-5 agent harnesses. They author skills for their own productivity, update them as harness APIs evolve, and occasionally share them publicly.

**Concrete goals:**
- Author a skill once and deploy it to all their agent harnesses without manual editing.
- Add a new harness to their workflow with minimal effort.
- Preview generated output before overwriting their active skills.
- Catch configuration errors at build time rather than at skill invocation time.

**Frictions:**
- Currently maintains N copies of each skill (one per harness), each diverging slightly.
- Does not track which copies are stale or which harness has which version deployed.
- Has no systematic way to revert a bad deployment.

---

## Primary: Team Lead (Team)

**Operating context:** An engineering lead or platform engineer managing a shared skill library for a team of 3-20 agent users. They maintain versioned skills, enforce consistency across the team's agent configurations, and integrate skill management into CI/CD pipelines.

**Concrete goals:**
- Maintain a canonical skill library that all team members' agents can consume.
- Ensure skill updates are reviewed and tested before reaching the team.
- Support onboarding of new harnesses as the team's agent tooling evolves.
- Prevent individual team members from deploying divergent skill versions.

**Frictions:**
- Requires a shared source of truth for skills; current copy-paste approach is unmanageable at team scale.
- No diff/review workflow for skill changes before deployment.
- Different team members use different agent harnesses, compounding duplication.

---

## Secondary: Tool Integrator

**Operating context:** A developer or community contributor who wants to add support for a new agent harness that skillprism does not yet support. This may be a niche or upcoming platform not covered by built-in harness definitions.

**Concrete goals:**
- Define a new harness by writing a single YAML file.
- Test that existing templates render correctly for this new harness.
- Optionally contribute the harness definition upstream.

**Frictions:**
- Must understand the harness definition schema and macro contract.
- No visual feedback — must build and inspect output to validate correctness.

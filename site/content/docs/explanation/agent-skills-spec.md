---
title: "The Agent Skills specification"
description: "Why skillprism checks the Agent Skills spec, and where rendering goes further than that spec."
group: "Explanation"
weight: 72
aliases:
  - /docs/spec-compliance/
---

skillprism checks the [Agent Skills specification](https://agentskills.io/specification) because the file it writes is the file an agent reads. There is no skillprism runtime. A harness that implements the spec should be able to load the output as an ordinary `SKILL.md`.

The spec describes a finished file. skillprism's source is a template, so the header an agent sees is the rendered header, not the template text. A header that is not a mapping is not a skill. The cases `build`, `add`, and `update` reject are in [Templates](../reference/templates/).

A harness may set its own name or description cap, sometimes above the portable spec and sometimes below it. Past the harness cap, the build fails for that harness. Between the spec cap and a more generous harness cap, skillprism warns: the file is legal there and may not travel. A description one harness accepts can still be too long for another. Portable and accepted by this harness are different constraints, and both are checked. The caps are in [Harness](../reference/harness/).

The rendered name and description have to be text. A name has to match a directory, and a description has to be something an agent can show. YAML can read an unquoted token as a number, a boolean, or null, and two parsers need not agree that it was text. skillprism does not coerce those values. The build fails so the header's type does not depend on who reads it. How a template forces a string is in [Templates](../reference/templates/).

Progressive disclosure is a recommendation of the spec, not a second compiler. skillprism does not rewrite that layout and does not measure it. What the build copies is in [Directories and state](../reference/directories/). The source fields are in [skill.yaml](../reference/skill-yaml/).

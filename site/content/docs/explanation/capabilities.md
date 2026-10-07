---
title: "Capabilities"
description: "Why a skill that requires a capability is skipped for a harness that lacks it."
group: "Explanation"
weight: 71
---

A harness is part of the output, not a label on an otherwise identical file. A skill can require a capability the document depends on, such as a subagent, an allowed-tools list, or a manifest. If that harness cannot carry the skill, that one pair is skipped with a warning. The other pairs in the project still render.

Writing the file anyway would leave a skill where the agent looks, describing behavior the agent will not perform. Skipping is the smaller failure. An unknown harness name is a different failure: the project configuration is broken, and resolution stops. A capability name the harness definition does not know is treated as unsupported, so that pair is skipped too.

Once a harness is eligible, an override can still change that harness's document. Gating decides whether a tree is produced. An override decides how that tree differs.

Which capability each built-in harness has is recorded in [Harness](../reference/harness/). Changing what an eligible harness renders is covered in [How to vary output per harness](../how-to/vary-output-per-harness/).

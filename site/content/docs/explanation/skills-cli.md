---
title: "skillprism and the skills CLI"
description: "Why skillprism matches skills CLI install directories without becoming that package manager."
group: "Explanation"
weight: 73
aliases:
  - /docs/comparison/
---

The [skills CLI](https://github.com/vercel-labs/skills) is a package manager for agent skills. skillprism follows skills CLI v1.7.1 where an agent would otherwise look in the wrong directory, and it stops there. The two tools can sit in the same project. They do not share a model of what a skill is.

Matching those directories means a skill installed by either tool lands where the harness already reads. The paths are in [Directories and state](../reference/directories/). A subset of flags and source forms is matched so someone who already uses the skills CLI can point either tool at a repository. skillprism keeps install options of its own. Those names are in the [CLI reference](../reference/cli/).

The boundary is the tree. The skills CLI installs one unchanged skill tree and symlinks that tree into the agent directories. skillprism renders a separate tree for each harness. A symlink to one canonical copy cannot be the default: the document for one harness is not the document for another once a template, a macro, a length cap, or a capability skip has been applied. Some harnesses also share one project directory. Identical bytes can occupy that path once. Different bytes are a collision, not a prompt to collapse both harnesses into one file. A plain `SKILL.md`, already the finished file, is copied as bytes. That is the case where the two tools agree. A templated skill is still rendered into those same directories.

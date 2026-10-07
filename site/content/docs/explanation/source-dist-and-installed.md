---
title: "Source, dist, and installed"
description: "Why one skill source is compiled into dist, and why live agent directories belong to install."
group: "Explanation"
weight: 70
aliases:
  - /docs/concepts/
---

A skill passes through three states, and they are not three names for the same files. Source is the project an author keeps: one skill, one template, and the configuration that says which harnesses that source is for. Dist is what `build` compiles from that source: a separate tree for each harness, written inside the project. Installed is what a live agent reads, in a directory that agent already owns.

Compiling and installing are different decisions. Compiling answers what each harness should receive. Installing answers whether to put that document where an agent will load it. Before v0.2.0, `build` could do both, and a compile could overwrite a working agent's skills. The split makes `build` compile-only.

`build` renders into dist and stops, so the result can be read before any agent sees it. `add` and `update` render from the skill source again, straight into the live directories. They do not publish dist. One source is not yet a file any harness can load, and writing the agent's file is not part of compiling.

skillprism writes files. It does not start an agent or load a skill into a running session.

To follow one source through a first compile, use [Create a skill and compile it](../tutorials/compile-a-skill/). To put a skill into a live directory and take it back out, use [How to install, update, and remove a skill](../how-to/install-update-remove/). The directories each state uses are in [Directories and state](../reference/directories/).

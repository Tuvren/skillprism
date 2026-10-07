---
title: "How to move pre-0.4.0 installs"
description: "Leave only the current directories for skills installed by an older skillprism."
group: "How-to guides"
weight: 25
---

Leave only the current directories for skills that an older skillprism installed. Do this before you update those skills. `update` writes the new directories and leaves the previous copies in place.

The directories that changed are in [Directories and state](../reference/directories/). Claude, Codex, and Factory user installs did not move.

From the project that holds the install:

```bash
skillprism list
```

Remove the skill while that record still lists the old files:

```bash
skillprism remove my-skill --target project
```

Pass `-g` for a user skill. Repeat for each skill whose directory changed.

Add the same skill again at the same scope, with the original source:

```bash
skillprism add owner/repo --skill my-skill --target project
```

Check `skillprism list`. The skill is under the current directory, and the old directory is gone.

If you already ran `update`, the record lists the new files and `remove` will not delete the previous copies. Delete the leftover directories yourself. Add the skill again if the new directory is missing.

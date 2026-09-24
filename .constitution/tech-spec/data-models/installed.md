# Installed skill state

The store is `installed.yaml` under the user config directory, `$XDG_CONFIG_HOME/skillprism` or `~/.config/skillprism`. The directory mode is `0o700` and the file mode is `0o600`.

`InstalledState` has `version` and `skills`. The current version is 1. A newer or older version fails the read.

Each `InstalledSkill` records the source string, the source kind, the install scope (`project` or `user`), the harnesses written, the skill format, and a per-file `sha256` list used by `update`. Records are sorted by name.

A write reads the whole file, builds the next state in memory, and replaces the file with one temp-rename. Two concurrent writers can clobber each other. v1 does not take a lock.

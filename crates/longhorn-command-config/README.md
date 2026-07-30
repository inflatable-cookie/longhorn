# longhorn-command-config

Coordinated active-preset and sparse keymap override persistence.

The crate reuses `longhorn-config` coordination, migration, recovery, atomic
publication, durability, and backup authority. It owns no filesystem root,
Tauri handler, renderer state, or product command execution.

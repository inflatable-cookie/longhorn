# `longhorn-history-tree`

Optional pure Rust forkable history authority for Longhorn.

The crate owns immutable single-parent nodes, stable branch references,
canonical child indexes, structural validation, and lossless divergent record.
It depends downward on `longhorn-history`; linear history never depends on this
crate.

The authority also plans bounded mixed undo/redo routes through lowest common
ancestors, commits them only through one consumer atomic transaction, protects
current/named/pinned lineages during deterministic leaf pruning, and accounts
for replay after bounded opaque consumer checkpoints.

Persistence and client protocols are not part of the current package surface.

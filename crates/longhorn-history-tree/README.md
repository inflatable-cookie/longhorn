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

Graph persistence uses a strict `longhorn.history-tree` envelope, deterministic
identity ordering, RFC 4648 base64 payload strings, independent exact-step
structural and consumer-payload migration, explicit byte limits, and complete
state validation before authority returns. It accepts and returns bytes only;
storage paths, writes, durability, snapshots, and recovery stay with consumers.

The optional `bindings` feature exposes the exact-v1, payload-free metadata
protocol used by `@inflatable-cookie/longhorn-history-tree` and `longhorn-tauri-history-tree`.
Default projections stay linear; branch and alternate-path pages are bounded
and explicit.

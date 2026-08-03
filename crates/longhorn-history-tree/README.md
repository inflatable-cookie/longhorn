# `longhorn-history-tree`

Optional pure Rust forkable history authority for Longhorn.

The crate owns immutable single-parent nodes, stable branch references,
canonical child indexes, structural validation, and lossless divergent record.
It depends downward on `longhorn-history`; linear history never depends on this
crate.

Navigation, retention, checkpoints, persistence, and client protocols are not
part of the current package surface.

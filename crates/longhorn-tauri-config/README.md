# longhorn-tauri-config

Tauri path mapping plus an injected config-operation command edge.

The path adapter injects lifecycle facts without app leaves. On Windows it
keeps local data and roaming shared-product data distinct: consumers supply
both `local_data_dir` and `roaming_data_dir`. macOS Application Support and
Linux XDG data supply both ordinary durable data and shared-product data.

The command assembly owns no storage policy. A consumer authority keeps
authorization, idempotency, executable transition, retention, and restore
plans, committed receipts, filesystem selection, pending-publication handling,
unlock authority, encryption providers, and durable recovery. Renderer
commands carry only built-in choices, inventoried archive hashes, explicit
domain choices, and host-issued confirmation digests.

Install only the permission groups needed by each window. Tauri capabilities
remain an outer allow-list; the injected authority must still authorize the
window label. Restore inspection belongs in the read group. Conflict planning,
ordinary restore, adapter restore, and recovery use the separate destructive
permission group.

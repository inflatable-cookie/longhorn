# longhorn-tauri-history

Narrow Tauri 2 handler assembly over a consumer-injected history authority.

The adapter exposes payload-free snapshots, bounded entry pages, checked
navigation, and non-durable revision hints. The injected consumer authority
still owns caller authorization, product mutation, rollback, and payload
meaning. Tauri capabilities grant command reachability only.

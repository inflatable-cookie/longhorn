# longhorn-storage-layout

Longhorn's storage identity, pure layout resolver, roots, and bootstrap
locator, without the config store.

The crate owns the contract 004 storage surface that needs no filesystem,
coordination, or backup machinery: `StorageIdentity`, the profile resolver,
`StorageRoots`, platform directory facts, and the fixed native locator. A
plain Rust application — a CLI with no Tauri stack — can resolve exactly the
paths a Tauri application with the same storage identity resolves by feeding
`native_platform_directory_facts()` into `resolve_storage_layout`.

`longhorn-config` consumes this crate for its store, backup, and transition
machinery. The types live here only; that crate does not re-export them.

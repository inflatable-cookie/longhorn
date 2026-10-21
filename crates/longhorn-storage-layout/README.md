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

## Resolve On Demand

`resolve_storage_root` resolves one required root from a request containing
only that root's platform fact. Workspace derives from the state root, and
optional policy and project roots are returned only when explicitly
overridden. A per-root result has no layout digest.

Hosts without Tauri can use `native_platform_directory_fact` to read only the
environment entries needed for one fact, then pass the returned partial
`PlatformDirectoryFacts` directly to `StorageLayoutRequest`. Its pure
counterpart, `native_platform_directory_fact_from`, accepts a platform and an
environment snapshot. The eager facts provider continues to collect all facts
for `resolve_storage_layout`.

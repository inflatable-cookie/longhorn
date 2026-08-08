# Loophole Storage Policy And Profile Selection

Date: 2026-08-01
Card: 103
Roadmap: g01.015

## Result

Selected and implemented `shared-product-root-v1`. Loophole keeps roaming
AppData on Windows and uses the exact stable leaf `Loophole` on macOS,
Windows, and Linux. The canonical id `com.inflatablecookie.loophole` remains
app and fixed bootstrap-locator identity. No Loophole application code changed.

## Exact Matrix

| Platform | Injected shared-data parent | Product root |
| --- | --- | --- |
| macOS | `Application Support` | `Application Support/Loophole` |
| Windows | roaming `%APPDATA%` | `%APPDATA%\Loophole` |
| Linux | `$XDG_DATA_HOME` | `$XDG_DATA_HOME/Loophole` |

Each root contains typed `config`, `data`, `state`, `cache`, `logs`,
`runtime`, and `backups` children. The resolver records stable-name leaf
provenance and `shared-product:shared-data` root provenance. It reports the
non-native cache, log, runtime, and backup consequences separately.

## Compatibility

The new `shared-data` fact is additive. Existing `PlatformDirectoryFacts::complete`
callers and the path, warning, bootstrap, and digest behavior of
`platform-native-v1`, `unified-app-root-v1`, and `portable-v1` remain
unchanged. Windows Tauri snapshots now require both local and roaming data
parents so the adapter cannot silently collapse the product choice.

Rust-to-TypeScript generation adds `shared-product-root-v1` to the checked
profile union and protocol fixture. The Poodle storage page presents the new
choice. Compatibility guards admit it for snapshot and transition commands.

## Product Contract

Longhorn contract 004 now defines the reusable profile and its sharing limit.
Loophole's Chorus storage contract records the selected matrix, exact casing,
typed children, and canonical-id locator rule. Shared filesystem placement
does not grant multi-writer authority or remote synchronization.

## Private Artifact Evidence

All artifacts are private proof inputs. Package-manager publication remains
deferred.

| Artifact | Receipt | SHA-256 |
| --- | --- | --- |
| `longhorn-config` Cargo inventory | `cargo package --list --allow-dirty` | `4cfcb5a3bca34f2caa4c7a35bd43562e0718e82ba22e93ebc6a9145e4065c338` |
| `longhorn-tauri-config` Cargo inventory | `cargo package --list --allow-dirty` | `86cdac7c42c2af2be9bebc4ebe613be403d6fb99577fe15f2ff06092c31d9c14` |
| `longhorn-config-0.1.0.private.tar.gz` | private source archive | `90aab39b11afc62ff342f700299fff880fed84d785fbbd9d2d2aa7927338728a` |
| `longhorn-tauri-config-0.1.0.private.tar.gz` | private source archive | `9f64ee6432020e5ad2530e61b2683c0bf4d8175483b12971128e23e677e19a8e` |
| `@inflatable-cookie/longhorn-config` 0.1.0 | packed npm artifact | `3be4f0ca3153dd8bcaa408d7393178ee2bfebfa026722c4e5ac29b13e8b79367` |

The Rust archives are private source snapshots after successful Cargo package
inventory checks. Registry-normalized Cargo archives remain a release gate,
not a Card 103 claim.

## Validation

- full `longhorn-config` and `longhorn-tauri-config` tests with all features
- cross-platform profile, stable identity, bootstrap, transition, and adapter
  fixtures
- generated config binding check
- TypeScript compatibility, Svelte, and package checks
- Cargo package inventories for both affected Rust crates
- focused Northstar Card 103 validation

## Next

Card 104 restores Loophole's clean baseline health and admits the exact private
Longhorn and Poodle dependency graph. The pre-existing `libsqlite3-sys` links
conflict remains that card's first gate.

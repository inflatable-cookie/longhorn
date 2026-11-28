# System Inventory

Use the [capability map](shared-desktop-system-suite.md) to find the contract
for a system. [System architecture](system-architecture.md) shows how the
systems compose, [package topology](package-topology.md) owns dependency
directions, and the [repository authority map](repo-authority-map.md) owns the
consumer roster and product boundaries.

## External Authorities

| Surface | Role | Boundary |
| --- | --- | --- |
| Platform filesystem and app roots | Config, data, cache, log and temporary locations | [Storage](../contracts/004-configuration-storage-backup-and-recovery.md) |
| Secure credential store | Built-in and consumer-scoped slots outside ordinary config and backups | [Credential slots](../contracts/021-consumer-scoped-credential-slots.md) |
| Tauri and GPUI | Native observation, windows and event execution | [Host adapters](../contracts/020-host-adapter-boundary.md) |
| Local or remote service | Optional product authority, acquired and authenticated by the app | [Backend topology](../contracts/007-optional-backend-topology.md) |
| Poodle | Visual primitives, tokens and interaction | [Adapter lifecycle](../contracts/013-svelte-and-poodle-adapter-lifecycle.md) |
| npm and Git tags | TypeScript packages and Rust source respectively | [Release](../contracts/release.md) |

## Evidence

Pure Rust suites exercise decisions, invariants and failure paths. Rust/TS
fixtures check serialization and schema migration. Direct, serialized and
mock host suites exercise typed commands, events and adapters. Svelte tests
cover subscription lifetime and public component projections.

Packaged desktop proofs cover native display, window, transfer and content
paths against real host behavior. Consumer conformance proofs record exact
package, capability and authority edges. See the
[validation input map](../validation-input-map.md) for the checks scoped to
a change and the limits that travel with their results. The
[original delivery record](https://github.com/inflatable-cookie/longhorn/blob/65a06d17b3a28f2c9f5342e225e75686795c85fa/docs/knowledge/architecture/system-inventory.md)
preserves the historical proof sequence.

## Limits

- Server-synchronized configuration, remote conflicts and offline mutation
  policy require a product authority and domain-specific evidence.
- Production service discovery, authentication and endpoint security are
  consumer choices.
- Macro triggers, native accelerators and synchronized keymaps are outside
  the current [command contract](../contracts/006-command-action-and-input.md).
- Native OS notification delivery needs a separate adapter. The ledger owns
  retained state and Poodle owns toast timing.
- Cross-document panel transactions and copy-transfer authority are outside
  the [transfer contract](../contracts/011-cross-window-transfer.md).
- Platform-specific evidence lives in the [host boundary](../contracts/020-host-adapter-boundary.md)
  and [native-content contract](../contracts/017-native-content-island-coordination.md).
  Strong display correlation and packaged transfer outside macOS, and live
  native scale transitions for child views and backing surfaces, remain
  unproved.

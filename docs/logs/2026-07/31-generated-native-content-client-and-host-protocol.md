# Generated Native-content Client And Host Protocol

Date: 2026-07-31
Card: 088
Roadmap: g01.018

## Result

Added the checked renderer seam for production native-content coordination.
Rust owns exact-version requests, results, snapshots, changes, observations,
content-size negotiation, apply receipts, host destruction, and stable typed
rejections. The bindings generator now emits TypeScript plus one golden
protocol fixture.

Added framework-neutral `@inflatable-cookie/longhorn-native-content`. Its root composes injected
direct or serialized transports. `/tauri` maps four narrow commands and one
changed event without importing mechanism or product authority.

## Lifetime Decisions

- Listeners register before connect and snapshot reconciliation.
- Each renderer connection receives a new client epoch.
- Client epoch and native attach generation remain distinct.
- Desired and observed revision cursors independently reject late results.
- Pending request correlation is bounded; remount and teardown are exact.
- Late asynchronous listener registration is disposed once after teardown.
- Session replacement is an explicit failure, not a silent fallback.

## Boundary Result

The protocol contains opaque identities, geometry, scale, lifecycle,
observations, plans, proposals, decisions, and receipts only. Product content,
URLs, browser policy, plugins, GPU/render data, raw handles, Tauri labels,
Svelte, and Poodle remain outside the package.

Capability templates admit read-only or coordinating protocol access. They do
not grant browser, plugin, renderer, or product-content authority.

## Evidence

- 17 pure-kernel and 4 protocol Rust tests pass with `bindings` enabled.
- 14 TypeScript tests pass across fixture compatibility, direct/serialized
  conformance, listener-first races, remount, stale results, bounded
  correlation, teardown, Tauri mapping, package edges, and capabilities.
- TypeScript compilation, Clippy, binding drift, and package dry-run pass.
- The package contains 18 source, README, and capability-template files with
  no native mechanism or UI artifact.

## Roadmap Result

Card 088 is complete. Card 089 is ready to implement the Tauri child-view
adapter and packaged macOS proof. Donor writes remain blocked through Card
093.

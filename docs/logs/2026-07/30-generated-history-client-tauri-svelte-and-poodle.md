# Generated History Client, Tauri, Svelte, And Poodle

Date: 2026-07-30
Card: 066
Roadmap: g01.011

## Result

Added the payload-free generated history protocol, checked framework-neutral
clients, caller-aware Tauri assembly, per-instance Svelte state, and a
controlled public-Poodle linear panel.

Rust remains structural authority. Consumers retain payload meaning, product
authorization, atomic apply, snapshots, storage, journal, recovery, labels,
and presentation policy. No donor repository changed.

## Protocol

`longhorn-history` now projects exact version-1 renderer types for:

- authority epoch, history id, revision, mode, depths, labels, and baseline
- bounded past/current/future metadata pages
- undo, redo, and entry-id checkout commands
- committed receipts and stable rejected outcomes
- non-durable history-changed hints

The golden fixture is `fixtures/history/protocol-v1.json`. Bindings generate
through `longhorn-bindings history`. Exact TypeScript compatibility rejects
future versions, unknown fields or variants, unsafe integers, zero epochs,
and any recursively nested `payload` field.

Product payloads never enter the generated protocol.

## Client Lifecycle

`@inflatable-cookie/longhorn-history` provides injected direct and serialized ports plus one
checked controller. The root imports no Tauri, Svelte, Poodle, browser, or
product dependency.

The controller:

- registers its listener before the initial snapshot
- converges snapshot and page revisions
- refreshes across revision gaps and authority replacement
- sends the current epoch and revision on navigation
- installs the returned authoritative snapshot after stale rejection
- rejects late results and disposes late listener registration after teardown
- keeps filtering and pagination presentation-local

The listener/snapshot interleaving fixture publishes a commit during listener
registration and still converges to the committed revision.

## Tauri Assembly

`longhorn-tauri-history` exposes exact snapshot, page, and navigation commands
over one injected `HistoryHostAuthority`. Every call receives the renderer
window label. Tauri capability grants reachability only; the consumer
authority still decides product authorization and applies navigation.

Only committed navigation projects `longhorn://history/changed`. The event is
a refresh hint. Event publication failure cannot turn an already committed
navigation into an apparent command failure.

Read-only and mutable permission/capability examples keep query, mutation, and
event lifetime explicit.

## Svelte And Poodle

`@inflatable-cookie/longhorn-history/svelte` wraps one controller per rune-backed session.
Mounted multi-instance and SSR fixtures prove state and teardown isolation.

`@inflatable-cookie/longhorn-history/poodle` composes its panel only from public controlled
Poodle primitives. It covers undo, redo, checkout, filter, pagination,
loading, error, and rejection presentation without owning durable state or
copying a visual primitive.

## Boundary Audit

- `longhorn-history` keeps Tauri, Svelte, Poodle, config, bridge, filesystem,
  and async-runtime edges absent
- `ts-rs` is binding-feature-only
- the framework-neutral package root resolves no optional adapter
- Tauri authority remains injected and caller-aware
- events contain revision metadata only
- Poodle is a peer and public visual authority
- no product payload, mutation bus, storage path, journal file, or recovery
  choice enters the client slice

## Validation

- `effigy check:history-bindings`
- `effigy test:history-core`
- `effigy test:tauri-history`
- `effigy check:history-ts`
- `effigy test:history-ts`
- `effigy check:history-svelte`
- `effigy test:history-svelte`
- `effigy check:history-package`
- Rust 1.85 history and Tauri-history tests
- focused clippy and rustdoc
- focused and full Northstar/Effigy QA
- dependency, payload, package, and diff audits

## Next

Card 067 is ready. Prove isolated rich Loophole-shaped and minimal
non-editor compositions from produced artifacts, publish the linear
composition guide, and stop at the pre-fork checkpoint.

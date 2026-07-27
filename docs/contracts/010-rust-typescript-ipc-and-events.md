# 010 Rust, TypeScript, IPC, And Events

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`

## Boundary

Rust owns serialized Longhorn domain contracts. Generated TypeScript types,
thin clients, and testable Tauri handlers expose that authority. IPC is an
adapter seam, not a second product command bus.

## Type Authority

- Rust `serde` types are authoritative for Longhorn payloads.
- `ts-rs` generates TypeScript into the owning domain package.
- Generated files are checked in for package consumers.
- CI regenerates and fails on drift.
- Consumer-owned product payloads remain consumer-owned and may use their own
  checked generation seam.
- Longhorn clients do not maintain handwritten duplicate DTOs.

## Names And Envelopes

- Tauri commands use `longhorn_<domain>_<verb>`.
- Events use `longhorn://<domain>/<kind>`.
- Requests carry a request id and may carry an expected revision, client id,
  or session id.
- Mutations return the request id, authoritative revision, and result or
  current snapshot.
- Errors have stable code, message, retryability, and optional structured
  details. Bare strings are not protocol errors.
- Snapshots and events carry an epoch plus monotonic revision.

## Subscription Handshake

Clients:

1. attach the event listener
2. query the current snapshot
3. accept the newest epoch and revision
4. ignore duplicate or older messages
5. resync on a revision gap or epoch change

Teardown is idempotent and safe when an asynchronous listener registration
completes after disposal. Events are projections and invalidation hints, not
durable delivery.

## Handler And Client Shape

- One handler assembly function is used by the real Tauri app and mock-runtime
  tests.
- Domain packages own typed command, query, and subscription clients.
- Raw `invoke` and `listen` calls for Longhorn domains remain inside the
  bridge package.
- Direct and serialized adapters pass the same domain conformance suite.
- Every command validates current domain authority and capability. Tauri
  capability files are necessary host policy, not sole authorization.

## Compatibility

- Additive optional fields are permitted within a compatible protocol line.
- Removing or changing field meaning requires a breaking package version.
- Unknown future enum variants produce an explicit incompatibility result.
- Bridge startup exposes protocol version and capability negotiation.

## Acceptance

- generated TypeScript has a zero-diff regeneration check
- a Tauri mock runtime exercises the same handler assembly as the app
- listener-before-snapshot tests cannot miss an intervening mutation
- duplicate, stale, gap, and new-epoch fixtures behave deterministically
- late listener registration can be disposed without leaking
- typed error fixtures round-trip between Rust and TypeScript
- no Longhorn Svelte component calls raw Tauri IPC


# 010 Rust, TypeScript, IPC, And Events

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-31
Evidence: `003-foundation-boundary-characterization.md` (Git history);
`013-typed-bridge-and-backend-topology-boundary.md` (Git history);
`014-command-input-and-palette-boundary.md` (Git history)

## Boundary

Rust owns serialized Longhorn domain contracts. Generated TypeScript types,
thin clients, and testable Tauri handlers expose that authority. IPC is an
adapter seam, not a second product command bus.

The shared bridge owns structural session, topology, correlation, error,
retry, and ordering metadata. Domain packages own operation names, payloads,
validation, snapshots, revisions, event meaning, and write policy.

Command ids remain discovery and input identities under contract 006. A
consumer maps an admitted command to one typed domain operation. Neither the
bridge nor a shared Tauri adapter accepts a generic `{ commandId, args }`
execution payload.

## Type Authority

- Rust `serde` types are authoritative for Longhorn payloads.
- `ts-rs` generates TypeScript into the owning domain package. The bridge and
  command host error types are generated from their adapter Serde definitions.
- Generated files are checked in for package consumers.
- CI regenerates and fails on drift.
- Consumer-owned product payloads remain consumer-owned and may use their own
  checked generation seam.
- Longhorn clients do not maintain handwritten duplicate DTOs.
- Adapters do not rename serialized keys or enum values after receipt.

## Names And Envelopes

- Tauri commands use `longhorn_<domain>_<verb>`.
- Events use `longhorn://<domain>/<kind>`.
- Requests are typed queries, commands, or cancellation requests.
- Requests carry a request id and may carry an expected revision, client id,
  session id, or distinct idempotency key.
- A request id is correlation. It is not replay permission.
- Mutations return the request id, authoritative revision, and result or
  current snapshot.
- Errors have stable code, message, retry class, failure phase, and optional
  structured details. Bare strings are adapter failures, not protocol errors.
  Every `longhorn-tauri-*` adapter exposes operational failures as typed
  `{code, message, retryable}` wire errors using its own camelCase code enum.
  A poisoned mutex or failed blocking-task join is permanently unavailable
  (`retryable: false`); adapters never recover a poisoned lock with
  `into_inner` or silently skip teardown on poison.
- Snapshots and authoritative events carry an authority epoch plus monotonic
  revision or sequence.
- Core-backed revisions and bridge and command counters carried as JSON
  numbers admit integers through `2^53 - 1`, with a positive minimum for
  authority epochs. Rust readers reject out-of-range values. Advancing these
  counters returns a typed error when the next value would exceed the bound.
- Progress and terminal events echo their initiating request id.
- Cancellation returns a receipt and does not imply immediate termination.

## Subscription Handshake

Clients:

1. attach the event listener
2. query the current snapshot
3. accept the newest epoch and revision
4. ignore duplicate or older messages
5. resync on a revision gap or epoch change

Teardown is idempotent and safe when an asynchronous listener registration
completes after disposal. Events are projections and invalidation hints, not
durable delivery. Query-only domains do not require an event transport.

A malformed event ends its subscription and is reported as an event failure.
An error thrown by a consumer callback is not a transport failure and does not
end the subscription. A job's terminal event that arrives before its listener
is registered is not replayed; see Q-007.

## Session Lifecycle

- A session ends when its window does. `longhorn_tauri_bridge::bridge_plugin`
  installs `TauriBridgeState` and wires `teardown_window` onto Tauri's
  window-destroyed hook by default; hosts that install `TauriBridgeState::new`
  manually must wire `teardown_window` from `.on_window_event`. A destroyed
  window's session must stop validating, not linger.
- Session ids are per-session secrets. The authority provider must mint an
  unguessable id (128-bit random or better): session use requires only the id
  and the window label, and labels are predictable. Fixture-style ids
  (`session:fixture-*`) are for fixtures, never shipped providers.
- Tauri's transport deserializes before Longhorn sees bytes, so pre-parse
  size caps are the transport's ceiling, by recorded reliance. Longhorn's
  discipline is that every bound lands before work: collection and payload
  limits are validated after parse and before any effect, never after
  mutation.

Event delivery is scoped to the window that owns the publishing session
(`emit_to` the session's caller, never app-wide). A full payload crosses only
to a window whose session carries read authority for the domain; the client
drops foreign-session cursors, so targeting changes no consumer behavior and
removes the one channel where read authority was advisory. Cross-window
invalidation is not a current property — if a product needs it, it is a
designed channel (hint events plus authorized refetch), not an accident of
broadcast.

## Handler And Client Shape

- One handler assembly function is used by the real Tauri app and mock-runtime
  tests.
- Every `longhorn-tauri-*` command adapter follows one policy:
  - **One authority trait per adapter** (`<Domain>HostAuthority`, or
    `BridgeAuthorityProvider` / `PanelTransferAdapter` / `SurfaceTransferAdapter`
    where domain semantics name a provider or split adapter). No second
    `*Service` trait mirroring the authority trait.
  - **Off-main-thread execution:** every `#[tauri::command]` that invokes
    consumer authority, configuration I/O, or blocking work is `async` and
    runs its blocking work on `tauri::async_runtime::spawn_blocking`. Adapter
    mutexes are never held across consumer I/O or window provisioning.
  - **Non-retryable poison:** poisoned adapter state (and `spawn_blocking`
    `JoinError`) maps to a typed `*StateUnavailable` error with
    `retryable: false`.
  - **Window-scoped hints:** command-triggered invalidation hints emit on the
    invoking `WebviewWindow` (`window.emit`), never app-wide.
- Domain packages own typed command, query, and subscription clients.
- Command catalogue and keymap adapters may expose typed metadata, preview,
  and mutation queries. They expose no generic product command executor.
- Raw `invoke` and `listen` calls for Longhorn domains remain inside the
  bridge package.
- Direct and serialized adapters pass the same domain conformance suite.
- Every command validates current domain authority and capability. Tauri
  capability files are necessary host policy, not sole authorization.
- Host connection, authentication, capability advertisement, and domain
  authority are checked separately.
- Domain clients expose indeterminate outcomes for uncertain non-idempotent
  writes.

## Compatibility

- Additive optional fields are permitted within a compatible protocol line.
- Removing or changing field meaning requires a breaking package version.
- Unknown future enum variants produce an explicit incompatibility result.
- Bridge startup exposes exact protocol version, session/host identity,
  transport features, capability advertisement, and authority descriptors.
- v1 uses exact-version negotiation. Version ranges wait for a real
  rolling-upgrade requirement.

## Boundary Validation Target

Decided 2026-08-10, after measuring what the boundary does today rather than
what it was assumed to do.

**The TypeScript boundary validates to the same strictness as the Rust
authority, and derives that strictness from it.** Concretely:

- **Unknown fields are rejected.** Rust declares `deny_unknown_fields` on 332
  types. Nine of thirteen TypeScript packages accepted them, so a payload
  TypeScript waved through was one Rust would refuse. That asymmetry was
  accidental — it is the shape of who wrote which validator, not a decision
  anyone took.
- **Missing fields are rejected.** `ts-rs` emits `Option<T>` as
  always-present-and-nullable, so there is no absent-versus-null ambiguity to
  navigate: zero optional fields across the generated protocol, 88 explicit
  `| null`. A derived validator can be exactly as strict as a hand-written one
  without guessing.
- **Every bound comes from a named constant, never a literal.** A `MAXIMUM_*`
  that bounds a wire-visible collection is enforced from a generated constant,
  and `check:bindings` fails when Rust changes and TypeScript does not.
- **Generated field maps preserve their known keys in TypeScript.** Oxlint
  rejects inline allowed-key arrays passed to validation helpers.
- **A rule Rust declares is generated, not copied.** Including rules that are
  not types: the connection state/reason matrix lived in a `matches!` arm, was
  hand-copied into TypeScript, and agreed by maintenance rather than by
  construction until it was declared and emitted.

### What this costs, stated plainly

Nine packages become stricter than they were. A consumer sending a field the
protocol does not declare starts failing where it was previously ignored.
Every consumer is on a `file:` install, so this surfaces during a coordinated
change rather than in the field — which is the argument for doing it now
rather than after publication.

### What it is not

Not forward compatibility. Ignoring unknown fields would let a newer host add
one without breaking an older client, and that is a real posture with real
advantages. It is rejected here because it diverges from the authority, and a
boundary that is deliberately less strict than the thing it defends needs its
own written rule and its own drift gate. Longhorn has one authority; the
boundary derives from it.

If rolling upgrades ever require it, that is a version-negotiation change and
belongs beside the exact-version rule above — not a per-package validator
loosening.

## v1 Adapter Proof

- direct/in-process execution
- Tauri invoke/event execution
- deterministic serialized loopback execution

The loopback proves serialization and semantic parity. It does not claim a
production network protocol. Service transports implement the same injected
port after their production security and platform contract is selected.

The operation domain proves the same checked state through direct,
JSON-serialized, Tauri, and bridge-domain composition. Bridge request and job
ids remain correlation; operation id and authority cursor remain domain
authority.

## Acceptance

- generated TypeScript has a zero-diff regeneration check
- a Tauri mock runtime exercises the same handler assembly as the app
- listener-before-snapshot tests cannot miss an intervening mutation
- duplicate, stale, gap, and new-epoch fixtures behave deterministically
- late listener registration can be disposed without leaking
- progress, cancellation, terminal, and indeterminate-write fixtures remain
  request-correlated
- typed error fixtures round-trip between Rust and TypeScript
- no Longhorn Svelte component calls raw Tauri IPC

## Implemented Evidence

Cards 049-055 generate and zero-diff check the exact-v1 bridge protocol,
execute one semantic contract through direct, Tauri, and serialized-loopback
adapters, and install five isolated consumers from packed TypeScript
artifacts. Import inventories distinguish invoke-only, stream, event, and
supervision edges. Capabilities remain exact outer admission; per-domain
authority remains separately checked.

Private Rust crates pass package inventory checks and offline unpacked
consumer builds. Registry-normalized Cargo artifacts remain a public
release-lane gate.

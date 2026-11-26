# 010 Rust, TypeScript, IPC, And Events

Status: active compiled boundary
Owner: Tom
Updated: 2026-11-24
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
- `ts-rs` generates TypeScript into the owning domain package. All Tauri
  operational host errors with a `{code, message, retryable}` envelope are
  generated from their adapter Serde definitions into the owning domain.
- Generated files are checked in for package consumers.
- CI regenerates and fails on drift.
- Consumer-owned product payloads remain consumer-owned and may use their own
  checked generation seam.
- Longhorn clients do not maintain handwritten duplicate DTOs.
- Adapters do not rename serialized keys or enum values after receipt.
- Config restore failure phases and terminal states, adapter receipt outcomes,
  and recovery receipt results are closed camelCase wire enums. Renderer
  validation uses the value sets generated from Rust.

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
  The bridge, command, config, history, history-tree, licence, notifications,
  operation, settings, transfer, and update Tauri adapters expose operational
  failures as `{code, message, retryable}` using their own camelCase code enum.
  `TauriTransport` decodes a rejection as `TauriHostError` when its object has
  exactly `code` and `message` strings and a `retryable` boolean.
  The transport preserves the code as an opaque string so consumers can use
  the domain's generated code types without losing diagnostic text at the
  shared boundary. Other native rejections keep their original value.
  A poisoned mutex or failed blocking-task join is permanently unavailable
  (`retryable: false`); adapters never recover a poisoned lock with
  `into_inner` or silently skip teardown on poison.
- Snapshots and authoritative events carry an authority epoch plus monotonic
  revision or sequence.
- Core-backed revisions, bridge and command counters, and settings registry
  generations and scope revisions carried as JSON numbers admit integers
  through `2^53 - 1`, with a positive minimum for authority epochs. Rust
  readers reject out-of-range values. Advancing these counters returns a typed
  error when the next value would exceed the bound.
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
completes after disposal. A failure reporter cannot prevent teardown.
Teardown attempts every registered unlisten. Registration keeps its original
rejection if cleanup also fails. Events are best-effort projections and
invalidation hints. Query-only domains do not require an event transport.

The commands, history, history-tree, licence, update, notifications and
operation controllers reset to idle when stopped. `stop()` attempts every
owned unlisten and rejects with the first failure in registration order.
Licence and update `refresh()` only read during a started lifetime. An older
refresh cannot replace a newer refresh or a settled command's state.

A malformed event ends its subscription and is reported as an event failure.
The checked history, history-tree, licence and update listeners report it to
the optional `onFailure` callback with phase `event`. Their controllers keep
the failure visible until stopped and started again. Other subscriptions in
the same controller remain active.
A job's listeners check its request and job identifiers before they decode the
payload. An event whose identifier reads as another request or job is dropped
without a failure, even when its other identifier is unreadable, and nothing
after the job's terminal is decoded. A throwing consumer callback leaves the
subscription active and any accepted snapshot current. After registration and
outside a snapshot query, the exception propagates to the event emitter.
During registration, the subscription contains it so the transport can return
its unlisten handle. During a snapshot query, it contains the exception so
the query can finish. A job's
terminal event that arrives before its listener is registered is not replayed.
See Q-007.

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

The Tauri bridge dispatches domain, progress and terminal events as
`CustomEvent` on the owning renderer's `window`. The host serializes the event
name and encodes the payload as a JSON string literal before building the
script. The script parses that string into `detail`, which the DOM listener
reads. The subscription capability grants only invoke permissions. The
Tauri event bus filters listener callbacks and
can still send the payload to a nonmatching webview, so it cannot carry
these full payloads across the authority boundary. The client also checks
session cursors and job correlation. A product that needs cross-window
invalidation can use a distinct hint and an authorized refetch.

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
- **Transfer client-change hints:** the host targets the invoking webview
  window with `emit_to`. The client requests a `current-webview` subscription
  because Tauri's default `Any` subscription also matches targeted events.
- **Broadcast hints:** history, history-tree, operation, licence,
  notifications, settings, update, and command keymap emit command hints
  app-wide via `window.emit`. The command catalogue and trusted domain
  publishers use `app.emit`. A window
  receiver on a Rust `emit` call does not narrow delivery.
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
- The operation bridge reports rejected, indeterminate, and miscorrelated
  replies through `BridgeOperationError.outcome`. Correlated failed and
  cancelled job terminals also throw this error. Bridge failures retain
  their code, message, retry class, phase, and details.

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

  The rule covers tagged enums too, with one Rust-side limit: serde ignores
  unknown fields on an internally tagged *unit* variant even when the container
  declares `deny_unknown_fields`. A unit variant on the wire is therefore an
  empty struct variant (`Undo {}`), which serde does check.
- **Missing fields are rejected.** A required field must be an own, enumerable
  property so it survives JSON serialization. `ts-rs` emits `Option<T>` as
  always-present-and-nullable, so there is no absent-versus-null ambiguity to
  navigate: zero optional fields across the generated protocol, 88 explicit
  `| null`. A derived validator can be exactly as strict as a hand-written one
  without guessing.
- **Every bound comes from a named constant, never a literal.** A `MAXIMUM_*`
  that bounds a wire-visible collection is enforced from a generated constant,
  and `check:bindings` fails when Rust changes and TypeScript does not.
- **Generated field maps preserve their known keys in TypeScript.** Oxlint
  rejects inline allowed-key arrays passed to validation helpers.
- **Check nested wire data before returning a typed value.** Validate array
  members and the contents of each admitted variant. Surface clients use
  non-negative safe-integer epochs and revisions and require the snapshot
  revision to match the document revision before applying it.
- **A rule Rust declares is generated, not copied.** Including rules that are
  not types: the connection state/reason matrix and the state-to-state
  transition matrix each lived in a `matches!` arm, were hand-copied into
  TypeScript, and agreed by maintenance rather than by construction until they
  were declared (`BridgeConnectionStatus::ADMITTED_REASONS`,
  `BridgeConnectionState::ADMITTED_TRANSITIONS`) and emitted.
  Bridge request contexts and session payloads share the validator for the
  generated `DomainId` grammar and byte bound. Layout ratio bounds and the
  command availability, config operation, notification receipt and transfer
  cancellation value sets also come from the Rust bindings. Service supervision uses the
  generated failure codes and reports unknown ownership as `invalid_ownership`.

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
- typed error fixtures round-trip through Rust Serde and decode through the
  TypeScript Tauri transport, including a rejected invoke shown by a panel
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

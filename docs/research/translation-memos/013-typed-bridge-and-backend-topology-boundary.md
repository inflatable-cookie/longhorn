# 013 Typed Bridge And Backend Topology Boundary

Status: complete and promoted
Owner: Tom
Updated: 2026-07-30
Extends: `001-tauri-application-extraction-audit.md`,
`002-shared-desktop-systems-follow-up.md`, and
`003-foundation-boundary-characterization.md`

## Prompt

Resolve the open bridge and optional-backend decisions before compiling
g01.009. The result must serve a Bovine-sized local app without forcing events
or a service while preserving Nucleus and Loophole host topologies.

## Evidence

### Longhorn

- `@longhorn/core` already owns a domain-free invoke/event transport and a
  listener-first checked snapshot connection.
- `@longhorn/tauri` is a thin raw Tauri transport. Domain clients own command
  names, validation, snapshots, and event meaning.
- Existing layout, Surface, transfer, configuration, and settings protocols
  already use Rust-owned generation, request ids, checked revisions, epochs,
  receipts, and golden fixtures.
- The bridge milestone should consolidate those structural rules. It must not
  replace the typed domain protocols with an untyped product bus.

### Nucleus

- The engine is authority; embedded desktop, local sidecar, remote
  authoritative host, worker/proxy, and managed hosting are deployment forms.
- Host connection, capability advertisement, authentication, and per-project
  domain authority are separate facts.
- The first desktop control wire uses exact `nucleus.control` v1 negotiation,
  request/response/server-event envelopes, generated TypeScript DTOs, command
  ids, query ids, and explicit idempotency keys.
- Current production evidence is embedded/Tauri-local. Remote transports,
  discovery, and authentication are planned seams, not implemented proof.
- A remote worker may own execution without becoming project write authority.

### Loophole

- Pulse remains sole project/session authority across embedded, local-brokered,
  and remote profiles. Topology changes lifecycle ownership, not command,
  snapshot, or event meaning.
- Aura and Spark are clients or host adapters. They cannot silently replace a
  failed authority with renderer state.
- The current Aura bridge is a useful negative specimen: hand-written command
  strings and renderer-side snake/camel conversion leave wire authority split.
- Pulse exposes bootstrap/readiness states and authoritative snapshots. Echo's
  current generic envelope is only a small header, not a complete shared
  protocol.
- Remote profiles remain additive. The current Aura local posture is embedded,
  so Longhorn must not claim a proven production network transport from it.

### Jetstream

- One renderer bridge contains all raw Tauri calls.
- The editor publishes one coherent state snapshot only when it changes.
- A new renderer attaches the listener before sending `page:ready`; the host
  then forces a current snapshot even if the state is unchanged.
- Rust owns the serialized state shape, but TypeScript duplicates it by hand
  and the stream has no epoch/revision gap detection. The pattern is valuable;
  the wire-authority gap is not.

### Soundcheck

- The renderer has one broad Tauri API facade with many handwritten return
  types and command strings.
- Long-running assistant and inspection work uses request-correlated progress,
  explicit cancellation, terminal events, and cleanup on completion.
- Plugin scanning also exposes snapshot/status/cancel semantics.
- Composer is optional and disconnected by default. Local workflows retain
  authority when it is unavailable.
- A separately exposed loopback API and helper processes prove that an app may
  host or supervise services without making service topology mandatory.

### Bovine Accelerator Desktop

- The app has 11 typed invoke calls, 11 Tauri commands, no event listeners, no
  host event emissions, and no service topology.
- It proves the minimum composition: checked request/response over a local
  Tauri authority with no subscription or backend package.

## Donor Translation

| Concern | Retain | Change | Reject |
| --- | --- | --- | --- |
| wire authority | Rust-owned payloads, generated checked TypeScript | centralize structural envelopes and compatibility | handwritten duplicate DTOs and renderer wire conversion |
| topology | stable domain semantics across host forms | model connection, host, and authority independently | equating connection with write authority |
| snapshots | listener before current snapshot | add epoch, revision, gap detection, and resync | unordered partial UI truth |
| jobs | request correlation, progress, cancel, terminal cleanup | make job capability optional | mandatory event support for simple apps |
| retries | explicit idempotency evidence | classify retry per operation and outcome | retrying uncertain writes from request id alone |
| services | injected supervision and explicit readiness | keep provisioning and update policy consumer-owned | bundled downloader/updater or silent local fallback |
| offline | local projections remain usable where valid | fail or defer mutations explicitly | generic mutation queue or cache promoted to authority |

## Boundary Decisions

### Protocol layers

The shared seam has three layers:

1. `@longhorn/core` and `@longhorn/tauri` keep domain-free transport and
   listener lifetime.
2. The bridge protocol owns session negotiation, topology, connection state,
   authority descriptors, correlation, error shape, retry classification, and
   ordered event metadata.
3. Domain packages own operation names, payload types, validation, snapshots,
   revisions, event meaning, and write policy.

Longhorn never accepts an arbitrary product JSON payload as shared domain
authority merely because it can serialize it.

### Version and capability negotiation

- v1 uses one exact bridge protocol version. Range negotiation waits for a real
  rolling-upgrade requirement.
- A successful connection returns host identity, host form, bridge version,
  session id, supported transport features, and domain capabilities.
- Capability advertisement does not grant authority. Each domain descriptor
  separately names availability, read/write authority, authority epoch, and
  optional authoritative revision.
- Authentication, endpoint admission, command authorization, and domain
  authority remain separate checks.

### Requests, replies, jobs, and errors

- Requests are queries, commands, or cancellation requests with one request id.
- A command may carry a distinct idempotency key. A request id is correlation,
  not replay permission.
- Replies echo the request id and return a typed result or stable coded error.
- Errors expose code, message, retry class, failure phase, and optional typed
  details. Bare strings are adapter failures, not protocol errors.
- Progress and terminal events are optional request-correlated capabilities.
  Cancellation is a request and receipt, not an assumption that work stopped.

### Snapshots and events

- Subscription-capable domains attach before loading the current snapshot.
- Snapshots and authoritative update events carry authority epoch and monotonic
  revision or sequence.
- Duplicate and older events are ignored. A gap or epoch change requires a
  fresh snapshot. Events from a superseded session or authority epoch cannot
  overwrite newer state.
- Events are live projections and invalidation hints, not durable delivery.
- Query-only domains do not need an event transport.

### Retry and offline policy

- Queries may retry only under declared transport policy.
- Commands may retry only when they carry a durable idempotency key and the
  authority advertises replay/deduplication support.
- An uncertain non-idempotent write becomes an explicit indeterminate outcome.
  It is never retried silently.
- v1 has no generic offline mutation queue. A domain may later define one only
  with durable ordering, conflict, expiry, cancellation, and authority rules.
- Offline caches are projections. They cannot accept authoritative writes.

### Lifecycle and supervision

- The connection model distinguishes idle, connecting, negotiating, ready,
  degraded, reconnecting, offline, incompatible, unauthorized, failed, and
  closed.
- Embedded, Tauri-local, local-service, and remote forms share semantic
  contracts. They differ in transport and lifecycle ownership.
- Consumers own topology selection, executable acquisition, installation,
  update policy, endpoint selection, and remote server lifecycle.
- Longhorn may provide an injected supervisor port, readiness projection,
  reconnect policy, and shutdown receipts. It does not download or silently
  replace services.
- Credentials are opaque adapter inputs backed by contract 004's secure-store
  seam. No credential enters ordinary config snapshots or bridge diagnostics.

### v1 transport proof

- Required executable adapters are direct/in-process and Tauri invoke/event.
- A deterministic serialized loopback adapter proves codec and semantic
  conformance without claiming a production network protocol.
- Local-service and remote examples compose the same supervisor, connection,
  and authority model through an injected transport port.
- HTTP, WebSocket, Unix socket, and Windows named-pipe selection stays deferred
  until a production consumer supplies cross-platform and security evidence.

## Package Consequences

- `longhorn-bridge`: pure protocol identities, negotiation, topology,
  connection, authority, request/reply/event metadata, retry classes, and
  conformance fixtures
- `longhorn-tauri-bridge`: narrow host assembly adapting registered domain
  handlers to Tauri
- `@longhorn/bridge`: checked generated bridge protocol, session client,
  authority projection, and direct/serialized conformance helpers
- `@longhorn/core`: unchanged structural transport and listener lifetime
- `@longhorn/tauri`: unchanged domain-free raw invoke/listen transport

Domain packages keep their generated payloads and clients. Service
supervision remains an optional adapter edge; a no-service consumer resolves
none of it.

## Deferred

- production local-service and remote network transport selection
- endpoint discovery, pairing, and authentication provider choice
- rolling bridge-version range negotiation
- durable offline mutation queues
- server-synchronized settings conflicts and transactions
- generic durable job history and notification presentation

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/system-inventory.md`
- `../../architecture/package-topology.md`
- `../../contracts/007-optional-backend-topology.md`
- `../../contracts/010-rust-typescript-ipc-and-events.md`
- `../../contracts/012-distribution-and-compatibility.md`
- `../../roadmaps/g01/009-typed-bridge-and-optional-backend-topology.md`


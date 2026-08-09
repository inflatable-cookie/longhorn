# g01.009 Typed Bridge And Optional Backend Topology

Status: complete
Owner: Tom
Updated: 2026-07-30
Governing refs: contracts 001, 004, 007, 010, and 012; research memo 013
Generation runway goal: establish the last shared authority/transport seam
before commands and consumer migration

## Outcome

Use one semantic client contract across direct, Tauri-local, injected
local-service, and remote host forms without forcing service or event
dependencies on simple apps.

## Goals

- [x] Make bridge negotiation, topology, connection, capability, and authority
  a Rust-owned checked protocol.
- [x] Keep operation names, payloads, snapshots, revisions, and write policy in
  their owning domain packages.
- [x] Prove request/reply, ordered snapshots/events, correlated jobs,
  cancellation, errors, retries, and indeterminate writes.
- [x] Run the same semantic fixtures through direct, Tauri, and deterministic
  serialized-loopback adapters.
- [x] Compose injected local/remote service supervision without selecting or
  claiming a production network protocol.
- [x] Prove Split-shell, Jetstream, Soundcheck, Nucleus, and Loophole-shaped
  compositions from produced artifacts.

## Execution Plan

### Batch 1 — Semantic protocol

- [x] Card 049: implement bridge identity, exact-version negotiation,
  connection state, host form, capabilities, and authority descriptors.
- [x] Card 050: implement typed requests/replies, coded failures, retry and
  idempotency classes, ordered streams, and correlated job lifecycle.

### Batch 2 — Checked clients and host adapters

- [x] Card 051: generate checked TypeScript and prove direct plus serialized
  loopback semantic conformance.
- [x] Card 052: assemble narrow Tauri host/client adapters with mock-runtime
  proof and query-only dependency absence.
- [x] Card 053: implement reconnect, indeterminate-write, authority-epoch, and
  injected supervisor lifecycle.

### Batch 3 — Topology composition and closeout

- [x] Card 054: prove local-only, embedded, local-service, remote-attach, and
  optional-service topology compositions without donor writes.
- [x] Card 055: install produced artifacts into five isolated donor-shaped
  proofs, audit boundaries, publish guidance, and close g01.009.

## Public Behavior

The bridge exposes negotiated host/session identity, transport features,
connection state, capability advertisement, and separate domain authority.
Domain clients then use their own generated operations and payloads through
that session.

Subscription-capable domains attach before loading current state. Authority
epoch and monotonic revision/sequence reject stale, duplicate, and gapped
events. Query-only domains use invoke without importing event support.

Commands retry only with a durable idempotency key and advertised
deduplication. Uncertain non-idempotent writes become indeterminate. Offline
projections never accept authoritative writes.

## Out Of Scope

- production HTTP, WebSocket, Unix-socket, or Windows named-pipe transport
- endpoint discovery, pairing, authentication provider, or credential UI
- service acquisition, installation, or update delivery
- durable offline mutation queues
- server-synchronized settings conflicts or cross-domain transactions
- donor repository migration or writes
- product payloads, commands, or authority in Longhorn

## Acceptance Criteria

- [x] Rust bridge types generate checked TypeScript with zero drift.
- [x] Direct, Tauri, and serialized-loopback adapters pass one semantic suite.
- [x] Query-only Split-shell shape resolves no event or service runtime.
- [x] Jetstream shape cannot miss initial or intervening snapshot state.
- [x] Soundcheck shape correlates progress, cancellation, and terminal cleanup.
- [x] Nucleus shape separates connection, capability, execution, and write
  authority.
- [x] Loophole shape preserves domain semantics across embedded and remote
  host forms.
- [x] Duplicate, stale, gap, new-epoch, reconnect, incompatible-version, and
  indeterminate-write cases are deterministic.
- [x] Produced artifacts preserve narrow optional dependency graphs.
- [x] Full Effigy QA passes at closeout.

## Stop Conditions

- A generic envelope requires Longhorn to own product operation names or
  payload schema.
- A request id must be treated as an idempotency key.
- A cache or disconnected renderer must accept authoritative writes.
- A production service transport must be selected without consumer security
  and cross-platform evidence.
- Service provisioning or updates cannot remain injected consumer policy.
- One donor shape needs a silent fallback authority.

## Lane Runway

Cards 049-055 and g01.009 are complete. Research memo 014 revalidates contract
006 against this bridge and compiles Cards 056-061. Cards 056-061 and g01.010
are complete.

## Next Task

Return to the g01 runway. Reassess the g01.011 history research gate against
the completed bridge and command foundations.

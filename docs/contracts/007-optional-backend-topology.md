# 007 Optional Backend Topology

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-30
Evidence:
`../research/translation-memos/013-typed-bridge-and-backend-topology-boundary.md`

## Boundary

Longhorn defines one transport-independent bridge session and authority model.
Domain packages retain operation and payload authority. Consumers decide
whether the authority is embedded, Tauri-local, separately hosted, or remote.

No foundation, configuration, settings, window, layout, or Surface API
requires a service, subscription, or event transport.

## Supported Host Forms

- direct in-process authority
- Tauri-hosted local authority
- separately supervised local service
- remote service
- local-first authority with remote synchronization

Host form changes transport and lifecycle ownership. It does not change domain
command, query, snapshot, event, or authority meaning.

## Negotiated Session

A successful bridge negotiation exposes:

- exact bridge protocol version
- session id and host-instance id
- host form and transport features
- connection state
- advertised domain capabilities
- per-domain authority descriptors

v1 accepts one exact protocol version. Unsupported versions produce an
actionable incompatible state; range negotiation is deferred.

Capability advertisement and authority grant are separate. Connection,
authentication, endpoint admission, command authorization, and write authority
must not be inferred from one another.

## Connection State

The shared state model distinguishes:

- idle
- connecting
- negotiating
- ready
- degraded
- reconnecting
- offline
- incompatible
- unauthorized
- failed
- closed

Transitions name their cause and whether reconnect is permitted. A client
cannot project `ready` before negotiation and required domain authority checks
complete.

## Authority

- Every domain names availability, read authority, write authority, authority
  epoch, and optional current authoritative revision.
- One domain has at most one current write authority for one authority scope.
- One connected host may expose no authority or only a subset of domains.
- A remote worker may own execution without owning product writes.
- Local UI state stays local unless a contract explicitly promotes it.
- Server-owned product data does not move into app config because the desktop
  has a cache.
- Offline caches and renderer snapshots are projections, not competing
  authorities.
- A failed or missing authority cannot be replaced by renderer simulation or a
  silent local fallback.
- Secrets use the secure-store boundary from contract 004.

## Requests And Retry

- Requests are typed queries, commands, or cancellation requests with one
  correlation request id.
- Commands may also carry a distinct durable idempotency key.
- A request id alone does not permit replay.
- Replies echo the request id and return a typed result or stable coded error.
- Queries retry only under declared adapter policy.
- Commands retry only when the authority advertises replay/deduplication and
  the command carries a durable idempotency key.
- An uncertain non-idempotent write produces an explicit indeterminate outcome
  and is never retried silently.

v1 has no generic offline mutation queue. A future domain queue requires
explicit durable ordering, conflict, expiry, cancellation, and authority
semantics.

## Snapshots, Events, And Jobs

- Query-only domains need no event transport.
- Subscription-capable domains attach before loading the current snapshot.
- Snapshots and authoritative updates carry authority epoch plus monotonic
  revision or sequence.
- Duplicate and older events are ignored.
- A gap or epoch change requires a fresh authoritative snapshot.
- Superseded session or authority events cannot overwrite newer state.
- Events are live projections and invalidation hints, not durable delivery.
- Request-correlated progress, cancellation, and terminal events are optional
  capabilities.
- Cancellation returns a receipt; requesting cancellation does not assert that
  work has stopped.

## Lifecycle

- The consumer owns topology choice, executable acquisition, installation,
  update policy, endpoint selection, credentials, and remote service
  lifecycle.
- Longhorn may supply injected supervisor ports, clients, transports,
  readiness models, reconnect policy, and shutdown receipts.
- Longhorn does not download, update, or silently replace services.
- Shutdown, upgrade, schema incompatibility, and reconnect behavior are
  observable.
- A remote outage cannot block access to unrelated local settings or safe
  window restoration.

## v1 Transport Set

- direct/in-process adapter
- Tauri invoke/event adapter
- deterministic serialized loopback adapter for codec and semantic conformance

Local-service and remote examples use an injected transport port. No production
HTTP, WebSocket, Unix-socket, or Windows named-pipe compatibility claim is made
until a production consumer supplies cross-platform and security evidence.

## Acceptance

- Rust-owned bridge types generate checked TypeScript with a zero-diff check.
- The same semantic fixture passes direct, Tauri, and serialized-loopback
  adapters.
- A Bovine-shaped request/response composition imports no event or service
  runtime.
- A Jetstream-shaped listener-first snapshot cannot miss the initial state.
- A Soundcheck-shaped job correlates progress, cancel, and terminal cleanup.
- A Nucleus-shaped optional host reports capability and authority separately.
- A Loophole-shaped topology switch preserves domain semantics.
- Stale events after reconnect cannot overwrite a newer snapshot.
- Incompatible versions and indeterminate writes produce actionable states.
- An offline projection cannot silently become write authority.

## Implemented Evidence

Cards 049-055 implement and prove this boundary. Five isolated consumers
install produced TypeScript artifacts with exact imports and permissions.
Separate private Rust source inventories compile query-only and full-host
graphs offline. The proof covers direct, Tauri, and serialized-loopback
semantics, exact-v1 rejection, listener-first resync, job correlation,
reconnect invalidation, bounded retry, injected ownership, and authority
separation.

Production transport, endpoint security, discovery, provisioning, updates,
and remote lifecycle remain outside the implemented claim. See
`../architecture/bridge-topology-composition.md`.

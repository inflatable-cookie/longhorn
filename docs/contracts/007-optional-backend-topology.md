# 007 Optional Backend Topology

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27

## Supported Shapes

Longhorn-facing clients may use:

- an in-process Rust authority
- a Tauri-hosted local authority
- a separately launched local service
- a remote service
- a local-first authority with remote synchronization

No foundation, configuration, window, or layout API requires a server.

## Seam

- Domain commands, queries, events, errors, and capabilities are
  transport-independent.
- Transport adapters implement the same semantic contract.
- Connection state is explicit: starting, ready, degraded, offline,
  incompatible, unauthorized, or failed.
- Clients perform capability/version negotiation before enabling features.
- Initial snapshot plus ordered update/subscription behavior is defined per
  domain.
- Retries require idempotency or an explicit non-retryable command class.

## Authority

- Every domain names one current write authority.
- Local UI state stays local unless a contract explicitly promotes it.
- Server-owned product data does not move into app config because the desktop
  has a cache.
- Offline caches are projections, not accidental competing authorities.
- Secrets use the secure-store boundary from contract 004.

## Lifecycle

- The app owns which topology it composes and how a service is provisioned.
- Longhorn may supply supervisors, clients, transports, and readiness models.
- Shutdown, upgrade, schema incompatibility, and reconnect behavior are
  observable.
- A remote outage cannot block access to unrelated local settings or safe
  window restoration.

## Acceptance

- the same fixture runs through direct and serialized adapters
- a no-server Bovine composition imports no service runtime
- a Nucleus-style remote/local service choice reports one authority
- stale events after reconnect cannot overwrite a newer snapshot
- incompatible versions fail with an actionable state

## Open Decisions

- generated contract tooling
- transport set for v1
- offline mutation queue semantics
- service launch and update ownership


# Bridge Topology Composition

Status: promoted  
Owner: Tom  
Updated: 2026-07-30  
Contracts: `../contracts/007-optional-backend-topology.md`,
`../contracts/010-rust-typescript-ipc-and-events.md`, and
`../contracts/012-distribution-and-compatibility.md`

## Selection Rule

Start with domain authority. Select only the host and delivery edges needed to
reach it.

| Need | Rust | TypeScript | Tauri capability |
| --- | --- | --- | --- |
| Direct in-process domain | `longhorn-bridge` | `@inflatable-cookie/longhorn/bridge` | none |
| Tauri-local query | `longhorn-tauri-bridge` | root plus `/tauri` | query |
| Ordered Tauri projection | same host | add `/stream` and `/tauri-events` | query, listen, unlisten |
| Correlated Tauri job | same host | add `/tauri-events` | query/mutate as registered, listen, unlisten |
| Injected service lifecycle | `longhorn-bridge/supervision` | add `/supervision` | none from supervision |

`@inflatable-cookie/longhorn/bridge` depends on `@inflatable-cookie/longhorn/core` and `@inflatable-cookie/longhorn-tauri`.
The Tauri package carries one `@tauri-apps/api` peer. Event support is a
subpath import, not an automatic capability grant.

Rust service supervision is feature-gated. `longhorn-tauri-bridge` is a
separate adapter crate. A query-only Rust consumer can compile with
`longhorn-bridge` alone.

## Five Proven Shapes

| Shape | Host form | Optional imports | Permissions | Service owner | Query retries |
| --- | --- | --- | --- | --- | ---: |
| Split-shell | Tauri-local | none | query | none | 0 |
| Jetstream | direct, Tauri-local | stream, Tauri events | query, listen, unlisten | none | 1 |
| Soundcheck | Tauri-local, local service | supervision, Tauri events | query, mutate, listen, unlisten | external local | 2 |
| Nucleus | direct, optional local service | supervision | query, mutate | external local | 2 |
| Loophole | local-first, remote attach | supervision | query, mutate | external remote | 3 |

These are composition proofs, not product prescriptions. Products keep their
operation names, payload types, domain registration, authority rules, service
choice, endpoint policy, and presentation.

## Session Sequence

1. Connect the selected transport.
2. Send an exact-version hello with requested domain ids.
3. Validate host identity, session identity, connection state, transport
   features, capabilities, and per-domain authority.
4. Construct domain clients from the accepted session.
5. For live projections, attach the listener before loading the snapshot.
6. On a gap or authority-epoch change, reload instead of applying uncertain
   events.
7. On reconnect, invalidate the old session and negotiate again.
8. Close subscriptions, jobs, session state, then any consumer-owned service.

Connection, capability, and authority answer different questions:

- connection: can the renderer currently reach this host?
- capability: does this host expose the operation class?
- authority: may this domain read, write, or execute here now?

No one field substitutes for another. A remote projection may be readable but
not writable. An advertised mutation capability does not override a
domain-level write denial.

## Operation And Delivery Rules

Domain packages own descriptors and codecs. The bridge owns correlation,
session checks, generic failure classes, delivery semantics, ordering, and
lifecycle.

- queries may retry only within the consumer-selected bound
- commands retry only with a durable idempotency key and advertised
  deduplication
- a request id is correlation, not an idempotency key
- uncertain non-idempotent writes return indeterminate
- progress and terminal events match both request and job id
- the first accepted terminal event ends the job
- stale, duplicate, gapped, and wrong-epoch events are deterministic

The direct and Tauri adapters execute the same semantic contract.
Serialized loopback proves codec and serialization behavior. It is not a
production network transport.

## Optional Service Boundary

Supervision accepts a consumer-injected operation and an opaque credential
reference. It owns validation and observable lifecycle:

```text
absent -> starting/attaching -> awaiting readiness -> ready
       -> reconnecting/restarting -> ready
       -> stopped/failed
```

The consumer owns:

- executable or service selection
- process creation and platform packaging
- endpoint discovery and connection transport
- credential lookup and authentication provider
- pairing, trust, and certificate policy
- acquisition, installation, updates, and rollback
- remote host lifecycle

External local and remote services may attach and reconnect. Longhorn does not
stop, restart, or replace externally owned services.

## Tauri Host Assembly

Register domain handlers explicitly with `longhorn-tauri-bridge`. Compose only
the query, command, snapshot, cancellation, and publication ports the product
needs. Tauri capabilities remain the outer admission boundary.

Renderer composition uses:

- `@inflatable-cookie/longhorn-tauri/bridge` for invoke
- `@inflatable-cookie/longhorn-tauri/bridge-events` only for ordered domain or job events
- domain-owned codecs and operation descriptors

Do not pass raw product command names through a generic bridge string bus. Do
not infer renderer authority from a visible control.

## Migration From Handwritten Tauri

Migrate one domain at a time.

1. Inventory current commands, events, payload versions, correlation ids,
   retry behavior, and authority.
2. Name the domain authority and retain it in the product or owning Longhorn
   domain package.
3. Define checked operation descriptors and codecs.
4. Put the existing handler behind the bridge domain registration seam.
5. Run its semantic fixtures through direct and Tauri adapters.
6. For subscriptions, replace load-then-listen with listener-then-snapshot.
7. Replace ad hoc reconnect and retries with explicit session, epoch, and
   delivery policy.
8. Narrow Tauri permissions to the registered operation classes.
9. Remove the old commands and events only after parity evidence passes.

Rollback is domain-local: restore the previous handler/client pair and its
capabilities. Do not keep a silent fallback between old and new authorities.

## Artifact Evidence

`proof:bridge-topology-artifacts` packs `@inflatable-cookie/longhorn/core`,
`@inflatable-cookie/longhorn-tauri`, and `@inflatable-cookie/longhorn/bridge`, then installs five clean consumers.
It rejects workspace aliases, sibling source resolution, undeclared imports,
permission drift, product vocabulary, credential values, and production
network dependencies.

Private Rust crates cannot yet produce registry-normalized `.crate` archives
because their private dependencies are not published. The proof therefore:

- runs `cargo package --list --allow-dirty` for each crate
- archives the exact private source roots
- checks query-only and full-host consumers offline from unpacked archives
- proves supervision and Tauri remain removable graph edges

Registry-normalized Rust packaging remains a release-lane gate.

## Deferred Production Work

The bridge does not claim:

- HTTP, WebSocket, Unix-socket, or named-pipe compatibility
- endpoint discovery or remote provisioning
- authentication, secret storage, or pairing
- service installer or updater behavior
- durable offline mutation queues
- cross-domain transactions

Select those only with consumer security, deployment, failure, and
cross-platform evidence.

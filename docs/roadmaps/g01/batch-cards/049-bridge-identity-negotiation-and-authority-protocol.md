# 049 Bridge Identity, Negotiation, And Authority Protocol

Status: complete
Owner: Tom
Roadmap: g01.009 batch 1
Governing refs: contracts 001, 007, 010, and 012; research memo 013
Depends on: Card 048
Auto-start next card: no
Completed: 2026-07-30

## Objective

Implement the pure Rust bridge identity, exact-version negotiation, connection,
host-form, capability, and per-domain authority protocol without domain
payload, Tauri, service, renderer, or consumer dependencies.

## Scope

- `longhorn-bridge` crate
- bounded bridge, session, host-instance, domain, capability, authority-scope,
  transport-feature, and diagnostic ids
- exact v1 hello request and negotiated receipt
- direct, Tauri-local, local-service, remote, and local-first host forms
- explicit connection states and transition reasons
- host/session identity and transport-feature advertisement
- separate domain capability and authority descriptors
- authority epoch and optional authoritative revision evidence
- stable negotiation and authority failures
- Bovine, Nucleus, and Loophole-shaped negotiation fixtures

## Public Behavior

Negotiation validates one exact bridge protocol version and returns one host
and session identity. It advertises transport features and domain capabilities,
then separately names each domain's read/write authority and epoch.

Connection, authentication posture, capabilities, execution ownership, and
write authority remain distinct facts. One authority scope has at most one
declared writer. An absent domain remains absent rather than becoming a failed
placeholder.

## Out Of Scope

- domain operations or payloads
- requests, replies, events, jobs, or retries
- TypeScript generation
- Tauri or service adapters
- authentication, endpoint admission, or credential storage
- topology selection or process launch

## Steps

1. Add the pure crate and its bounded identity types.
2. Define exact protocol-version and hello/receipt types.
3. Define host forms, transport features, and host/session descriptors.
4. Define connection states, causes, and permitted terminal/reconnect posture.
5. Define capability advertisement independent of authority.
6. Define per-domain availability, read/write authority, authority epoch, and
   optional revision descriptors.
7. Reject duplicate domain descriptors, ambiguous writers, invalid state
   combinations, and incompatible versions.
8. Add minimal, optional-service, multi-host, execution-only, and topology
   switch fixtures.
9. Audit dependencies, serialization, bounds, and public API.

## Acceptance Criteria

- exact v1 negotiation succeeds and every other version is incompatible
- all ids and descriptor collections are bounded and validated
- session id and host-instance id cannot be conflated
- capability advertisement does not grant read or write authority
- connection does not imply project/domain authority
- execution-only ownership can be represented without write authority
- one scope cannot declare multiple current writers
- query-only Bovine shape advertises no subscription or service feature
- Nucleus shape represents embedded and remote hosts plus per-domain authority
- Loophole shape changes host form without changing domain identity
- pure crate imports no Tauri, async runtime, network, renderer, or consumer

## Evidence Required

- negotiation success and rejection matrix
- state and transition validation matrix
- capability-versus-authority fixtures
- three donor-shaped topology fixtures
- serialization and bounded-input fixtures
- dependency and public-API audit
- focused Rust and Effigy checks

## Stop Conditions

- host form changes domain semantic identity
- authority requires a consumer payload type
- authentication must be inferred from capability or authority
- topology selection or process policy leaks into the pure crate
- multiple plausible protocol identities require a new product decision

## Next Task

Card 050 is ready. It defines typed operations, ordering, jobs, retry, and
indeterminate outcomes over this protocol.

## Result

`longhorn-bridge` now supplies bounded bridge, host, session, capability,
authority-scope, transport-feature, diagnostic, and domain identities. Exact
v1 hello and receipt types fail closed through validated construction and
deserialization.

Five host forms, checked connection state/reason pairs, authentication posture,
transport features, domain capabilities, per-domain read/write/execution
authority, nonzero epochs, and optional authoritative revisions remain
separate facts. One authority scope rejects multiple current writers.

Bovine, Nucleus, and Loophole-shaped fixtures prove the query-only floor,
execution-only ownership, embedded/remote host distinction, and topology
switches without semantic domain drift.

## Validation

- `cargo test -p longhorn-core -p longhorn-bridge`
- `cargo clippy -p longhorn-core -p longhorn-bridge --all-targets -- -D warnings`
- `effigy qa`
- exact-version, bounded-input, state/reason, capability/authority, donor, and
  serialization fixtures
- dependency audit: core plus Serde only; no Tauri, async, network, renderer,
  service, or consumer dependency

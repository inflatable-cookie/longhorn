# 036 Client Lifecycle And Domain-free Tauri Transport

Status: complete
Owner: Tom
Roadmap: g01.007 batch 1
Governing refs: contracts 010-013; research memo 011
Depends on: Card 035
Auto-start next card: no

## Objective

Add one small framework-neutral client lifetime foundation, align existing
clients to it, and remove the transfer dependency from the raw Tauri edge.

## Scope

- usable `@inflatable-cookie/longhorn-core` package
- structural invoke, event, and unlisten contracts
- checked subscription registration, synchronization, and teardown
- injected domain validation and freshness comparison
- Surface and transfer client consolidation
- domain-free `@inflatable-cookie/longhorn-tauri`
- package, import-safety, fault, and dependency checks

## Public Behavior

Listeners attach before the initial snapshot. A connection accepts only
domain-current snapshots and closes its listener exactly once, including when
registration resolves after disposal. Registration, validation, load, and
unlisten failures remain explicit.

`@inflatable-cookie/longhorn-core` does not define domain commands or freshness policy.
`@inflatable-cookie/longhorn-tauri` satisfies structural transport contracts without importing
transfer, Surface, layout, Svelte, or Poodle packages.

## Out Of Scope

- Svelte state
- Poodle components
- new layout host commands or events
- optimistic UI policy
- cross-window drag actions
- public package publication

## Steps

1. Add the pure core package and structural transport types.
2. Add a bounded checked subscription primitive.
3. Inject validation, freshness, snapshot loading, and failure reporting.
4. Make asynchronous disposal exact and idempotent.
5. Move Surface subscription lifetime onto the primitive.
6. Move transfer connection lifetime onto the primitive without erasing its
   client-epoch rule.
7. Replace the transfer-specific Tauri class with structural raw transport.
8. Make Tauri 2 a peer and remove mandatory domain dependencies.
9. Add fault, late-registration, import, and package-graph fixtures.

## Acceptance Criteria

- listener registration precedes snapshot load
- late async unlisten runs exactly once
- repeated dispose is safe
- stale Surface revision and stale transfer client epoch are ignored
- a newer domain epoch is accepted under its domain comparator
- validation or load failure tears down the listener
- module import touches no browser global
- core imports no host or UI package
- Tauri imports no Longhorn domain package
- direct and Surface-enabled existing client tests still pass

## Evidence Required

- lifecycle and failure matrix
- Surface and transfer regression fixtures
- SSR/import-safety checks
- package-content dry runs
- dependency report
- TypeScript and Effigy QA

## Stop Conditions

- one generic comparator cannot preserve a domain's freshness rule
- consolidation changes a serialized protocol
- Tauri transport needs domain behavior
- a layout service must be invented to complete the card
- a package name requires registry authority

## Next Task

Card 037 is ready. Add per-window Svelte reactive state without changing the
framework-neutral lifetime or optional package graph.

## Outcome

Completed 2026-07-29.

- added pure `@inflatable-cookie/longhorn-core` structural invoke/event transport contracts
- added one checked snapshot connection with listener-first registration,
  boolean-coalesced refresh, injected validation and freshness, explicit
  failure phases, and exact asynchronous teardown
- migrated Surface subscriptions while retaining epoch/revision invalidation
- migrated transfer connections while retaining client-epoch freshness
- removed transfer-specific transport aliases and call sites
- made `TauriTransport` structural, with only core as a Longhorn dependency
  and Tauri 2 as a peer
- added package-wide TypeScript checks, core lifecycle fixtures, import-safety
  checks, package dry runs, and dependency evidence

Evidence:
[Client Lifecycle And Domain-free Tauri Transport](../../../logs/2026-07/29-client-lifecycle-and-domain-free-tauri-transport.md).

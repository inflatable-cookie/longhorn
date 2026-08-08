# 037 Svelte Reactive Client State

Status: complete
Owner: Tom
Roadmap: g01.007 batch 2
Governing refs: contracts 010-013; research memo 011
Depends on: Card 036
Auto-start next card: no

## Objective

Add per-window Svelte 5 reactive state over checked clients without creating a
singleton, durable renderer authority, or mandatory Surface dependency.

## Scope

- private `@inflatable-cookie/longhorn-svelte` package
- Surface-free root lifecycle and status primitives
- optional domain subpaths
- Surface snapshot and mutation state
- transfer client, preparation, lease, completion, and cancellation state
- consumer-fed layout projection and request dispatch state
- request-id keyed optimistic projections
- mounted teardown and SSR checks

## Public Behavior

Each adapter instance has explicit `start`, `stop`, and destroy behavior.
State distinguishes idle, loading, ready, reconnecting, unsupported, and
failed. Authoritative snapshots always supersede optimistic projections under
domain revision rules.

Layout projection accepts checked authoritative documents and an injected
dispatcher. It does not imply a Longhorn layout IPC endpoint. Optional Surface
and Surface-transfer code resolves only through their explicit subpaths.

## Out Of Scope

- Poodle imports
- component rendering
- panel catalogue, label, icon, or body ownership
- native drag payloads
- source aliases or public publication

## Steps

1. Add the package with Svelte 5 and core peer boundaries.
2. Add a Surface-free root entry.
3. Add explicit optional domain subpaths and optional peer metadata.
4. Bind core subscriptions to per-instance rune state.
5. Add consumer-fed layout projection and dispatch state.
6. Add Surface and transfer state adapters.
7. Track optimism by request id and reconcile authoritative responses.
8. Cancel timers, preparations, leases, and pending optimism on teardown.
9. Mount, unmount, remount, and destroy fixture components.
10. Check SSR imports, subpath resolution, and package contents.

## Acceptance Criteria

- import creates no singleton and reads no browser global
- two windows can hold independent instances
- start and stop are explicit and idempotent
- late listener registration cannot leak after unmount
- stale optimistic completion cannot replace a newer snapshot
- unsupported capability differs from transport failure
- root import resolves no Surface package
- Surface subpaths fail clearly when their optional peer is absent
- mounted fixtures leave no listener, timer, preparation, or lease

## Evidence Required

- reactive state transition table
- request-id reconciliation matrix
- two-window isolation fixture
- repeated mounted lifecycle fixture
- SSR and subpath import checks
- package and dependency reports
- Svelte check, TypeScript, and Effigy QA

## Stop Conditions

- Svelte state must become durable fallback truth
- an optional domain becomes a root dependency
- a mounted test requires a consumer repository
- the package needs Poodle to compile
- current clients cannot expose needed teardown without changing Card 036

## Next Task

Run the Poodle public seam checkpoint in Card 038 before building visual or
cross-window drag bindings.

## Outcome

Completed 2026-07-29.

- added private `@inflatable-cookie/longhorn-svelte` with a Surface-free root
- added per-instance rune-backed lifecycle and explicit status
- added consumer-fed layout projection and dispatch state
- added request-id keyed optimistic reconciliation
- added optional Surface, transfer, and Surface-transfer subpaths
- made concurrent mounted and explicit teardown promise-idempotent
- added exact timer, late preparation, session, lease, and listener cleanup
- proved SSR import, two-window isolation, repeated mount, package boundaries,
  and Svelte 5.38.6 through 5.56.8 compatibility

Evidence:
[Svelte Reactive Client State](../../../logs/2026-07/29-svelte-reactive-client-state.md).

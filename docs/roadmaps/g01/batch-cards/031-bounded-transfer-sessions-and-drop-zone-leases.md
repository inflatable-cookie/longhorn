# 031 Bounded Transfer Sessions And Drop-zone Leases

Status: complete
Owner: Tom
Roadmap: g01.006 batch 3
Governing refs: contracts 001, 009-012, and 014; research memo 010
Depends on: Card 030 checkpoint
Auto-start next card: no

## Objective

Add the framework-neutral bounded session, lease, expiry, cancellation, and
deterministic target-resolution core.

## Scope

- `longhorn-transfer`
- unguessable injected session-id allocation
- injected monotonic clock
- finite session and lease limits
- source authority records containing ids only
- complete replacement leases per window and client epoch
- bounded `ScreenDip` drop zones and insertion positions
- explicit-zone and screen-point target resolution
- expiry, cancellation, replay, ambiguity, and window-destroy behavior
- direct-container target bindings without Surface types

## Public Behavior

Session payloads expose only protocol version and session id. Session and lease
registries are process-local, finite, expiring, and never persisted.

Target resolution rejects overlaps instead of choosing by array order.
Invalid replacement leaves the prior lease intact. The first terminal commit
attempt or cancellation consumes the session.

## Out Of Scope

- panel or Surface mutation
- Tauri handlers
- TypeScript generation
- DOM measurement or Poodle behavior
- window provisioning
- durable session recovery

## Steps

1. Define bounded session, lease, zone, capability, and target identities.
2. Define injected entropy, monotonic clock, and finite policy.
3. Create sessions only from host-resolved source authority.
4. Publish complete lease generations atomically.
5. Validate typed geometry, ownership, count, and expiry.
6. Invalidate by newer generation, client epoch, window destroy, and time.
7. Resolve explicit zone ids against current leases.
8. Resolve screen points against current windows and zones.
9. Reject all ambiguous overlap deterministically.
10. Add cancellation, replay, capacity, and clock-boundary fixtures.

## Acceptance Criteria

- payload contains no subject snapshot or product state
- allocator failure and capacity exhaustion allocate nothing
- invalid lease replacement preserves the current generation
- expired or destroyed authority cannot resolve
- explicit-zone and screen-point paths agree
- overlapping eligible windows or zones are ambiguous
- insertion positions are bounded and advisory
- cancellation is idempotent
- terminal replay is rejected
- direct-container fixtures import no Surface package

## Evidence Required

- payload serialization fixture
- session and lease limit matrix
- fake-clock expiry boundaries
- replacement and epoch transition fixtures
- overlap and permutation matrix
- cancellation and single-use matrix
- package dependency report
- Rust 1.85 and full Effigy QA

## Stop Conditions

- target selection needs renderer focus or enumeration order
- durable state must enter the payload
- a Surface type is required by the base transfer crate
- ambient wall-clock or random generation is required
- client geometry is treated as screen geometry without checked projection

## Next Task

Start Card 032.

## Outcome

Implemented `longhorn-transfer` as a Surface-free process-local coordinator.
Session ids contain exactly 128 allocator-supplied bits. Time and entropy are
injected; no ambient clock or random source exists.

Finite policy independently bounds sessions, current client windows, leases,
zones, insertion positions, and lifetimes. Session admission checks current
client epoch, lifetime, reclaimable capacity, and allocator success before
publication. Cancellation is idempotent. Expiry, source-window destroy,
client-epoch replacement, first terminal attempt, and host discard have
explicit state and receipts.

Lease publication validates the complete candidate before atomic replacement.
It rejects stale generations, stale client epochs, duplicate ids, zero,
overflowing, outside-window, over-limit, capability-mismatched, and
over-positioned zones while preserving the prior generation.

Explicit-zone and screen-point paths use current leases and fresh managed
window bounds. Missing, expired, stale, ineligible, overlapping-window, and
overlapping-zone outcomes are typed. Enumeration order never breaks ties. The
first active terminal attempt is consumed before target resolution, so every
target abort rejects replay.

Direct layout-region and opaque hosted-window bindings share this core.
`longhorn-transfer` depends only on core and serde and imports no Surface
package. Card 032 is ready.

# 028 Surface Identity, Topology, And Resolution

Status: complete
Owner: Tom
Roadmap: g01.006 batch 1
Governing refs: contracts 001, 002, 009, 012, and 014; research memo 010
Auto-start next card: no

## Objective

Add the pure optional Surface identity, bounded topology, validation,
normalization, presence input, and window-host resolution foundation.

## Scope

- `longhorn-surfaces` workspace crate
- bounded `SurfaceId`, request id, revision, and document limits
- Surface-to-layout-container bindings
- optional labels
- ordered window-host preferences
- ordered hosted-Surface and active-Surface state
- consumer-resolved presence input
- deterministic resolution against available participating windows
- typed unresolved outcomes
- Loophole-shaped and minimal no-Surface dependency fixtures

## Public Behavior

Every Surface binds one distinct layout container and resolves to at most one
available window. Product presence predicates arrive only as current admitted
ids. Missing preferred windows try explicit fallbacks; no host remains an
unresolved result.

Normalization preserves declared Surface order, canonicalizes structural
records, and validates active membership. It does not silently repair duplicate
ownership, invent a fallback, create a window, or mutate preferred hosting.

## Out Of Scope

- lifecycle mutation
- configuration persistence
- native window apply
- transfer sessions or drop zones
- TypeScript, Svelte, Poodle, Tauri, or donor changes
- product presence-expression languages

## Steps

1. Add bounded Surface identity and monotonic revision types.
2. Add the optional pure workspace crate with explicit dependency limits.
3. Define document and resolver limits.
4. Define Surface records and distinct layout-container bindings.
5. Define participating-window hosting preferences, order, and active state.
6. Validate every identity, reference, uniqueness, count, and active member.
7. Normalize structural order without changing declared tab order.
8. Apply consumer-resolved presence and current window availability.
9. Return resolved windows and typed unresolved Surfaces.
10. Add permutation, absence, fallback, and donor-shaped fixtures.

## Acceptance Criteria

- invalid, duplicate, or excessive identities fail typed
- revision and count overflow never wrap
- one layout container cannot bind two Surfaces
- one Surface cannot resolve into two windows
- active Surface is absent or belongs to its resolved window
- missing preferred windows follow explicit fallback order
- no available host remains unresolved without state mutation
- presence input contains no product predicate or payload
- input permutations produce one resolved snapshot
- `longhorn-layout` and Nucleus-shaped fixtures import no Surface dependency
- package graph contains no config, Tauri, TypeScript, Svelte, or Poodle

## Evidence Required

- identity and limit rejection matrix
- topology invariant and normalization fixtures
- preferred/fallback resolution table
- presence and missing-window fixtures
- Loophole multi-window/multi-Surface fixture
- dependency report proving optionality
- serde, Rust 1.85, and full Effigy QA

## Stop Conditions

- the model requires Loophole's presence-clause schema
- Surface state must absorb window geometry or panel contents
- one container must bind multiple Surfaces
- missing hosts require implicit window creation
- product policy is needed to choose a generic fallback

## Outcome

`longhorn-core` now supplies bounded `SurfaceId`, `SurfaceRequestId`, and
checked `SurfaceRevision`. The optional `longhorn-surfaces` crate supplies:

- explicit finite Surface, participating-window, host-preference, and label
  limits
- strict Surface records with one distinct external layout-container binding
- ordered candidate windows with complete per-window tab order
- participating-window active-Surface preference
- typed topology validation and canonical structural normalization
- current admitted-Surface and available-window input without product
  predicates
- deterministic preferred/fallback resolution
- typed not-admitted and no-available-window outcomes

Resolution never changes the source document, creates a host, repairs invalid
topology, or imports layout contents. Loophole-shaped fixtures cover three
Surfaces, two windows, ordered fallback, active selection, and opaque layout
bindings. Existing Nucleus layout fixtures and `longhorn-layout` retain no
Surface dependency.

## Next Task

Card 029 is ready. Implement authoritative Surface lifecycle and registered
persistence without changing this public document shape.

# 062 Lossless History Donor Fixtures And Typed Foundation

Status: complete
Owner: Tom
Roadmap: g01.011 batch 1
Governing refs: contracts 001, 003, 008, and 012; research memo 015
Depends on: Card 061
Auto-start next card: no

## Objective

Freeze the current Loophole linear mechanics as consumer-neutral evidence and
implement the pure generic history identity, entry, payload-policy, and linear
state foundation.

## Scope

- `longhorn-history` crate
- bounded history, entry, kind, group, and plan identities
- distinct history revision and entry sequence
- generic typed payload
- injected inverse, coalesce, and no-op policy
- linear applied/future state without product apply
- Loophole-shaped fixture for record, inverse, coalesce, redo clearing,
  limits, persistence shape, and current projection
- non-editor typed document fixture
- explicit donor parity and correction matrix

## Public Behavior

The crate stores an arbitrary typed payload without interpreting it. Consumer
policy decides inverse, no-op, and adjacent coalescing. Longhorn validates
identities, bounds, state revision, and the structural effect of those
decisions.

A newly committed linear entry after undo removes the future path. Entry ids
come from an injected source. Product-model revision and history revision
cannot be confused.

## Out Of Scope

- product apply or navigation commit
- persistence encoding
- transition journals
- TypeScript, Tauri, Svelte, or Poodle
- branch trees
- donor repository writes

## Steps

1. Add the pure crate and bounded identity/revision types.
2. Define generic entry metadata and typed payload storage.
3. Define pure inverse, no-op, and coalesce policy results.
4. Implement empty, applied, and future linear state invariants.
5. Implement successful record and linear redo-clearing transitions.
6. Add count and metadata bounds without final pruning policy.
7. Freeze the corrected Loophole behavior matrix from memo 015.
8. Add a materially different non-editor document fixture.
9. Audit dependencies, generic payload leakage, and serialization assumptions.

## Acceptance Criteria

- consumer payload remains a generic Rust type
- inverse, coalesce, and no-op behavior is injected
- record requires the current history revision
- divergent linear record clears future entries exactly
- no-op and coalesced removal produce explicit structural outcomes
- entry ids and revisions are stable and distinct
- Loophole and non-editor fixtures share one public API
- no Pulse type or arbitrary JSON enters the crate
- the crate imports no config, bridge, Tauri, async runtime, Svelte, or Poodle

## Evidence Required

- corrected donor behavior fixture and parity table
- typed policy success and rejection matrix
- divergent record and future-clear fixtures
- id, revision, bound, overflow, and insertion-order fixtures
- non-editor fixture
- dependency and public-API audit
- focused Rust, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- product apply is required to represent structural state
- inverse requires a shared product mutation enum
- payloads must become arbitrary JSON
- entry identity depends on wall-clock time or ambient randomness
- the donor fixture claims live grouping or branching

## Next Task

Card 063 is ready. Add revision-bound navigation plans and atomic
apply/commit failure invariance after the foundation passes.

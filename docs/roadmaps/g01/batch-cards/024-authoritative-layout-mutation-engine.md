# 024 Authoritative Layout Mutation Engine

Status: complete
Owner: Tom
Roadmap: g01.005 batch 2
Governing refs: contracts 001, 002, 010, and 014; research memo 009
Auto-start next card: no

## Objective

Add one atomic expected-revision engine for structural layout, active state,
sizing, and collapse mutation.

## Scope

- request ids and expected layout revision
- create, close, activate, complete reorder, and cross-region/container move
- sizing-slot and collapsible-region mutation
- explicit insertion positions
- full candidate validation and deterministic normalization
- typed rejection with unchanged-state evidence
- previous/committed revision and authoritative snapshot receipts
- optional injected bounded request-id replay authority

## Public Behavior

The engine validates current state and policy, applies one command to a private
candidate, normalizes, validates again, then commits one revision. Failure
returns the original revision and no candidate state.

Create uses a caller-supplied instance id. Duplicate singleton creation is a
typed rejection, not implicit focus. Reorder requires a complete permutation.
Move rechecks target eligibility and instance limits.

Closing or moving an active panel selects the item now at its old index, then
the previous final item when required. An empty region has no active panel.

## Out Of Scope

- persistence or debounce
- multi-command transaction batches
- product resource teardown
- cross-window drag sessions
- optimistic renderer state
- TypeScript, Svelte, Poodle, Tauri, or donor writes

## Steps

1. Define strict request, command, receipt, and rejection envelopes.
2. Implement checked expected-revision admission.
3. Implement panel creation and instance-limit enforcement.
4. Implement close policy and deterministic active fallback.
5. Implement explicit activation.
6. Implement complete-permutation same-region reorder.
7. Implement cross-region and cross-container move with insertion.
8. Implement bounded sizing-slot mutation.
9. Implement supported collapse mutation.
10. Commit only a fully valid normalized candidate.
11. Add optional injected bounded idempotency behavior.
12. Add permutation, stale-state, and rejection invariance tests.

## Acceptance Criteria

- stale expected revision rejects without mutation
- successful command increments revision exactly once
- revision overflow rejects without mutation
- duplicate instance id and instance-limit violations fail
- create cannot target an ineligible region
- non-closeable or non-movable instances remain unchanged
- incomplete, duplicate, foreign, or reordered-foreign ids fail reorder
- move is one atomic remove/insert operation
- active fallback is exact after close and move
- invalid ratio or unsupported collapse fails
- every rejection preserves an equal document and revision
- request replay exists only when an explicit store is injected
- command and receipt ordering is deterministic

## Evidence Required

- command-by-command success matrix
- exact failure invariance matrix
- singleton, one-per-container, bounded, and explicit-multiple fixtures
- active-selection transition table
- insertion and permutation fixtures
- stale, overflow, and optional-idempotency fixtures
- Loophole and Nucleus shaped mutation sequences
- serde and Rust 1.85 evidence
- full Effigy QA

## Stop Conditions

- mutation must call Tauri, configuration, or product cleanup
- renderer state must become mutation authority
- partial command effects must escape on rejection
- cross-window transfer policy is required
- Card 023 public shape must change materially without roadmap reassessment

## Outcome

`longhorn-layout` now exposes a strict `LayoutMutationRequest` carrying bounded
`LayoutRequestId`, expected revision, and one command. The engine implements:

- create with caller-supplied instance id and explicit insertion
- close with registered policy and former-index active fallback
- activation and exact complete same-region reorder
- atomic cross-region and cross-container move
- bounded sizing-slot and supported collapse mutation
- one checked revision commit with a normalized authoritative snapshot

Every command runs against a private candidate. Invalid current state, stale
revision, overflow, unknown identity, policy failure, malformed reorder,
invalid insertion, sizing, collapse, or candidate state returns a typed
rejection containing the exact unchanged source document and revision.

Successful receipts contain request id, previous and committed revision,
command-specific evidence, and the complete authoritative document. Replay is
absent from ordinary execution. `BoundedLayoutReplayStore` adds exact
successful-request replay only when explicitly passed to the engine; request
id reuse with different content fails typed.

Seventeen mutation tests cover every command, exact rejection invariance,
active-selection transitions, insertion and permutation behavior, singleton,
one-per-container, bounded, and multiple policy, replay and eviction,
structural input permutation, strict serde, and Loophole eight-region and
Nucleus five-region sequences.

## Next Task

Cards 025-027 and `g01.005` are complete. Card 028 is ready under the compiled
`g01.006` runway.

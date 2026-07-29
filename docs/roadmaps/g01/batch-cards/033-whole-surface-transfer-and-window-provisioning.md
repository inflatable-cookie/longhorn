# 033 Whole-Surface Transfer And Window Provisioning

Status: planned
Owner: Tom
Roadmap: g01.006 batch 4
Governing refs: contracts 001, 002, 004, 009-012, and 014; research memo 010
Depends on: Cards 029-032 and completed g01.004
Auto-start next card: no

## Objective

Apply the shared transfer protocol to whole-Surface moves and add explicit,
receipted empty-display window provisioning.

## Scope

- `longhorn-surface-transfer` optional adapter crate
- fresh Surface source admission and target re-resolution
- expected-revision move between participating windows
- retained Surface-to-layout-container binding
- consumer policy for allowed targets
- optional empty-display target
- injected window provisioner and cleanup authority
- existing hidden creation, placement, readiness, and close seams
- commit, cleanup, partial-failure, and reconciliation receipts
- donor-shaped screen-point behavior without donor repair rules

## Public Behavior

A whole-Surface move commits one Surface document. The bound layout container
and its contents do not serialize or change.

Empty-display creation is disabled unless consumer policy enables it. The
provisioner creates a neutral hidden target and returns cleanup authority.
Surface commit occurs only after provisioning succeeds. Failed commit invokes
cleanup and reports both outcomes.

## Out Of Scope

- inferred product window roles or URLs
- panel-to-new-window creation
- layout cloning or deletion
- silent orphan cleanup
- TypeScript, Svelte, Poodle, or donor migration

## Steps

1. Admit whole-Surface sessions from fresh topology state.
2. Resolve current participating-window targets.
3. Commit expected-revision Surface moves.
4. Preserve the external layout-container binding.
5. Define explicit empty-display target and policy input.
6. Define provisioner commit and cleanup receipts.
7. Compose neutral hidden window creation with current placement/readiness.
8. Roll back provisioned targets when Surface commit fails.
9. Report unresolved cleanup as host reconciliation failure.
10. Add move, provision, target-loss, and failure-injection fixtures.

## Acceptance Criteria

- stale or moved source aborts unchanged
- target policy and participation are rechecked
- successful move increments only Surface revision
- layout document and container binding remain exact
- empty-display behavior is off by default
- provision failure leaves the source unchanged
- commit failure invokes cleanup
- cleanup failure is typed and inspectable
- no product URL, title, role, or capability default enters the package
- native target remains hidden until placement and readiness

## Evidence Required

- ordinary whole-Surface move matrix
- exact layout-binding preservation proof
- disabled-policy and target-loss fixtures
- provision/commit/cleanup failure table
- mock window-host receipt chain
- donor behavior delta report
- Rust 1.85 and full Effigy QA

## Stop Conditions

- Surface move requires layout mutation
- provisioning cannot return cleanup authority
- native creation precedes policy admission
- source mutation must occur before target readiness
- product window defaults become shared behavior

## Next Task

Card 034 remains planned until whole-Surface commit and provision cleanup are
fully receipted.

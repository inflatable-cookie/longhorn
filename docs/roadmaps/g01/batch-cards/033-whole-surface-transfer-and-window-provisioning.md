# 033 Whole-Surface Transfer And Window Provisioning

Status: complete
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

Start Card 034.

## Outcome

`longhorn-surface-transfer` now admits a whole-Surface session only after
loading the registered Surface domain and matching the current primary host
to a fresh opaque binding. Terminal commit consumes the session, reloads the
Surface document, rechecks source, revision, target participation, declared
host preference, host binding, consumer target policy, insertion, and
empty-window policy, then publishes the existing expected-revision
`MoveSurface`.

Ordinary and provisioned moves advance one Surface revision. The layout
document is immutable input, no layout payload enters transfer state, and the
Surface retains its exact external layout-container binding.

The transfer core now distinguishes a screen point outside every fresh
managed window from missing lease or zone authority inside a managed window.
Only the former may enter the Surface adapter's empty-display path.

Provisioning is disabled by default. Enabled policy supplies current display
bounds, a predeclared logical target window, exact placement, and optional
insertion. The injected provisioner returns a hidden, placed, ready receipt
plus retained authority. Surface publication happens next. Failure invokes
cleanup. Cleanup failure and host commit failure after durable publication
return typed reconciliation evidence with the available receipts.

### Failure table

| Failure point | Surface state | Host action | Evidence |
| --- | --- | --- | --- |
| policy or target rejection | unchanged | none | consumed typed abort |
| create, placement, or readiness | unchanged | provision failed | stage failure |
| expected-revision publication | current authority retained | cleanup invoked | provision plus cleanup outcome |
| cleanup | current authority retained | unresolved target | host reconciliation required |
| host commit after publication | committed move retained | unresolved finalization | publication plus provision and commit failure |

### Donor delta

Retained from Loophole:

- whole-Surface movement by screen point
- optional new host on empty display space
- logical window target selected before movement

Changed for shared authority:

- overlap is ambiguous; enumeration order never selects
- target topology must already be declared; transfer performs no repair
- consumer policy supplies display bounds and placement with no product
  defaults
- native creation follows policy admission and remains hidden through
  placement and readiness
- failed publication invokes explicit cleanup
- partial host outcomes are typed reconciliation, never silent repair

Card 034 is ready.

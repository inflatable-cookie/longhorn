# 029 Authoritative Surface Lifecycle And Persistence

Status: complete
Owner: Tom
Roadmap: g01.006 batch 1
Governing refs: contracts 001, 002, 004, 010, 012, and 014; research memo 010
Depends on: Card 028
Auto-start next card: no

## Objective

Add expected-revision Surface lifecycle and exact registered configuration
persistence without merging Surface, layout, or window authority.

## Scope

- create, duplicate metadata, rename, activate, reorder, move, and close
- caller-supplied fresh Surface and layout-container ids
- target-container existence and uniqueness evidence
- exact active-member fallback
- typed rejection with unchanged-state evidence
- `longhorn-surfaces-config`
- registered domain, migration, backup policy, and coordinated publication
- independent Surface/layout/window domain conformance

## Public Behavior

Every command validates fresh state, applies to a private candidate, normalizes,
revalidates, and commits one revision or none. Duplicate copies generic Surface
metadata only. Close returns layout-container cleanup intent; it never deletes
layout or product state.

The persistence adapter accepts an exact consumer descriptor and publishes a
complete Surface document under store coordination. Future, corrupt, or
incompatible state enters typed recovery.

## Out Of Scope

- cloning layout-container contents
- automatic cleanup execution
- cross-domain atomic mutation
- native window creation or apply
- transfer sessions, TypeScript, Svelte, Poodle, or donor changes

## Steps

1. Define strict request, command, receipt, and rejection envelopes.
2. Add expected-revision admission and exact failure invariance.
3. Implement create and generic metadata duplication.
4. Implement rename, activation, and complete reorder.
5. Implement cross-window host move.
6. Implement close and deterministic active fallback.
7. Return explicit layout-container cleanup intent.
8. Add registered Surface domain and migration policy.
9. Publish fresh complete documents under the configuration coordinator.
10. Prove independent Surface, layout, and window domains preserve each other.

## Acceptance Criteria

- stale, invalid, or overflow requests preserve exact source state
- success increments revision once
- create and duplicate require caller-supplied fresh ids
- target layout container must exist and be unbound
- duplicate copies no panels or product resources
- move and close select active fallback exactly
- consumer policy controls whether a window may become empty
- close returns cleanup intent without executing it
- registered publication rechecks fresh revision
- one domain cannot replace another
- changed document shape requires explicit migration

## Evidence Required

- command success and rejection matrices
- exact active transition table
- duplicate and cleanup-intent fixtures
- stale concurrent writer fixture
- recovery and migration fixtures
- three-domain independence proof
- Rust 1.85 and full Effigy QA

## Stop Conditions

- duplicate requires hidden product or layout cloning
- close must delete another domain to be correct
- multi-domain atomicity becomes required
- Card 028 public shape changes materially
- persistence scope must be inferred

## Outcome

`longhorn-surfaces` now supplies:

- strict expected-revision create, duplicate, rename, activate, reorder, move,
  and close commands
- caller-supplied fresh Surface and layout-container ids
- a read-only `LayoutContainerInventory` evidence boundary without a layout
  package dependency
- exact unchanged documents on typed rejection
- one checked revision advance on success
- declared-target move with deterministic primary-host promotion
- exact former-index then previous-final active fallback
- consumer-selected allow/reject empty-window policy
- explicit unexecuted layout-container cleanup intent on close

`longhorn-surfaces-config` now supplies exact consumer-registered Surface
domains, strict current raw shape, explicit one-step migrations, backup
participation, and immediate coordinated complete-document publication.
Publication derives container evidence from a caller-supplied
`LayoutDocument` but mutates only the Surface domain.

Contract fixtures cover all commands, stale/invalid/overflow invariance,
identity and container rejection, active transitions, generic-only duplicate,
cleanup intent, corrupt/future/incompatible recovery, explicit shape
migration, same-revision concurrent writers, backup policy, and independent
Surface/layout/window publication.

## Next Task

Card 030 is ready. Compose Surface resolution with the existing window host
without moving geometry, factory policy, or native apply authority into the
Surface packages.

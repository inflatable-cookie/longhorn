# Nucleus Renderer And Poodle Cutover

Date: 2026-08-01
Status: complete
Card: 099

## Outcome

Nucleus commit `74ca4e7c72f447e064419de6dc72502265cbbf49`
replaces renderer-owned aggregate workspace saves with exact Longhorn layout
commands. Longhorn commit `ca755cbd332260abd971d86509f6190a0e76d269`
supplies the small public Poodle additions required by the real consumer.

One `WorkspaceLayoutSession` exists per selected project. It connects through
`CheckedSnapshotConnection`, registers the event listener before loading the
first snapshot, filters project scope, accepts only newer projection revisions,
and owns one client epoch and request sequence. `LayoutState` owns
request-keyed optimistic projection. Project switch, stop, and destruction
clear the listener, snapshot, binding, pending presentations, and optimism.

The host accepts generated `LayoutMutationRequest` commands. Create and close
stage the Nucleus product-presentation domain before layout publication and
roll it back if layout publication fails. Titles, icons, resources, editor and
forge refs, panel bodies, terminal/browser handles, and cleanup remain outside
the shared document.

## Public Poodle Composition

Nucleus now composes `LayoutDockRegion` and `LayoutSplitView`; tabs and drag
semantics remain inside those public bindings. Longhorn adds:

- `@inflatable-cookie/longhorn-poodle/binding` for state-only consumers
- DockRegion size, semantic size-role, and density pass-through
- projection-only `primaryHidden` and `secondaryHidden` SplitView inputs
- zero-size, disabled hidden panes without durable collapse commands

The Poodle package gate exposed stale test ids from an older generic Nucleus
shape. Tests now address the accepted `left`, `center_top`, `center_bottom`,
`right_top`, `right_bottom`, and four-slot fixture.

Nucleus retains the five-region frame, empty-region choice, panel presentation,
product routing, and same-window drag reveal policy. It inspects no Poodle
class, generated tab id, portal structure, or MIME.

## Native Overlay Visibility

Popover and Menu feed public surface-geometry changes into a Nucleus adapter.
Browser panels feed explicit viewport geometry. The adapter recomputes the
final intersected panel-id set when either side moves or disappears. It imports
public Poodle geometry types only. Project management keeps its independent
hide-all policy.

The former `querySelector` path and `data-native-browser-*` discovery markers
are removed. Tests cover Popover movement between Browser panels, nested Menu
surface maps, and Browser viewport movement.

## Lifecycle Evidence

Mounted Svelte tests prove:

- project A cannot publish into mounted project B
- a late project-A mutation fails with `StaleWorkspaceLayoutEpochError`
- old listeners dispose on project switch and unmount
- old optimism is empty after teardown
- a later project C remount starts clean

The old `workspaceUi.ts` whole-snapshot authority and its tests are deleted.
The app exposes loading, reconnecting, failure, and retry states from the
checked session.

## Validation

- `effigy check:rust`: pass
- Nucleus workspace UI authority: 8 passed
- Nucleus panel guards: 10 passed
- Nucleus renderer unit tests: 38 passed
- Nucleus mounted lifecycle: 2 passed
- Nucleus production renderer build: pass
- Nucleus Svelte check: 0 errors; one pre-existing ProjectRail ARIA warning
- frozen Bun install: pass
- Longhorn Poodle tests: 18 passed
- Longhorn Poodle TypeScript/Svelte/package checks: pass
- `effigy proof:nucleus-renderer-cutover`: pass

The lock uses the private sibling `file:` graph with exact overrides. This is
development source consumption, not package artifact or release evidence.
Package-manager publication remains deferred.

## Next

Card 100 replaces Nucleus Browser child-view coordination with the production
native-content graph while retaining Nucleus browser and security policy.

# 092 Svelte Native-content Session And Layout Seam

Status: complete
Owner: Tom
Roadmap: g01.018 batch 3
Governing refs: contracts 010, 013, and 017; Card 086
Depends on: Cards 088, 089, and 091
Auto-start next card: no

## Objective

Add one per-instance Svelte lifecycle for checked native-content coordination.
Prove consumer composition with public layout elements without depending on
Poodle or inspecting its private DOM.

## Scope

- `@inflatable-cookie/longhorn-native-content-svelte`
- mounted client and attach-generation lifetime
- consumer-supplied viewport element measurement
- current scale input and resize observation
- explicit visibility inhibitors, focus intent, and input mode
- serialized updates, stale async rejection, remount, and teardown
- child-view and backing-surface composition fixtures
- public Poodle-seam proof through consumer-owned layout markup

## Out Of Scope

- Poodle component copies or package dependency
- DOM overlay discovery or inferred occlusion
- product browser, renderer, or semantic input behavior
- native host implementation
- donor migration

## Acceptance Criteria

- each mounted instance owns isolated state and one connection lifetime
- measurement names the current attach generation and explicit scale
- consumers submit final visibility policy; adapter never inspects overlays
- resize, scale input, focus, and visibility changes serialize deterministically
- stale results cannot cross remount or generation changes
- teardown removes every listener, observer, timer, and pending callback
- public Poodle composition works without private DOM knowledge or dependency

## Evidence Required

- two-instance, remount, stale-result, and teardown traces
- viewport and visibility policy fixtures
- public layout/Poodle-seam artifact
- dependency and DOM-authority audit
- focused TypeScript, Svelte, docs, and Effigy checks

## Stop Conditions

- reusable lifetime requires private Poodle DOM inspection
- one global store is required for multiple windows or islands
- semantic input payloads enter the adapter
- renderer measurement becomes durable authority without host admission

## Next Task

Execute Card 093. Prove every selected graph from produced artifacts and close
the production gate before donor migrations.

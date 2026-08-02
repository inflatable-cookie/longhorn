# 123 Jetstream Backing-surface Coordination Cutover

Status: ready
Owner: Tom
Roadmap: g01.016 batch 4
Governing refs: contracts 003, 009-010, 012-013, and 017; Card 122
Depends on: Card 122
Auto-start next card: no

## Objective

Adopt Longhorn's backing-surface desired/observed coordination and Svelte
viewport lifetime around Jetstream's engine-owned WGPU view.

## Repository Scope

- Longhorn: backing-surface and Svelte adapters, fixtures, evidence, and docs.
- Jetstream: native attachment seam, renderer composition, viewport session,
  tests, and docs.
- Poodle: read-only public composition.

## Scope

- stable island identity, attach generation, host binding, and full-host storage
- CSS viewport measurement, physical scale, clipping, visibility, and focus gates
- deterministic render and semantic-input admission
- resize, remount, page-not-loaded, window-destroyed, failure, and teardown paths
- macOS native view attachment and explicit Windows/Linux unsupported behavior
- no-surface diagnostic mode as explicit product policy

## Steps

1. Freeze native attach, viewport, scale, input, resize, and destroy traces.
2. Bind one backing-surface record to the editor window and attach generation.
3. Store the engine-owned native view through an injected consumer port.
4. Replace renderer measurement/listener lifetime with one checked Svelte session.
5. Gate rendering and semantic input from checked clipped physical viewport state.
6. Preserve WGPU construction, resize, frame loop, scene, and input mapping downstream.
7. Make detach/terminal observation explicit without pretending plugin portability.
8. Remove only superseded generic coordination and listener code.

## Acceptance Criteria

- Longhorn owns coordination, generation, clip, gates, planning, and receipts only
- Jetstream owns NSView/WGPU storage, renderer, frame loop, world, camera, picking, and gizmos
- Svelte owns no durable native-content authority
- stale generations and remounted renderers cannot drive the current surface
- clicks outside the admitted viewport do not reach the engine
- window destruction terminates coordination and the render loop deterministically
- no raw pointer or WGPU type crosses Longhorn's public boundary
- Windows/Linux and live scale-transition support remain evidence-bounded

## Stop Conditions

- the generic package must construct WGPU or interpret semantic input
- consumer-owned native storage cannot satisfy explicit detach/terminal evidence
- the existing process-lifetime leak is required but cannot be truthfully modelled
- unsupported platforms would appear successful

## Next Task

Execute Card 124. Prove and close Jetstream's selected composition.

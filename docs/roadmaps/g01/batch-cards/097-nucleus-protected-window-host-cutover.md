# 097 Nucleus Protected Window Host Cutover

Status: ready
Owner: Tom
Roadmap: g01.014 batch 2
Governing refs: contracts 003, 004, 009, and 012; Cards 094 and 096
Depends on: Card 096
Auto-start next card: no

## Objective

Replace Nucleus primary-window restore, capture, settling, persistence, reveal,
and shutdown mechanics with the Longhorn protected single-window host.

## Repository Scope

- Nucleus desktop host, capability policy, and focused tests may change.
- Longhorn may receive donor conformance fixtures and migration evidence.

## Scope

- one protected predeclared `main` window
- display observation, correlation, fallback, clamping, and maximized state
- explicit physical/logical scale mapping
- hidden restore and guarded reveal
- user/programmatic event attribution
- settled placement capture into the window config domain
- focus-loss, close, and aggregate shutdown flush policy
- removal of superseded `window_geometry.rs` mechanics

## Steps

1. Freeze the migrated placement fixture and bind stable main-window identity.
2. Assemble the protected predeclared host with injected Nucleus policy.
3. Restore against fresh displays before reveal with explicit scale mapping.
4. Wire page readiness, user/programmatic event attribution, and settled capture.
5. Publish placement through the separate coordinated window domain.
6. Wire bounded focus-loss, close, shutdown flush, and host teardown.
7. Compare restart and display-fallback traces to Card 094 evidence.
8. Remove the old geometry worker and duplicate native lifecycle path.
9. Audit capabilities, retained policy, failures, and rollback.

## Acceptance Criteria

- saved, intersecting, main, and deterministic display fallback match fixtures
- removed-display restore always produces a visible bounded window
- normal bounds and maximized state survive restart
- renderer layout writes cannot replace window placement
- programmatic restore events do not persist as user moves
- startup reveals only after native convergence and renderer readiness
- close and shutdown return bounded inspectable flush outcomes
- the old geometry worker and duplicate display/window helpers are removed
- Nucleus retains main-window role, defaults, titlebar, and close policy

## Evidence Required

- migrated placement fixture and restart trace
- display fallback and scale report
- event attribution, flush, failure, and teardown receipts
- capability diff
- duplicate-code and retained-policy audit
- focused Nucleus and Longhorn conformance tests

## Stop Conditions

- the host cannot map Nucleus physical geometry without ambiguous scale
- restore changes accepted window behavior without operator policy
- renderer readiness has no explicit signal
- another live writer can overwrite the window domain
- donor worktree changes overlap host or geometry files

## Next Task

Execute Card 098. Transfer the project-keyed five-region document and mutation
authority without introducing Surface state.

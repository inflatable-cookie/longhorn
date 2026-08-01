# 098 Nucleus Project Layout Authority Cutover

Status: ready
Owner: Tom
Roadmap: g01.014 batch 3
Governing refs: contracts 003, 004, 012, and 014; Cards 094, 096, and 097
Depends on: Card 097
Auto-start next card: no

## Objective

Replace Nucleus generic workspace-layout mechanics with registered Longhorn
layout authority while preserving project scope and product panel policy.

## Repository Scope

- Nucleus workspace and desktop host code plus focused tests may change.
- Longhorn may receive Nucleus conformance fixtures and migration evidence.

## Scope

- five-region schema and four sizing slots
- project-keyed layout containers
- panel definition registry and consumer-supplied instance ids
- expected-revision create, close, activate, reorder, move, and resize
- new-project Agent Chat seed policy
- product resource and panel-runtime adapters outside layout records
- registered layout persistence and bounded sizing debounce
- generic `nucleus-workspaces` duplicate removal

## Steps

1. Freeze the accepted g05 project-layout schemas and migration fixtures.
2. Register the five regions, four sizing slots, and product panel definitions.
3. Map project ids to consumer-owned layout-container scopes.
4. Import each project layout into the registered configuration domain.
5. Replace structural helpers with expected-revision Longhorn mutations.
6. Keep resources, titles, bodies, runtimes, and cleanup in product adapters.
7. Exercise switching, late-result, policy, and failure invariance.
8. Remove generic donor mechanics and audit the retained workspace crate.
9. Prove separate window/layout writes and exact Surface absence.

## Acceptance Criteria

- two projects retain distinct panels, order, active tabs, regions, and ratios
- rapid switching cannot apply a late load or mutation to another project
- new projects contain exactly one product-defined Agent Chat panel
- resources, titles, bodies, terminal/browser handles, and cleanup stay external
- every rejected or stale mutation preserves document and revision
- layout and window domains mutate independently
- no Surface id, type, package, protocol, or state resolves
- Nucleus retains only product registry, scope, planning, and resource adapters

## Evidence Required

- schemas 1-current migration and project-isolation fixtures
- Rust shared/donor trace comparison
- stale revision and failure-invariance report
- package and Surface-absence inventory
- removed-versus-retained `nucleus-workspaces` audit
- focused persistence and mutation tests

## Stop Conditions

- Nucleus g05 project-layout validation is not accepted or superseded
- product resources must enter the shared layout document
- project/server state would become layout write authority
- an arbitrary split tree or Surface dependency becomes necessary
- donor worktree changes overlap layout authority files

## Next Task

Execute Card 099. Replace renderer whole-snapshot mutation with checked
per-window state and public Poodle composition.

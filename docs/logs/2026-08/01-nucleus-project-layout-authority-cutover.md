# Nucleus Project Layout Authority Cutover

Date: 2026-08-01
Status: complete
Card: 098

## Outcome

Nucleus commit `fc083647f1bad76a7f544efe0f1644b66c042571`
transfers project layout structure to `longhorn-layout` and
`longhorn-layout-config`. Longhorn commit
`01b9c0a79a8af9214984c29c3969db8a8dc790d3` freezes the accepted donor shape.

The registered schema has five regions:

- `left`
- `center_top`
- `center_bottom`
- `right_top`
- `right_bottom`

It has four sizing slots with the accepted 0.20/0.74/0.74/0.74 defaults.
Project ids derive deterministic container identity. External panel ids derive
deterministic internal instance identity. Tasks is `OnePerContainer`; new
projects seed exactly one Agent Chat panel.

## Authority Split

Longhorn owns definitions, structural state, validation, normalization,
expected-revision mutation, and registered publication. Nucleus owns panel
catalogue policy, project scope mapping, external ids, titles, resource
targets, editor and forge refs, panel bodies, terminal/browser handles, and
cleanup.

Layout, product presentation, and native window placement use separate domains
and files. Layout writes cannot rewrite window placement. Resources and runtime
handles do not enter the shared document.

## Migration And Failure Posture

Raw schemas 1 through 10 convert backup-first. Existing project layouts retain
panels, order, active tabs, regions, and ratios. A former single layout is
claimed once by the first project; later new projects receive the minimal
seed. Interrupted migration resumes from verified backup and published-domain
state without dual-write or silent fallback.

The transition renderer DTO carries layout revision and serializes host
operations. Sizing retains a 200 ms bound. Card 099 removes whole-snapshot
mutation and adopts the checked generated client.

Stale, invalid-kind, and invalid-active requests preserve the exact layout
document and revision.

## Duplicate Audit

Unused `nucleus-workspaces` display, geometry, local-layout, planning,
project-panel, region, and window modules were removed. The retained ids,
layout, and panel modules are server-facing product planning records, not
desktop layout authority. `nucleus-engine` no longer depends on the crate.

No Surface id, type, package, protocol, or state resolves in the selected
Nucleus layout graph.

## Validation

- Nucleus workspace check: pass
- Nucleus desktop library: 55 passed
- Nucleus layout authority: 7 passed
- Nucleus renderer: 40 passed
- Nucleus docs: pass
- Nucleus Svelte check: 0 errors; one pre-existing ProjectRail ARIA warning
- Longhorn layout model: 35 passed
- Longhorn conformance generator/test: pass
- `effigy proof:nucleus-layout-cutover`: pass

Package-manager publication remains deferred.

## Next

Card 099 replaces renderer whole-snapshot mutation with checked per-window
state and public Poodle composition.

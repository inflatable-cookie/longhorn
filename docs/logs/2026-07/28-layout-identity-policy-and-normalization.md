# Layout Identity, Policy, And Normalization

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 023
- added bounded layout ids and checked monotonic revision to `longhorn-core`
- added the pure `longhorn-layout` workspace crate
- registered flat schemas, semantic regions and families, named sizing slots,
  and panel definitions
- made placement, instance count, move, and close policy explicit
- added Surface-independent durable containers, regions, panel instances,
  active state, collapse state, and fixed-point sizing
- added typed validation and deterministic structural normalization
- derived empty-region and transient-reveal visibility without mutation
- made Card 024 the sole ready lane

## Policy And State

Consumers register all product policy. Missing or contradictory policy fails
at registry construction. Durable documents contain only shared ids and
structural state; product payload and runtime handles stay outside the crate.

Panel definitions distinguish singleton, one-per-container, bounded, and
multiple instance policy. Placement can select exact regions or consumer-owned
region families. Sizing uses integer millionths in named slots. The library
does not interpret Poodle composition or pixel geometry.

Valid documents contain complete region and sizing state for each selected
schema. Every panel instance is placed exactly once. Active ids must belong to
their region. Collapse state exists only for collapsible regions.
Normalization orders containers, instances, regions, and sizing slots while
preserving tab order.

## Visibility

Occupied regions are visible. Empty regions follow registered keep-visible or
hide policy. A movable panel can transiently reveal an empty eligible target.
Projection changes no durable byte or revision.

## Evidence

- 18 focused layout tests pass
- registry rejection, ratio bounds, serde, normalization, instance policy,
  visibility, and failure invariance are covered
- Loophole-shaped eight-region and Nucleus-shaped five-region registries use
  the same shared types
- current-toolchain warnings-denied Clippy passes
- Rust 1.85 package tests and workspace all-target checks pass
- full Effigy QA passes
- the only direct Longhorn package dependency is `longhorn-core`

## Boundary

No fixed donor enum, recursive split tree, Surface, window, configuration,
Tauri, TypeScript, Svelte, Poodle, product payload, or donor write entered the
crate.

## Posture

`strict-ready`

Card 023 is complete. Card 024 is ready against the normalized document and
registry contract.

## Next

Review and explicitly start Card 024. Do not add mutation authority from this
closeout.

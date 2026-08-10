# 178 Layout Container Provisioning

Status: superseded by [Card 179](179-surfaces-absorb-containers.md) — 2026-08-10
Owner: Tom
Roadmap: g02 planning checkpoint
Governing refs: contract 014; contract 002
Depends on: none
Auto-start next card: no

## Superseded

Card 179 removes the container concept rather than giving it a lifecycle. Both
halves of the gap below dissolve: creating a Surface creates its layout from a
schema, and closing one removes it, so there is nothing left to provision and
no cleanup intent to execute.

The findings stay recorded because they are what made the case for 179 — in
particular that `CloseSurface` instructed a consumer to perform cleanup the
protocol could not perform.

## Objective

A layout container can be created and removed at runtime. Today neither is
possible, so a Surface can only ever bind a container that was seeded before
the application started.

## Why This Exists

Loophole's "+" add-surface button binds a fixture-seeded spare container,
because there is no other way to obtain one. When the spares run out, the
button stops working. That is the reported symptom.

The gap is symmetric, and the other half is worse.

`LayoutMutationCommand` covers `CreatePanel`, `ClosePanel`, `ActivatePanel`,
`ReorderRegion`, `MovePanel`, `SetSizingSlot` and `SetRegionCollapsed`. Every
one of them operates *inside* a container. Nothing creates one and **nothing
removes one** — `grep` for a container removal across the layout mutation
module returns nothing.

Meanwhile `CloseSurface` returns a `LayoutContainerCleanupIntent`, documented as
"explicit unexecuted cross-domain cleanup work", naming the container that is
now unbound. The Surface protocol instructs a consumer to clean up a container
and the layout protocol offers no way to do it. Every closed Surface leaks its
container into the stored layout document forever.

So this is not a missing convenience. It is a protocol that describes a
lifecycle it cannot complete.

## The Registry Already Has Everything

`LayoutMutationEngine::new` binds a validated `LayoutDefinitionRegistry`, and
`registry.schema(id)` returns a `LayoutSchemaDefinition` carrying complete
`RegionDefinition`s and `SizingSlotDefinition`s — the latter with an explicit
`default: LayoutRatio`.

A container is therefore fully determined by its schema: regions with no
panels, sizing slots at their declared defaults. Creation needs no new evidence
parameter and no new authority, which is why this card is small.

## Steps

1. `CreateContainer { layout_container_id, layout_schema_id }`. Materialise
   regions empty and sizing slots at schema defaults. Reject a duplicate
   container id and an unregistered schema with typed codes.
2. `RemoveContainer { layout_container_id }`. Reject removal of a container
   that still holds panel instances, so the command cannot silently discard
   state; the caller closes panels first.
3. Outcomes and receipts for both, following the existing shapes.
4. Regenerate bindings, extend the TypeScript protocol surface, add the
   commands to the golden fixture so the discriminant-coverage test sees them.
5. Conformance tests, including that a created container validates against the
   registry and that a container with panels refuses removal.

## Decisions To Take In Implementation

**Whether `RemoveContainer` rejects a bound container.** The layout domain
cannot see Surface bindings — the dependency runs the other way, with the
Surface domain taking a `LayoutContainerInventory` as evidence. So layout
cannot check "is this container bound to a Surface". Rejecting on *panels*
present is checkable locally; rejecting on *binding* is not, and would need the
same composition-layer owner that Card 177 identified for the focused-panel
container invariant. Two cards now want that owner.

**Whether surface creation should provision its own container.** Rejected as
the primary shape: it would give the Surface domain a reason to construct
layout state, which is exactly the boundary Card 177 preserved. A consumer
issuing two commands is the honest sequence.

## Acceptance Criteria

- a container can be created from a registered schema and removed
- a removal that would discard panel instances is rejected with a typed code
- `CloseSurface`'s cleanup intent is executable end to end
- `effigy qa` green, including `check:bindings`
- the golden fixture covers both new discriminants

## Notes

Found while relaying Card 177 to the Loophole thread. Worth recording that the
consumer reported the creation half — the half that blocks a visible button —
and the removal half, which silently leaks durable state, had gone unnoticed on
both sides.

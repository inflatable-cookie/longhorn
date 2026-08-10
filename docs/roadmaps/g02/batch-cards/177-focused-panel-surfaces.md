# 177 Focused Panel Surfaces

Status: in progress
Owner: Tom
Roadmap: g02 planning checkpoint
Governing refs: contract 002; contract 014
Depends on: none
Auto-start next card: no

## Objective

A Surface can present one panel full-surface, with no regional layout and no
panel tabs. Loophole holds this app-side today; the concept belongs in the
Surface model.

## Why This Exists

The legacy application had it as a surface `habitat`, and Loophole rebuilt it
by convention: a `focusSurfacePanels` map from surface id to panel definition
id, a derived `focusPanel` that renders full-bleed, and a drop guard that
blocks panels landing on a focus surface's tab. The container is seeded with a
single panel in `center-top` and nothing enforces that it stays that way.

That is three unrelated mechanisms standing in for one missing property, and
none of them survives a second consumer.

## Decisions

### It is a Surface property, not a panel claim

A panel-claimed mode would let any panel declare itself full-surface, which
makes presentation depend on container contents and gives two panels a way to
disagree. The Surface already owns hosting policy, its label and its container
binding; how it presents is the same kind of fact. It also matches how the
consumer uses it — a *dedicated* console surface, decided when the surface is
created, not negotiated at render time.

### Longhorn records the focused panel; it does not police the container

`longhorn-surfaces` depends on `longhorn-core` and serde, and nothing else. It
has no view of container contents: `LayoutContainerInventory` answers exactly
one question, whether a container exists. Validating "this container holds
exactly that panel" from inside the surfaces domain would mean either a
dependency on the layout crate or a second authority for layout membership.

So the surfaces domain validates what it owns — that a focused Surface names a
panel, and a regional one does not — and the container invariant is enforced
where both domains are already visible.

The alternative considered and rejected was widening `LayoutContainerInventory`
to carry panel membership per container. It preserves the crate boundary,
because the evidence is still caller-supplied, and it would work. It is
rejected because it makes every caller of the surface mutation engine assemble
panel membership for every container in order to mutate a label, to serve one
command. The evidence parameter should stay as small as the smallest command
needs.

### No migration, because the field defaults

`SurfaceMigration` is a consumer-supplied hook and `NoSurfaceMigration` is the
default; Longhorn does not own the stored schema version. A new field with a
serde default of `regional` means a document written before this card loads
unchanged, so there is no migration step to write and nothing for a consumer to
register. `NoSurfaceMigration` stays.

### Surface transfer is unaffected; panel transfer is where the guard belongs

The brief asks whether a focused-panel Surface can be a transfer target. In the
session-based *surface* transfer protocol a Surface moves wholesale between
windows, so a focused Surface moves like any other and needs no new rejection.

The interaction the consumer actually guards against is a *panel* being dropped
onto a focus surface, which is `longhorn-transfer`, a different protocol. That
guard needs container-to-surface visibility and is the same composition-layer
question as the container invariant above.

## Steps

1. `SurfacePresentation` in `longhorn-surfaces`: `regional`, or
   `focused_panel` carrying a `PanelDefinitionId`. Defaults to `regional`.
2. `SetSurfacePresentation` mutation command, outcome and receipt, following
   the existing shapes.
3. Validation: a focused Surface names a panel; presentation participates in
   canonical form.
4. Regenerate bindings; extend the TypeScript protocol, client and validation.
5. Conformance tests: the new command, its rejections, and a focus Surface
   round-tripping snapshot to mutation to snapshot.
6. Document the container invariant as a consumer obligation until a
   composition-layer owner exists.

## Acceptance Criteria

- `SurfaceRecord` carries presentation, defaulting to regional
- a document written without the field still loads
- the mutation command sets and clears focus, with typed rejections
- `effigy qa` green, including `check:bindings`
- a focus Surface survives snapshot → mutate → snapshot unchanged

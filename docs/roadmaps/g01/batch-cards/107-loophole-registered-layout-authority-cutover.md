# 107 Loophole Registered Layout Authority Cutover

Status: planned
Owner: Tom
Roadmap: g01.015 batch 3
Governing refs: contracts 002-004, 010, 012, and 014; Card 106
Depends on: Card 106
Auto-start next card: no

## Objective

Move generic region/panel identity, validation, normalization, mutation, and
persistence from Echo/Aura into one registered Longhorn layout authority.

## Repository Scope

- Longhorn: literal donor fixtures and admitted adapter corrections only.
- Loophole: Echo/Aura layout registration, migration, host adapter, tests, and docs.
- Poodle, Pulse product model, and panel bodies: unchanged.

## Scope

- literal current eight-region schema and sizing slots
- product panel catalogue registration and placement policy
- container identity bound externally to Surfaces
- create, close, activate, reorder, move, collapse, and size
- named-layout and project-restore adapter behavior
- explicit old-schema migration and independent layout persistence

## Steps

1. Replace the shape-only Longhorn fixture with literal current donor fixtures.
2. Register regions, families, sizing slots, panel definitions, and limits.
3. Translate current Echo documents through explicit schema migrations.
4. Replace sequential JSON `WorkspaceCommand` application with checked commands.
5. Publish one authoritative snapshot per successful mutation.
6. Retain panel labels, icons, bodies, resources, and runtime cleanup in Aura/Pulse.
7. Remove generic Echo mutation and whole-file best-effort persistence.
8. Prove window/layout concurrency and named-layout/project restore.

## Acceptance Criteria

- all eight regions retain structure, visibility, sizing, collapse, order, and active tab
- focused Surfaces carry no duplicate regional document
- every rejected mutation preserves exact bytes and revision
- persistence failure cannot return an uncommitted snapshot as success
- registry digest changes require explicit migration
- no WindowId, SurfaceId, product payload, Svelte, Poodle, or Tauri enters the core

## Stop Conditions

- the live donor shape contradicts contract 014
- a multi-command product action lacks atomic apply or rollback
- product attachment data would enter the layout document

## Evidence Required

- literal eight-region definitions and old-schema fixtures
- mutation/failure-invariance and persistence traces
- named-layout/project restore and independent-domain proof
- removed/retained Echo authority inventory

## Next Task

Execute Card 108's Surface lifecycle and hosting cutover.

# Loophole Registered Layout Authority Cutover

Date: 2026-08-02
Time: 02:30:16 Europe/London
Card: 107
Roadmap: g01.015

## Result

Loophole now registers `loophole.regional-layout` as an independent Longhorn
configuration domain. One deterministic opaque container represents each
regional Surface. Focused-panel Surfaces have no regional container.

The registered donor shape is literal:

- eight regions in current wire order
- strip, dock, editorial, and detail families
- left, right, and center-bottom collapse and sizing controls
- ten singleton panel definitions with current defaults and allowed regions
- hide-when-empty behavior, tab order, and active tab

Surface ids remain an external binding. Titles, bodies, resources, focused
eligibility, `canFillSurface`, and fallback Surface preferences remain
Loophole product policy.

## Migration And Persistence

First use converts the retained Echo workspace layout into a normalized,
validated Longhorn document. It publishes a source backup before the target
and writes a receipt binding source and target digests. A receipt without its
target stops startup.

Region size, collapse, activation, and reorder commands run through the
Longhorn expected-revision engine. Product attachment and compatibility
commands build a proposal through the retained Echo vocabulary, then publish
only after complete Longhorn normalization and validation. Renderer snapshots
project region state from Longhorn.

The compatibility workspace file remains during Cards 107-108 for topology,
named templates, and product fallback policy. Cross-domain commands serialize
publication. A workspace persistence failure restores the exact prior layout
document. Named-layout switches publish to Longhorn or restore both sources.

## Failure Evidence

Tests prove:

- focused Surfaces produce no duplicate container
- all literal regions, sizing slots, and panels register
- invalid collapse commands preserve exact domain bytes and revision
- named layouts, project layout hints, region state, active tabs, and restart
  round-trip
- the complete Aura native suite remains green

## Retained Echo Inventory

Retained for Card 108:

- window and Surface topology DTOs
- named template storage and metadata
- product panel fallback Surface preferences
- compatibility command decoding and workspace settings projection

No Echo region state drives renderer snapshots after initialization. Generic
Surface lifecycle remains the next authority cutover.

## Validation

- Longhorn literal fixture test: passed
- Aura native suite: 193 passed
- Rust formatting: passed

## Next Task

Execute Card 108. Replace Surface topology, lifecycle, binding, restoration,
and whole-Surface movement while preserving regional and focused habitats.

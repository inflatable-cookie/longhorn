# g01.011 History Kernel And Fork Prototype

Status: ready
Owner: Tom
Updated: 2026-07-30
Governing refs: contracts 001, 003, 007, 008, 010, 012, and 013; research
memo 015

## Outcome

Deliver the proven generic linear history system without reducing Loophole's
successful behavior. Prototype forkable history privately, then promote,
retain, or reject it from measured evidence.

## Generation Runway

This lane advances the optional-system branch after the completed bridge and
command foundations. It supplies history before g01.015 can migrate Loophole.
It does not pull async operations, native islands, or consumer migration
forward.

## Execution Plan

### Batch 1: Pure linear authority

- [x] Card 062: lossless donor fixtures and typed entry/policy foundation
- [ ] Card 063: transactional navigation and failure invariance
- [ ] Card 064: coalescing, grouping, retention, and projections

### Batch 2: Persistence and client composition

- [ ] Card 065: structural persistence and committed transition stream
- [ ] Card 066: generated client, Tauri host, Svelte, and Poodle projection
- [ ] Card 067: two-shape artifact proof and linear closeout

### Batch 3: Private fork prototype

- [ ] Card 068: divergent tree, checkout, pruning, checkpoint, migration, and
  performance prototype

### Batch 4: Promotion decision

- [ ] Card 069: promote, retain, or reject forkable history and close g01.011

## Goals

- [ ] Keep payload types, inverse meaning, and apply logic consumer-owned.
- [ ] Record only successful product mutations.
- [ ] Make undo, redo, and checkout plan/apply/commit transactions.
- [ ] Preserve Loophole's linear record, coalesce, limit, persistence,
  journal, recovery, cross-session undo, and panel capability.
- [ ] Replace renderer-remembered redo with authoritative metadata pages.
- [ ] Prove a materially different non-editor fixture.
- [ ] Keep config, bridge, Tauri, Svelte, Poodle, and branch code removable
  from the pure linear crate.
- [ ] Decide forkable history from a private measured prototype.

## Public Linear Boundary

`longhorn-history` owns structural linear state over a generic typed payload.
Consumer policy supplies inverse, coalesce, and no-op behavior. A consumer
transaction applies the whole navigation batch. History commits only after
success.

Structural persistence, generated metadata projections, committed transition
receipts, a narrow injected Tauri host, per-instance Svelte state, and a
public-Poodle linear panel are in scope. Product payloads never cross the
generic renderer protocol.

Loophole's 83 mutation variants, Pulse apply match, tempo/cache reconciliation,
project format, autosave, journal file policy, recovery choice, versions, and
variants stay in Loophole.

## Private Fork Boundary

The prototype preserves abandoned futures after divergent edits, checks out
through the lowest common ancestor, accounts for checkpoints and replay cost,
and prunes under count and encoded-weight budgets without deleting pinned
lineage.

The prototype is non-publishable. Linear mode remains the only public
compatibility claim until Card 069.

## Acceptance Criteria

- [ ] Loophole-shaped fixtures retain every live mechanic claimed in memo 015.
- [ ] Failed, stale, or partial apply leaves exact model and history state
  unchanged.
- [ ] Compounds and multi-entry checkout require atomic apply or verified
  rollback.
- [ ] Coalescing and grouping use consumer policy and injected time.
- [ ] Structural and payload compatibility fail visibly.
- [ ] Committed transitions can drive a Loophole-shaped journal without
  moving file policy into Longhorn.
- [ ] TypeScript clients expose authoritative past/current/future metadata.
- [ ] The minimal fixture imports no Tauri, Svelte, Poodle, bridge, config, or
  journal package.
- [ ] Fork evidence covers divergent record, preferred redo, branch
  references, checkout, pruning, checkpoints, migration, and realistic
  payload weight.
- [ ] Card 069 records one explicit promotion decision and updates canonical
  architecture, contracts, package topology, and later migration gates.

## Explicit Non-goals

- Loophole repository writes before g01.015
- event sourcing as canonical product state
- project version or collaboration unification
- generic product payloads in TypeScript
- renderer-owned durable history
- silent empty-history fallback
- public branch package before Card 069

## Planning Checkpoint

Card 067 closes the public linear slice and pauses before private branch work.
Card 068 produces evidence only. Card 069 is the mandatory product and package
checkpoint; it cannot auto-promote the prototype.

## Next Task

Execute Card 063. Add revision-bound undo, redo, and entry-id checkout planning
plus checked post-apply commit. Stop if the core must mutate the product model
or stack position must move before product success.

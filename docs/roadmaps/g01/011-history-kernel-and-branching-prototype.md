# g01.011 History Kernel And Fork Prototype

Status: complete
Owner: Tom
Updated: 2026-07-31
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
- [x] Card 063: transactional navigation and failure invariance
- [x] Card 064: coalescing, grouping, retention, and projections

### Batch 2: Persistence and client composition

- [x] Card 065: structural persistence and committed transition stream
- [x] Card 066: generated client, Tauri host, Svelte, and Poodle projection
- [x] Card 067: two-shape artifact proof and linear closeout

### Batch 3: Private fork prototype

- [x] Card 068: divergent tree, checkout, pruning, checkpoint, migration, and
  performance prototype

### Batch 4: Promotion decision

- [x] Card 069: promote forkable history semantics and close g01.011

## Goals

- [x] Keep payload types, inverse meaning, and apply logic consumer-owned.
- [x] Record only successful product mutations.
- [x] Make undo, redo, and checkout plan/apply/commit transactions.
- [x] Preserve Loophole's linear record, coalesce, limit, persistence,
  journal, recovery, cross-session undo, and panel capability.
- [x] Replace renderer-remembered redo with authoritative metadata pages.
- [x] Prove a materially different non-editor fixture.
- [x] Keep config, bridge, Tauri, Svelte, Poodle, and branch code removable
  from the pure linear crate.
- [x] Decide forkable history from a private measured prototype.

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

## Promoted Fork Decision

The prototype preserves abandoned futures after divergent edits, checks out
through the lowest common ancestor, accounts for checkpoints and replay cost,
and prunes under count and encoded-weight budgets without deleting pinned
lineage.

Card 069 selects `Promote`. The accepted semantics move to planned g01.017 as
a separate optional production layer. The prototype stays non-publishable and
outside the root workspace. Linear mode remains the only current public
compatibility claim.

## Acceptance Criteria

- [x] Loophole-shaped fixtures retain every live mechanic claimed in memo 015.
- [x] Stale and verified-rollback failures leave exact model and history
  state unchanged. Rollback failure leaves history exact and reports terminal
  partial-model evidence.
- [x] Compounds and multi-entry checkout require atomic apply or verified
  rollback.
- [x] Coalescing and grouping use consumer policy and injected time.
- [x] Structural and payload compatibility fail visibly.
- [x] Committed transitions can drive a Loophole-shaped journal without
  moving file policy into Longhorn.
- [x] TypeScript clients expose authoritative past/current/future metadata.
- [x] The minimal fixture imports no Tauri, Svelte, Poodle, bridge, config, or
  journal package.
- [x] Fork evidence covers divergent record, preferred redo, branch
  references, checkout, pruning, checkpoints, migration, and realistic
  payload weight.
- [x] Card 069 records one explicit promotion decision and updates canonical
  architecture, contracts, package topology, and later migration gates.

## Explicit Non-goals

- Loophole repository writes before g01.015
- event sourcing as canonical product state
- project version or collaboration unification
- generic product payloads in TypeScript
- renderer-owned durable history
- silent empty-history fallback
- public branch package before Card 069

## Closeout

Cards 062-067 deliver and prove the public linear slice. Card 068 supplies
private measured tree evidence. Card 069 promotes the semantics, retains the
prototype as research, compiles g01.017, and closes this milestone without
publishing branch behavior.

## Next Task

Return to the generation runway. Start g01.012 async-operation and
notification characterization; g01.017 waits for the first linear consumer
and release checkpoint.

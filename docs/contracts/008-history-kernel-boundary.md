# 008 History Kernel Boundary

Status: active research boundary  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/002-shared-desktop-systems-follow-up.md`

## Current Claim

Longhorn may extract a generic history kernel. It does not yet promise
event sourcing, crash recovery, collaboration, or branching.

The first reusable boundary may own:

- stable entry metadata and revisions
- bounded undo/redo navigation
- compounds and explicit gesture groups
- configurable coalescing hooks
- current position and jump planning
- persistence envelopes and migrations
- UI-safe projections

Consumers own mutation payloads, apply/inverse semantics, domain validation,
and labels unless a later contract says otherwise.

## Failure Semantics

- History navigation plans mutations before committing stack position.
- Apply failure cannot leave the stack claiming a state the model did not
  reach.
- Partial compound application is forbidden unless the consumer supplies a
  transaction and rollback contract.
- A new mutation after undo clears redo in the linear kernel.
- Persisted history compatibility is separate from current model-state
  compatibility.

## Branching Gate

A forkable history tree requires a prototype covering:

- branch identity and parent/child topology
- new mutations after undo
- navigation and checkout failure
- pruning, limits, and current-branch behavior
- persisted schema migration
- checkpoints and replay cost
- consumer-specific payload evolution
- UI projection that preserves a simple linear default

Until those pass, branching remains a roadmap research outcome, not a library
feature claim.

## Loophole Admission

The live Pulse stack is evidence for mechanics. Its DAW mutation enum and
runtime apply match remain in Loophole. Extraction requires a consumer-neutral
fixture and a second domain-shaped fixture.

## Acceptance For Linear Kernel

- arbitrary consumer mutation type through a generic adapter
- inverse, compound, coalesce, group, limit, undo, redo, and jump fixtures
- apply failure leaves model and history position consistent
- persisted envelope rejects incompatible future payloads safely
- Loophole fixture retains current behavior
- a non-editor fixture proves the abstraction


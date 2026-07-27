# g01.011 History Kernel And Branching Prototype

Status: research; blocked on contract 008 gates  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 008

## Outcome

Extract only the proven generic linear history mechanics, then decide whether
a forkable persisted tree belongs in Longhorn.

## Batches

### 1. Donor characterization

- freeze Pulse inverse, compound, coalesce, group, limit, jump, and persistence
  fixtures
- separate stack behavior from DAW mutation and runtime apply
- document current redo-clearing behavior

### 2. Generic linear kernel

- consumer-owned payload and apply/inverse adapter
- plan-then-commit navigation
- atomic compound failure semantics
- versioned persisted envelope and UI projection
- second non-editor fixture

### 3. Branch prototype

- branch identity/tree and new mutation after undo
- checkout, pruning, limits, annotations, and current branch
- checkpoint/replay and payload migration experiments
- linear-default UI projection

### 4. Promotion decision

- benchmark realistic depth and persistence
- decide branch API, retain research, or reject
- define recovery relationship separately from ordinary config backups

## Acceptance

- linear kernel meets contract 008 without DAW types
- Loophole behavior does not regress
- apply failure cannot desynchronize model and cursor
- forkable history ships only after branch prototype and promotion


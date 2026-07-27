# g01.014 Nucleus No-Surface Migration

Status: blocked on foundation packages  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 003

## Outcome

Make Nucleus the first simple workspace consumer and prove the full layout
stack does not require Surfaces.

## Batches

### 1. Migration plan

- freeze window/workspace UI behavior and tests
- map project-keyed window containers, five regions, panels, and resources
- name Nucleus-owned policy and dirty-worktree constraints

### 2. Cutover

- adopt config, display/window, layout, and client bindings
- keep project/task/runtime and browser-panel policy in Nucleus
- migrate one coherent vertical slice at a time

### 3. Ownership transfer

- remove superseded duplicated mechanisms
- retain explicit product adapters only
- record behavior deltas and conformance fixtures

## Acceptance

- Nucleus carries no Surface dependency or state
- current window/layout behavior passes
- Longhorn owns the migrated mechanism tests
- Nucleus remains authority for projects, tasks, resources, and native browser
  behavior


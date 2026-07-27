# g01.007 TypeScript, Svelte, Poodle, And App Shell

Status: blocked on `g01.004` through `g01.006`  
Owner: Tom  
Updated: 2026-07-27
Governing refs: contracts 010, 012, and 013

## Outcome

Expose checked client state and thin Svelte/Poodle adapters without copying
components or creating a mandatory application framework.

## Batches

### 1. Client core

- checked snapshots, commands, errors, and current-snapshot handshake
- listener lifetime and teardown
- framework-neutral placement and drag helpers

### 2. Svelte/Poodle bindings

- runes/stores for subscriptions and dispatch
- public `Tabs`, `DockRegion`, and `SplitView` adapters
- compatible-region reveal and exact shared titlebar-drag helper

### 3. Shell guidance

- Poodle theme/presentation bootstrap
- custom titlebar, visible-on-ready, capabilities, and error surfaces
- minimal and full composition examples

## Acceptance

- no Poodle component source is copied
- adapters dispose listeners across mount/unmount and window destruction
- renderer projections cannot create durable fallback truth
- Bovine minimal shell and Loophole full shell remain different compositions

# g01.007 TypeScript, Svelte, Poodle, And App Shell

Status: complete
Owner: Tom
Updated: 2026-07-29
Governing refs: contracts 010-013; research memo 011

## Outcome

Expose checked client state and thin Svelte/Poodle adapters without copying
components or creating a mandatory application framework.

## Batches

### 1. Client foundation

- Card 036: structural client lifetime and domain-free Tauri transport

### 2. Svelte/Poodle bindings

- Card 037: per-window Svelte reactive state and optional subpaths
- Card 038: Poodle public drag seam and exact preview artifact checkpoint
- Card 039: public Tabs, DockRegion, and SplitView layout bindings

### 3. Shell guidance

- Card 040: armed cross-window drag, compatible reveal, and titlebar actions
- Card 041: theme, readiness, capabilities, errors, three-shape proof, and
  closeout

## Acceptance

- no Poodle component source is copied
- adapters dispose listeners across mount/unmount and window destruction
- renderer projections cannot create durable fallback truth
- Surface-free root imports resolve no optional Surface package
- cross-window drag uses a host session armed before native dragstart
- no private Poodle selector, generated id, class, or payload becomes a
  contract
- Split-shell minimal shell and Loophole full shell remain different compositions

## Compilation

Research memo 011 revalidates current Longhorn, Poodle, Loophole, Nucleus, and
Split-shell behavior. It promotes:

- a small domain-neutral client lifetime base
- a raw Tauri transport with no domain dependency
- Surface-free adapter roots and optional capability subpaths
- per-window Svelte state with exact teardown and request-keyed optimism
- public Poodle component bindings without a second durable snapshot
- a named Poodle upstream checkpoint for drag lifecycle and packable preview
- pre-drag host session arming before native DataTransfer writes
- compositional shell guidance and three distinct proof shapes

Cards 036-041 form the dependency-ordered runway. Card 036 is complete with
the pure checked client lifetime, migrated domain clients, and domain-free
Tauri transport. Card 037 is complete with isolated rune state, consumer-fed
layout projection, optional capability subpaths, request-keyed optimism, and
mounted teardown. Card 038 is complete with Poodle's public typed drag seam,
mounted lifecycle coverage, coherent preview package metadata, and exact
artifact set
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.
Card 039 is complete with Surface-free public layout bindings, consumer-owned
presentation snippets, serialized revisioned mutation, and mounted Nucleus and
Loophole shapes. Card 040 is complete with armed panel and Surface drag,
protocol-only payloads, compatible reveal, measured replacement leases,
authoritative commits, and injected titlebar behavior. Card 041 is complete
with isolated artifact installs, minimal Split-shell, Surface-free Nucleus, and
full Loophole shell proofs, public Poodle bootstrap, guarded reveal, visible
failures, narrow capabilities, and package-boundary audits.

## Next Task

Return to the g01 front door. No execution card is ready; resolve the
post-g01.007 intent gate.

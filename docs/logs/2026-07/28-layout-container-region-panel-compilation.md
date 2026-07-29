# Layout Container, Region, And Panel Compilation

Date: 2026-07-28
State: complete planning batch

## Outcome

- revalidated current Loophole and Nucleus layout donors read-only
- promoted the detailed Surface-independent layout boundary into contract 014
- separated flat semantic layout state from consumer Poodle split topology
- added the narrow `longhorn-layout-config` package boundary
- compiled g01.005 into five dependency-ordered cards
- made Card 023 the sole ready lane

## Donor Evidence

Loophole supplies:

- eight semantic regions
- Surface-hosted layout containers
- allowed-region and instance policy
- ordered panels and active tabs
- three resizable/collapsible region controls
- renderer drag with Rust-hosted placement mutation

Nucleus supplies:

- direct window-hosted layout containers
- five semantic regions in activity and workspace families
- distinct panel definition and instance identity
- singleton and multiple-instance panel kinds
- active tabs, close, reorder, cross-region move, and empty-region reveal
- four persisted Poodle split ratios

The donor refresh also confirmed what not to copy: Surface-coupled layout
records, fixed donor enums, renderer-owned whole-snapshot mutation, product
resource attachments inside panel state, and unrevisioned writes.

## Contract Decisions

- shared layout state uses an opaque container and consumer-selected schema
- region schemas are flat; arbitrary split trees remain uncontracted
- named fixed-point sizing slots bridge state into consumer Poodle composition
- visibility derives from occupancy, empty policy, and transient reveal
- panel definitions and instances are separate
- placement and instance policy are explicit and fail closed
- commands carry expected revision and commit one normalized candidate or none
- persistence uses an injected exact configuration descriptor
- layout and window geometry remain separate domains
- Rust owns serialized protocol; checked TypeScript arrives before UI adapters

## Compiled Runway

1. Card 023 — layout identity, policy, normalization, and visibility
2. Card 024 — authoritative expected-revision mutation
3. Card 025 — registered persistence, coordination, debounce, and flush
4. Card 026 — generated TypeScript protocol
5. Card 027 — two-shape conformance and closeout

Card 023 is ready. Later cards remain planned until their named dependencies
close.

## Limits

- no donor repository was modified
- no arbitrary recursive split-tree contract was inferred
- Surface lifecycle and cross-window transfer remain g01.006
- Svelte and Poodle adapters remain g01.007
- consumer ownership transfer remains g01.014 onward

## Validation

- Effigy documentation checks passed
- Effigy Northstar checks passed
- `git diff --check` passed
- Effigy Doctor remained warning-only with zero errors

## Posture

`strict-ready`

## Next

Review and explicitly start Card 023. Do not auto-start layout implementation.

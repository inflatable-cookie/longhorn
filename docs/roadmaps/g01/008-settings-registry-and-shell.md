# g01.008 Settings Registry And Shell

Status: complete
Owner: Tom
Updated: 2026-07-29
Governing refs: contracts 001, 004, 005, 010, 012, and 013; research memo 012

## Outcome

Provide an optional settings authority and shell that composes a one-page
Bovine preference, Soundcheck product and recovery pages, and Loophole custom
pages without importing their product schemas or optional systems.

## Generation Runway

This milestone advances the shared-system foundation from app-shell
composition into centralized configuration UX. It consumes the completed
g01.002 configuration mechanisms and g01.007 client/UI boundaries. It does not
pull bridge, command, history, or consumer-migration work forward.

## Goals

- [x] seal one deterministic registry of modules, sections, pages, and apply
  units
- [x] provide checked one-domain configuration mutation with policy and
  activation projection
- [x] generate framework-neutral TypeScript and narrow Tauri host bindings
- [x] provide per-instance Svelte session state and a public-Poodle shell
- [x] expose storage profile and backup workflows without weakening contract
  004
- [x] expose restore and recovery workflows without weakening contract 004
- [x] prove minimal, product-rich, and advanced custom-page compositions from
  produced artifacts

## Execution Plan

### Batch 1 — authority and configuration transactions

- [x] Card 042: pure settings identity, sealed registry, and authority protocol
- [x] Card 043: config-backed apply units, policy projection, and activation
  receipts

### Batch 2 — checked clients and shell

- [x] Card 044: generated TypeScript protocol and narrow Tauri host
- [x] Card 045: Svelte session lifecycle and public-Poodle shell

### Batch 3 — shared recovery pages and proof

- [x] Card 046: storage profile, diagnostics, and backup settings modules
- [x] Card 047: restore, conflict, and recovery settings flow
- [x] Card 048: artifact-installed composition proof and milestone closeout

## Lane Runway

Cards 042-048 are complete. The isolated proof installs four distinct
compositions from produced artifacts and closes the milestone.

The post-008 checkpoint selected g01.009. Research memo 013 promoted its
bridge/topology boundary. G01.009 is complete. Research memo 014 now compiles
g01.010. Settings may host the optional keybinding page only through the
command settings adapter and composed command/keymap capabilities.

## Deferred

- command-aware keybinding registration and editing
- backend connection pages and remote conflict policy
- server-synchronized settings
- schema-generated product forms
- live module installation and mutable registries
- donor migration
- public package release

## Acceptance Criteria

- [x] Bovine installs settings without layout, Surfaces, commands, or backend
- [x] Soundcheck-shaped product pages coexist with shared recovery pages
- [x] Loophole-shaped hardware and keybinding pages retain consumer authority
- [x] optional modules produce no dead navigation or upward dependency
- [x] invalid, stale, or policy-blocked changes cannot persist
- [x] immediate/staged timing is distinct from activation requirements
- [x] one-domain atomicity is exact and broader transaction limits are visible
- [x] modal, window, and panel hosts use one registry/session contract
- [x] storage, backup, restore, and recovery retain exact g01.002 plans and
  receipts
- [x] packages install from produced artifacts with one Svelte/Poodle runtime
- [x] full Effigy QA and milestone boundary audits pass

## Planning Gaps

Closed by Card 043. The narrow checked mutation seam compares fresh authority,
vetoes, patches, validates, and publishes under one existing coordinator
acquisition.

## Next Task

Continue from the g01 front door. Card 049 is the ready g01.009 entry.

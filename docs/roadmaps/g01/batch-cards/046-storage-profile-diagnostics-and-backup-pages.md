# 046 Storage Profile, Diagnostics, And Backup Pages

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.008 batch 3
Governing refs: contracts 004, 005, 010, 012, and 013
Depends on: Card 045
Auto-start next card: no

## Objective

Expose storage layout, profile transition, and backup operations as optional
shared settings modules without reducing contract-004 authority or receipts.

## Scope

- checked `@longhorn/config` storage-layout and backup protocol/client slice
- injected Tauri config-operation host edge
- storage identity, effective leaf, profile, roots, provenance, warnings, and
  locator state projection
- profile-transition inspect, plan, confirm, execute, recovery, and
  receipt-bound cleanup flow
- backup inventory, capture, publication, export, retention, encryption state,
  and receipts
- optional settings module registration and public-Poodle page adapters
- user-selected root/export target and encryption-provider ports

## Public Behavior

The storage page shows the actual selected profile, effective leaf, resolved
roots, provenance, warnings, and recovery state. Profile changes always use
the existing preview, journal, locator-last commit, recovery, and explicit
cleanup protocol.

Backup shows inspected same-app inventory and exact exclusions. Creation,
export, retention, and encryption outcomes retain their original receipts.
Locked, corrupt, foreign, unknown, or unparseable archives are not relabelled
or deleted.

## Out Of Scope

- restore execution
- secret identity storage
- arbitrary filesystem browsing by the renderer
- new profile ids or retention semantics
- live database copying
- product backup policy

## Steps

1. Generate checked storage-layout, transition, inventory, backup, publication,
   retention, encryption-state, and receipt protocol types.
2. Add framework-neutral clients and compatibility fixtures.
3. Adapt existing operations through injected Tauri handlers.
4. Register storage and backup modules only when their capabilities exist.
5. Build storage diagnostics and profile-transition controller/pages.
6. Build backup inventory, create, export, and retention controller/pages.
7. Inject user-selected target and encryption authorities.
8. Mount success, warning, locked, corrupt, transition-recovery, and failure
   states.
9. Audit filesystem authority, secret exclusion, receipts, package graph, and
   Poodle APIs.

## Acceptance Criteria

- actual profile, leaf, roots, and provenance are visible and exact
- diagnostics expose no secret payload
- profile change cannot bypass inspection, confirmation, journal, or
  locator-last commit
- old layout cleanup requires its committed receipt
- backup captures published state only
- pending debounce requires explicit flush handling
- inventory distinguishes valid, locked, corrupt, foreign, and unknown
- retention cannot delete protected or uninspectable archives
- export uses an injected user-selected target
- encryption identity remains outside ordinary config and renderer state
- absent backup/profile capability creates no page
- root settings package gains no mandatory config dependency

## Evidence Required

- cross-language protocol fixtures
- profile-transition UI-to-receipt matrix
- backup inventory/publication/retention matrix
- locked, corrupt, foreign, partial, and recovery fixtures
- filesystem and secret authority audit
- mounted accessibility and teardown tests
- package, capability, Poodle, and dependency reports
- Rust, TypeScript, Svelte, and Effigy QA

## Stop Conditions

- UI convenience requires weakening confirmation or cleanup receipts
- renderer needs ambient filesystem access
- an encryption secret would enter settings snapshots
- a live database would be treated as ordinary files
- shared pages require product retention policy

## Next Task

Card 047 is ready. Add restore and recovery as a separate destructive flow.

## Result

`longhorn-config` now owns a renderer-safe config-operations protocol and exact
projections over its existing storage transition, backup publication, and
retention mechanisms. The wire excludes filesystem capabilities, executable
plans, archive payloads, retention policy, and encryption identities.
Non-UTF-8 paths fail projection instead of becoming lossy evidence.

`@longhorn/config` adds checked generation, fixtures, compatibility guards, and
a framework-neutral client. Its optional `/poodle` subpath supplies storage
and backup pages. Storage shows exact identity, leaf, profile, roots,
provenance, warnings, locator/recovery state, inspected transitions, committed
receipts, and receipt-bound cleanup. Backup shows valid and preserved
inventory, pending publication, encryption availability, publication/export
receipts, and confirmation-bound retention.

`longhorn-tauri-config` now provides eight narrow commands over one injected
authority. Portable-root and export selection plus redacted encryption status
are explicit host ports. `longhorn-settings-config` registers storage and
backup pages independently by base capability and adds no dependency to the
settings root.

Focused Rust, TypeScript, Svelte, mounted, SSR, package, capability, fixture,
and authority evidence passes.

Evidence:
`../../../logs/2026-07/29-storage-profile-diagnostics-and-backup-pages.md`.

# 116 Soundcheck Settings, Backup, And Recovery Cutover

Status: planned
Owner: Tom
Roadmap: g01.016 batch 2
Governing refs: contracts 003-005, 010, 012-013; Card 115
Depends on: Card 115
Auto-start next card: no

## Objective

Replace Soundcheck's central dialog mechanism with the shared settings
registry and shell while retaining product pages and soundcheck-library backup
semantics.

## Repository Scope

- Longhorn: shared config/settings adapters, fixtures, evidence, and docs.
- Soundcheck: registry composition, product page adapters, renderer shell,
  Tauri capabilities, tests, and docs.
- soundcheck-library: read-only plus existing injected backup adapter use.

## Scope

- modal settings registry and one per-instance Svelte/Poodle shell
- Agent Review, Custom Tags, Vendors, Composer, and Keepsake product pages
- shared storage diagnostics, backups, restore, conflicts, and recovery pages
- one-domain apply units and explicit immediate/staged policy
- SQLite native backup inventory and restore through a capability adapter
- library-restored invalidation without renderer backup authority

## Steps

1. Register stable Soundcheck page, scope, renderer, capability, and apply identities.
2. Bind app settings to checked one-domain apply units.
3. Keep product page bodies and validation in Soundcheck.
4. Compose shared storage, backup, restore, and recovery modules only when capable.
5. Adapt soundcheck-library native backup/restore receipts without changing semantics.
6. Replace the bespoke navigation/controller mechanism with one settings session.
7. Preserve deep links, busy/conflict state, restore confirmation, and invalidation.
8. Remove the superseded generic dialog registry/controller copy.

## Acceptance Criteria

- one sealed registry and one modal shell own structural settings state
- product schemas, page bodies, wording, and integrations stay Soundcheck-owned
- SQLite backup ids, retention, validation, restore, and migration remain sibling authority
- restore uses fresh confirmation-bound evidence and visible recovery state
- unavailable modules render no empty navigation
- renderer teardown cannot lose an accepted settings mutation
- only admitted Tauri read/mutate capabilities resolve
- no private Poodle seam or duplicate settings authority remains

## Stop Conditions

- the generic shell must interpret a product schema
- a library restore loses its native validation or rollback contract
- product and shared pages require separate structural registries
- a private Poodle API is needed

## Next Task

Execute Card 117. Adopt Longhorn operation authority for plugin scan while
keeping scan execution and reports in soundcheck-library.

# 105 Loophole Storage And Domain Transition

Status: planned
Owner: Tom
Roadmap: g01.015 batch 2
Governing refs: contracts 003-005 and 012; Cards 102-104
Depends on: Card 104
Auto-start next card: no

## Objective

Replace `echo-os-paths` and copy-if-missing startup import with the selected
Longhorn profile, registered stores, backup participation, and receipted
transition.

## Repository Scope

- Longhorn: only fixture or adapter fixes proven necessary by the admitted graph.
- Loophole: Aura/Echo/Pulse path composition, registered domains, migration,
  diagnostics, tests, and docs.

## Steps

1. Register canonical id, stable storage name, profile, and fixed locator.
2. Inventory current Echo roots, the old Tauri id root, profiles, settings,
   machine state, caches, databases, and unknown files.
3. Classify app/profile/machine/server/project/cache/state authority.
4. Use ordinary and native database adapters; stage and verify every durable store.
5. Commit the locator last and expose recovery before normal mutation.
6. Retain source roots and bind cleanup to the exact receipt.
7. Replace renderer `localStorage` authority with registered configuration.
8. Prove embedded, local-Pulse, and remote-Pulse path ownership.

## Acceptance Criteria

- every Loophole process resolves the same local product root when appropriate
- project files and journals remain project-owned
- config, cache, state, logs, backups, runtime, and databases use correct classes
- interrupted migration recovers to one authority
- old roots are never silently read or dual-written
- diagnostics expose identity, profile, roots, provenance, and recovery state

## Stop Conditions

- one store has multiple undeclared writers
- a live SQLite/WAL pair would be copied as ordinary files
- an unknown source must be deleted or silently merged
- remote server data would become client config

## Evidence Required

- complete source/target inventory and conflict plan
- transition journal, locator-last commit, recovery, and cleanup receipts
- SQLite/native adapter and backup/restore proof
- embedded, local-server, remote-server, and interrupted-start traces

## Next Task

Execute Card 106's display and native-window cutover.

# 105 Loophole Storage And Domain Transition

Status: complete
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

## Result

Loophole now owns one `echo-storage-profile` adapter over Longhorn. Aura,
embedded Pulse, the local brokered Pulse host, Spark, and profile config use
the same canonical id, stable `Loophole` leaf, locator, and selected layout.
Remote Pulse keeps server and project storage outside the client profile.

Aura replaces copy-if-missing import with an inspected, planned, adapter-backed
transition. Existing `Loophole` roots adopt in place. The old Tauri id root is
eligible only when the established product root has no recognized durable
domain. Sources and unknown files remain retained. JSON files and profile trees
are staged, validated, verified, and receipted. Soundcheck's SQLite database
remains external authority; no Loophole-owned live database enters this card.

App settings, profiles, recent projects, plugin presets, hosting policy, and
plugin preferences are user config. Windowing, hardware settings, input
bindings, and plugin-editor reopen state are machine state. Plugin catalogue
projection is excluded rebuildable cache. Recording recovery is excluded
incomplete state. Project documents, autosaves, journals, and media remain
project/Pulse authority.

Five renderer preference keys now import once from WebView storage into one
registered Longhorn config domain. Production mounts wait for host hydration.
There is no post-import local read or dual-write path.

Longhorn now admits same-layout locator adoption when a derived workspace root
nests beneath state. The regression proof prevents false overlap conflicts
without weakening transitions between different layouts.

Evidence: `../../../logs/2026-08/02-012529-loophole-storage-and-domain-transition.md`.

## Next Task

Execute Card 106's display and native-window cutover.

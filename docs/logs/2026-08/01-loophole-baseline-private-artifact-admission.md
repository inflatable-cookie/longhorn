# Loophole Baseline And Private Artifact Admission

Date: 2026-08-01
Card: 104
Outcome: `pass`

## Baseline Repair

The pinned Loophole baseline reproduced the two-`libsqlite3-sys` resolver
failure. Pulse declared `rusqlite 0.31` but had no direct call site. Its live
database path is owned by Soundcheck Library, which requires `rusqlite 0.40.1`.

Loophole removes the unused direct dependency. Pulse and Spark now resolve one
`rusqlite 0.40.1` / `libsqlite3-sys 0.38.1` graph. No schema, migration,
backup, restore, or product-data meaning changed.

Clean repaired Loophole commit:
`4b9835ef328f13a761fe2037c45aecfb48314d21`.

Evidence:

- Soundcheck SQLite migration, backup, restore, and readback suite: 94 passed
- Loophole live Soundcheck catalogue database sync: 1 passed
- `effigy health`: passed
- `effigy test`: Aura 1,031; Echo 414; Pulse full suite; Spark 11
- `effigy validate`: passed

## Exact Sources

| Source | Receipt |
| --- | --- |
| Longhorn selected package tree | `79d86dc1a29de29c553c5559c091d76c3daa0cfe5386a39a2ef20f51f65be171` |
| Loophole | `4b9835ef328f13a761fe2037c45aecfb48314d21` |
| Loophole Pulse lock | `d2ec071cca416c5151ca64856fc276e8cf1272376a74f97af6b17334a186cfc8` |
| Loophole Spark lock | `510eb08aa0bec74bd1a6742db58f295a99ff4083975e1c38bc423ba3e02819cd` |
| Poodle | `208532f0d18dcd1683cdef157e370d0ba0f0d3b3` |
| Poodle artifact set | `25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69` |
| Signal | `4c5b60681071095aedbf9013720e76c1c89e91ed` |
| Soundcheck Library selected Rust tree | `b014663ca03c21d8cd2e82da35cbd7ef8086e7808b3cfde077fba75243249a92` |

Soundcheck Library had unrelated renderer work under
`packages/library-svelte`. The selected core, jobs, SQLite, store, composer,
scan, workspace manifest, and lock were clean and remained read-only.

## Renderer Artifacts

`effigy proof:loophole-private-artifacts` packs and installs the exact
Cards 105-111 renderer graph outside both workspaces.

| Longhorn package | SHA-256 |
| --- | --- |
| `@inflatable-cookie/longhorn-core` | `b41c3585e48f8e538acaccc68209660e7be55962bdea0af443af924011fcc9f0` |
| `@inflatable-cookie/longhorn-config` | `3be4f0ca3153dd8bcaa408d7393178ee2bfebfa026722c4e5ac29b13e8b79367` |
| `@inflatable-cookie/longhorn-layout` | `f3ebecc3f6abab4404be23eb73a64c6db8185421754e7c5644754f997d8f1fa5` |
| `@inflatable-cookie/longhorn-surfaces` | `449cc0b1751860912cc3468ca41f1be3e4987b513e306988cad05a27fd27d088` |
| `@inflatable-cookie/longhorn-transfer` | `37b50416c2b81531435a320e38138342d923f0b2a90b122cb13f4e18c3f0d7cf` |
| `@inflatable-cookie/longhorn-surface-transfer` | `2c3803d47892142a213c7388601721bbb3f281efa1276c9dbc369d58ba8b3d5b` |
| `@inflatable-cookie/longhorn-settings` | `d2b7b78122097705a95dafc48ab37fef52d3d5211268351e5263e059bf24997b` |
| `@inflatable-cookie/longhorn-commands` | `87abb5c662ac332e6faeb6f3970ed61017869f4d1277058d66b88e3d2d095207` |
| `@inflatable-cookie/longhorn-history` | `a73813a2c43bdcb45934c85bbe15c447db5af92e8abb1a3cc010dfb0aae73333` |
| `@inflatable-cookie/longhorn-tauri` | `4df629c1bc5ebd889bdb29e100956f0a2327fc4fc7cee38843b45a086f7fa3c5` |
| `@inflatable-cookie/longhorn-svelte` | `024dd873ff3d99e99755d76046076476783f60228b2bf1f2ecef7439aae83ca3` |
| `@inflatable-cookie/longhorn-poodle` | `e68d818335a7abf9dcd8c66dedc0bd40a5a5969dbf91b660085e195746963d75` |

The clean install uses one Svelte 5.56.8 runtime, one public Poodle artifact
set, TypeScript 6.0.3, and Tauri API 2.11.1. Both mounted eight-region
Loophole shell tests pass. No sibling source alias resolves.

## Rust Artifacts

The proof inventories and privately archives 24 selected crates. An isolated
Loophole-shaped consumer compiles offline on Rust 1.85 with Tauri 2.11.5 and
lock digest
`c79835ff7fc9c91e2c19be78a9f9d05bf65240dd27797e19b206b47ad6a4291e`.

The graph includes storage, display/window hosting, layout, Surfaces,
Surface-window composition, panel and Surface transfer, settings, commands,
and linear history. Generated config, layout, Surface, transfer,
Surface-transfer, settings, command, and history bindings are current.

Fork-tree packages are absent. Native-content, bridge, operation, and
notification packages are also absent because Cards 105-111 do not require
them. Private Rust archive hashes remain run-local evidence; the selector
emits each value and ties it to the selected source-tree receipt.

## Admission

Only Card 105's storage and registered-domain transition is admitted for donor
writes. Longhorn and Poodle remain unpublished. Registry ownership and public
compatibility ranges stay in the later release lane.

## Next Task

Execute Card 105's storage and registered-domain transition.

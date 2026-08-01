# Nucleus Migration Conformance Closeout

Date: 2026-08-01
Status: complete
Roadmap: g01.014
Card: 101

## Outcome

Nucleus is the first complete Surface-free Longhorn consumer. Its hierarchy is
`display -> window -> region -> panel`.

Exact source receipt:

- Longhorn selected source: `4f09e1c8a9b55965605f74dfc364c1bb8a836570`
- Poodle: `208532f0d18dcd1683cdef157e370d0ba0f0d3b3`
- Poodle artifact set: `25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69`
- Nucleus: `40a8314d067a3e33eb2254499e53a6db5a8962d5`

`effigy proof:nucleus-migration-closeout` packs the selected private Longhorn
graph, installs it with the exact Poodle artifacts outside all workspaces, and
proves one Svelte/Poodle runtime. It replays Cards 096-100 and runs the current
fresh, legacy, and interrupted migration cases.

Card 095's original live proof remains pinned to its historical donor
checkouts and is no longer part of aggregate QA. Its task and receipt remain
available for that exact checkout. Card 101 owns the current donor-head and
artifact rebuild invariant.

## Authority

Longhorn owns storage profile and transition mechanics, display/window
mechanics, registered layout mechanics, checked renderer reconciliation, and
native-content coordination.

Nucleus retains app identity, projects, tasks, resources, panel catalogue,
project presentation, Browser policy, Tauri composition, and product errors.
Poodle retains visual Surface primitives and overlay geometry events.

No active Nucleus Surface package, type, state, command, raw renderer Webview,
or superseded storage/window/layout/child coordinator remains. `LegacySurface`
and `active_surface_id` remain only in the old-layout decoder and its negative
guard. Poodle's use of “surface” describes visual overlay geometry, not
Longhorn hosting.

## Restart And Rollback

Fresh portable startup resolves the same layout after restart. Legacy startup
commits the fixed locator last, reuses the exact receipt after restart, and
retains `.nucleus` and unknown files.

The previous Nucleus authority build at
`74ca4e7c72f447e064419de6dc72502265cbbf49` read the same isolated database as
the current build. Both returned evidence digest
`6b93c6debc022ebb2b27011562ec1b56889b5521c7d947e9290afc8955d434c1`.
The database digest remained
`649058332e9cbf7bf06667479f4608dbe8c40ad910dbb702f8763b028f07b383`.
No dual-write or fallback was enabled. The temporary worktree and profile were
removed.

## Cleanup Gate

Legacy source deletion was not authorized and did not run. Later cleanup must
match the exact committed import receipt. Unknown files remain retained.

## Validation

- `effigy proof:nucleus-migration-closeout`: pass
- Nucleus `effigy qa`: pass; one pre-existing ProjectRail ARIA warning
- Longhorn `effigy qa`: pass
- package-manager publication: false

## Next

Compile g01.015 from a fresh read-only Loophole audit. Do not assume the
Surface-free Nucleus adapters fit Loophole's full hierarchy unchanged.

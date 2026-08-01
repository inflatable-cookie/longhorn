# Nucleus Behavior, Authority, And Rollback Freeze

Date: 2026-08-01
Roadmap: g01.014
Card: 094

## Result

Froze current Nucleus desktop behavior in a checked Longhorn fixture. The
verifier pins clean Nucleus `main` at
`c084d57ca15f9e4276f49a9b6b2923f5d10e7313` and clean Poodle `main` at
`c6ef2a0fdc162092cdbcff092a121aedf5323bcb`. Poodle advanced once during the
audit through an unrelated clean Rust-bootstrap commit; the audited Svelte
files did not change. This card wrote to neither donor.

Outcome: `pass_with_admission_gates`. Card 094 is complete. Donor-write
admission remains false.

## Frozen Shape

| Slice | Preserved behavior | Target authority |
| --- | --- | --- |
| storage | `~/.nucleus`, explicit proof override, combined `ui.json`, SQLite, snapshots, drafts | Longhorn mechanisms; Nucleus identity, domains, retention |
| window | hidden protected main window, physical outer bounds, 300 ms capture, one-second close flush, deterministic display fallback | Longhorn window host; Nucleus main-window policy |
| layout | `display -> window -> region -> panel`, five literal regions, four sizing slots, project-keyed complete documents, no Surface | Longhorn layout mechanism; Nucleus schema and panel policy |
| Browser | HTTP/HTTPS, HTTPS normalization, denied popups/downloads, normal shared engine store, no remote Tauri capability, hide/reuse/destroy lifecycle | Longhorn native-content mechanism; Nucleus browser policy |

The existing Longhorn Nucleus conformance fixture matches the no-Surface
five-region/four-slot shape. Its region names are older compatibility names,
not the current donor literals. Card 098 must freeze the literal current schema
rather than treating the old names as authority.

## Storage Classification

Task-review snapshots are workspace-local evidence, not cache. Nucleus keeps
immutable source snapshots resolvable while a work item is active or awaiting
review, then starts a seven-day cleanup grace. Card 096 must migrate the live
tree and retention records through a Nucleus adapter. Normal backup excludes
the store so expired review detail is not resurrected.

SQLite uses native snapshot transition. Window placement becomes machine
state. Project layouts and editor drafts are workspace-local. UI preferences
are user config. The fixed canonical-id locator commits last, the legacy source
remains, and no slice uses dual-write or silent fallback.

## Public Poodle Gate

Poodle publicly exposes open-state callbacks plus `anchored` and `portal` for
consumer-built overlays. Built-in Popover and Menu surfaces keep their DOM
elements private and expose no geometry callback. Nucleus currently finds them
through `.poodle-popover__surface, [role="menu"]` and hides a native Browser
only when an overlay intersects its viewport.

Global hide-on-any-overlay would change behavior. The private selector cannot
become a Longhorn contract. Exact preservation requires a Poodle-owned,
runtime-neutral built-in overlay geometry seam and a compatible release.

Post-freeze inspection found a stronger failure. Nucleus queries below the
component's former DOM ancestor, but Poodle contract 002 portals the anchored
surface to the theme root. The selector cannot reach the live surface after
portalling. Treat this as a latent donor defect. Preserve the intended
per-viewport intersection policy through public snapshots; do not preserve the
broken lookup or replace it with global hide-on-open.

Poodle later advanced through g12.018 at
`ef41f412ad7b45c2ee760c1da9bf41ef876855e8`. The freeze verifier now reads
donor files from the recorded commits rather than requiring donor HEAD to stay
there forever. It still requires clean donor worktrees.

## Admission Gates

- Poodle public built-in overlay geometry contract
- Nucleus g05 project-layout operator acceptance or explicit supersession
- published Longhorn and compatible Poodle prereleases

Card 095 stays planned. Nucleus remains read-only. The Poodle source/API gate
is resolved; compatible publication remains open.

## Evidence

- `fixtures/migration/nucleus-card094/behavior-freeze-v1.json`
- `scripts/verify-nucleus-card094.ts`
- `docs/architecture/nucleus-migration-map.md`
- `effigy qa:northstar:g01-nucleus-card094-freeze`

## Next

Define the upstream Poodle overlay-geometry contract and reconcile Nucleus g05
operator acceptance. Then review Card 095 publication readiness.

# Secondary-consumer Migration Map

Status: promoted
Owner: Tom
Updated: 2026-08-02
Contracts: 003-007, 009-010, 012-017
Roadmap: `../roadmaps/g01/016-secondary-consumers-and-greenfield-release.md`

## Boundary

g01.016 proves Longhorn in three materially different desktop apps and four
greenfield compositions. It does not turn product workflows into shared
packages and does not publish packages to registries.

Consumer repositories remain authoritative until their named cutover cards
pass. Source-linked development graphs require exact clean Longhorn and Poodle
receipts plus isolated produced-artifact proof.

## Recorded Sources

| Repository | Recorded commit | Worktree posture | Use |
| --- | --- | --- | --- |
| Longhorn | `9c7221ce1f36ddc674dad2a52c3b8e0cb78c4ce7` | clean | shared source and compilation authority |
| Soundcheck | `f4544e621b8ba3f4558c6b068db1cb36d6ef161f` | clean | desktop consumer |
| soundcheck-library | `40418635674d598f7e311bf311c3165300edd35b` | clean | shared plugin-library and SQLite authority |
| Signal | `4c5b60681071095aedbf9013720e76c1c89e91ed` | clean | plugin ABI, discovery, audio, and runtime authority |
| Bovine Accelerator Desktop | `b713644e08be384d20879b0ac31f7899589c8a9b` | unrelated docs work present | small local consumer; no writes before overlap clearance |
| Jetstream | `4df74e756c222a3b207391c44775e5b3148c46dd` | clean | editor and backing-surface consumer |
| Poodle | `208532f0d18dcd1683cdef157e370d0ba0f0d3b3` | clean | visual primitives and exact private artifact set |

These receipts are compilation evidence. Card 113 refreshes them before
admission; no migration may rely on a stale or dirty selected source.

## Card 113 Checked Freeze

Card 113 passed against Longhorn compilation commit
`efe3483d499b5416cd6f1690d1c4598fd75cdfa4` and the exact consumer commits
above. Soundcheck, soundcheck-library, Signal, Jetstream, and Poodle are clean
on `main`. Bovine remains on its recorded commit with exactly 19 unrelated
`docs/` or `CHANGELOG.md` paths; no runtime or package path overlaps.

The checked freeze covers 23 source matrices and 27 unique selected Longhorn
package paths. Every selected package exists, none changed after compilation,
and no consumer graph contains a forbidden system. The prior exact Poodle
artifact-set receipt is retained for comparison; Card 114 must produce a fresh
set before consumer writes.

Rollback requires retained consumer sources, exact source/artifact receipts,
and previous locks. Dual writes, silent fallback, automatic source cleanup,
package-manager publication, tags, and hosted releases remain forbidden.

## Soundcheck

Soundcheck is a single primary-window app with a stable human-readable product
storage root, a product settings dialog, reconnectable scan work, coordinated
SQLite backup, and disposable native plugin-inspection helpers.

### Selected shared composition

- `shared-product-root-v1`, canonical id
  `com.inflatablecookie.soundcheck`, stable storage name `Soundcheck`
- configuration domains, storage diagnostics, backup/restore coordination,
  and the protected single-window host
- settings registry and modal Poodle shell
- operation authority for the plugin-scan lifecycle
- isolated-window native-content coordination for plugin inspection

### Retained Soundcheck and sibling authority

- `soundcheck-library`: inventory, scan execution, same-active policy, reports,
  migrations, SQLite schema, native snapshots, and library backup semantics
- Signal: plugin formats, ABI, discovery, audio/MIDI, editor construction, and
  native processing
- Soundcheck: product pages, validation, Composer/Keepsake/DAW workflows,
  inspection authorization, helper launch, disposable-process policy, native
  header, screenshots, plugin resize acceptance, and user-facing wording

The SQLite database is an external data-class participant. Longhorn may
coordinate its native snapshot adapter but may not reinterpret its schema,
move it independently of the selected storage transition, or replace its
recovery rules.

## Bovine Accelerator Desktop

Bovine is the minimal no-service, no-Surface case. It currently stores one
unversioned `workspace.json` below Tauri `app_config_dir`; that document mixes
the selected content root with navigation ratio, expanded nodes, and selected
node. The renderer uses public Poodle controls through sibling source aliases.

### Selected shared composition

- `native-platform-v1`, canonical id
  `com.acowtancy.bovine-accelerator`, no stable-name override
- one registered configuration domain with safe mutation, explicit flush, and
  receipt-backed legacy import
- a minimal modal settings shell only for admitted shared and product pages
- public Poodle artifacts and framework-neutral Longhorn clients

### Retained Bovine authority

- repository parsing, content hierarchy, search, editorial, validation, and Git
- authored workspace content and every rule governing its mutation
- selected content, expanded-node, and navigation meaning
- the local two-pane presentation unless a later product need justifies a
  registered layout document

The current SplitView alone does not justify a Longhorn layout dependency.
g01.016 proves the smaller graph and keeps layout, Surfaces, history, commands,
backend topology, and native content absent.

## Jetstream

Jetstream is a one-window editor with one whole-state publication edge, a
small hard-coded keyboard table, product command dispatch, and an engine-owned
WGPU `NSView` beneath a transparent webview.

### Selected shared composition

- typed bridge session and listener-before-snapshot state projection
- command registry, availability, keyboard resolution, and checked renderer
  projection
- backing-surface desired/observed coordination and Svelte viewport lifetime

### Retained Jetstream authority

- command ids, product availability, execution, undo/save meaning, and world
  mutation
- editor snapshot payload, renderer, WGPU device/surface construction, native
  view storage, frame loop, scene, world, camera, picking, gizmos, and logs
- semantic viewport input mapping and every product-specific Tauri command
- outer-window and transparent-webview policy

Longhorn command hosting does not introduce an execute-by-string Tauri bus.
The consumer executor maps admitted command identity to Jetstream's existing
typed product route. Native-content coordination never owns WGPU objects or
semantic input.

## Greenfield Matrix

| Shape | Required systems | Forbidden accidental edges |
| --- | --- | --- |
| minimal | core, config, Tauri config, Svelte shell | layout, Surfaces, service, history, native content |
| workspace | minimal plus windowing, layout, settings, commands | Surfaces and service unless selected |
| full hosting | windowing, layout, Surfaces, transfer, settings, commands, linear history | fork tree and product payloads |
| optional server | minimal or workspace plus bridge and injected supervision | service authority in local domains |

Examples install produced artifacts outside workspace resolution. They are
composition proofs and starting references, not a generated product framework
or an umbrella package.

## Private Release Boundary

g01.016 may produce a coordinated `0.1.0` private candidate, compatibility
matrix, upgrade notes, API docs, deterministic package inventories, checksums,
and clean consumer installs. It may run read-only release gates.

It may not:

- claim registry ownership for working package names
- publish npm or Cargo packages
- create a release tag or hosted release
- widen Poodle, Svelte, Tauri, platform, or MSRV support beyond evidence
- describe unsupported Windows/Linux native-content hosts as portable

Registry ownership, public package names, and publication require a later
operator-authorized release action.

## Ordered Adoption

1. refresh read-only behavior and authority receipts
2. prove the exact private artifact graph for all three consumers
3. migrate and close Soundcheck
4. migrate and close Bovine without touching unrelated work
5. migrate and close Jetstream
6. build the greenfield matrix and public-facing usage guides
7. prove one private compatibility candidate and close g01.016

Each cutover has one structural authority, a focused rollback route, retained
source evidence, and a duplicate-mechanism audit. No card uses dual writes or
silent fallback as rollback.

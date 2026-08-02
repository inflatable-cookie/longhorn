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
| Bovine Accelerator Desktop | `8b3c19e8d1e96ce81d1dbbaf4769c9e72648dd28` | unrelated docs work preserved | minimal config/settings consumer; Card 121 closed |
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
artifact-set receipt was retained for comparison.

Rollback requires retained consumer sources, exact source/artifact receipts,
and previous locks. Dual writes, silent fallback, automatic source cleanup,
package-manager publication, tags, and hosted releases remain forbidden.

## Card 114 Private Artifact Admission

Card 114 freshly reproduced the exact Poodle artifact set and packed the nine
selected Longhorn TypeScript packages. Three isolated renderer installs resolve
one Svelte 5.56.8 runtime, Tauri API 2.11.1, and no original workspace or
sibling source.

Eighteen selected Rust crates were inventoried and archived from Longhorn
`ec465b2a86fe6fbaef789b3677a8e7288e7df2d2`; three dev-only support crates
enable isolated workspace checking but enter no consumer's normal graph. The
Soundcheck, Bovine, and Jetstream graphs pass locked and offline on Rust 1.85.0
with Tauri 2.11.5. Native-content isolated-window and backing-surface packages
remain independently selected.

The exact aggregate set ids are:

- Poodle: `25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69`
- Longhorn TypeScript: `7f62a7d21889c778803cda687248a9048e497cee80d2c2ceb7fa6957e18b3ce0`
- Longhorn Rust: `42a1a400a7a6066614273a44d86c1686991c92c5454594efe1000aa1730d65c8`

Cards 115-116 complete Soundcheck storage, protected-window, settings, backup,
and restore structure. Card 117 moved plugin-scan lifecycle structure onto
Longhorn while retaining soundcheck-library job truth. Card 118 wraps the
disposable inspection helper in isolated-window coordination while retaining
Soundcheck and Signal policy. Card 119 closes the exact composition, artifact,
restart, and rollback gates. Card 120 cuts over Bovine's minimal
config/settings graph. Card 121 closes exact conformance and optional-edge
audits. Jetstream remains read-only.

## Card 115 Soundcheck Cutover

Soundcheck selects `shared-product-root-v1` with canonical locator identity
`com.inflatablecookie.soundcheck` and stable leaf `Soundcheck`. The default
product roots remain `Application Support/Soundcheck`, `%APPDATA%\Soundcheck`,
and `$XDG_DATA_HOME/Soundcheck`. `library.db` stays directly below that root;
an explicit Data override records the retained soundcheck-library placement.
Other typed roots use `config/`, `state/`, `cache/`, `logs/`, `runtime/`, and
`backups/` children.

`soundcheck.app-settings` is UserConfig and
`soundcheck.window-placement` is MachineState. Legacy `settings.json` import
backs up exact bytes before publishing both domains, records their digests,
refuses conflicts, publishes the locator last, retains the source, and grants
no cleanup authority. Acceptance and assistant overrides are rejected unless
an explicit proof root contains every injected path.

The external `soundcheck.library-database` participant uses SQLite online
backup and failure-atomic restore. Longhorn coordinates evidence only;
soundcheck-library still owns schema, meaning, migration, validation, and
backup policy. Same-layout adoption now skips unrelated retained source data,
preventing a large product tree from becoming transition input.

The Tauri `main` label maps to protected `window:primary`. Soundcheck uses a
hidden restore, known-display reconciliation, `320x240` minimum, guarded
restore/page-ready reveal, 300 ms capture debounce, and two-second close flush.
The previous settings-file window worker is removed. Card 119 proves native
bundle startup, first-attempt hidden restore, and previous-build readback. The
visible GUI click path is explicitly environment-unmet on this host.

## Card 116 Soundcheck Settings And Recovery Cutover

One sealed registry and one modal `SettingsShell` now own settings structure.
Soundcheck retains the five product renderers and Agent Review schema. Its
model/reasoning pair publishes through one immediate
`soundcheck:app.apply` unit; direct renderer save coordination and the direct
Tauri mutation command are gone.

The shared Storage, Backups, and Restore pages are admitted for exactly four
capabilities: storage diagnostics, backup inventory, restore inspection, and
custom-adapter execution. Storage transitions, backup create/export/retention,
ordinary restore, and recovery remain absent instead of rendering empty
actions.

`soundcheck-library` still owns automatic backup ids, ordering, retention,
SQLite validation, safety backup, schema migration, restore, and rollback.
Longhorn projects exact inventory and coordinates confirmation only. Restore
inspection binds the native backup id, exact archive hash, and a fresh online
snapshot hash of the current database. Execution rechecks that evidence before
calling the sibling restore path and reports verified, rolled-back, or
recovery-required terminal state. The renderer receives no filesystem or
direct backup command.

## Card 118 Soundcheck Isolated-window Cutover

Each plugin inspection launch now maps to one
`island:soundcheck-plugin-inspection:<request>` identity, one logical host id,
and one attach generation. Longhorn installs the runtime listener before
Soundcheck starts the helper. Ready evidence contains product-free size,
visibility, focus, owner pid, and native attachment. The plugin's first editor
size remains authoritative; a differing bootstrap size is a partial apply, not
a forced resize or false receipt.

Signal accepts plugin and user resize requests before projection. The helper
retains the original request and accepted size in a bounded state channel.
Longhorn records exact or constrained decisions, desired updates, applies, and
fresh observations. Foreign generations and request-history gaps fail closed.
Normal close still exits the disposable helper immediately; bounded owner
termination is the only detach policy. No shared code loads or unloads a
plugin.

Renderer access is listener-first and observation-only. Product identity,
authorization, native titlebar, screenshots, startup warnings, cancellation,
parent keepalive, Browse usage, and disposable-exit policy remain Soundcheck.
ABI, editor, state, audio/MIDI, and size constraints remain Signal. macOS is
proved; Windows and Linux remain unsupported. Scale is limited to Signal's
current native 1:1 editor coordinate contract. Card 119 does not widen that
claim: the visible plugin-GUI matrix remains environment-unmet.

## Card 119 Soundcheck Closeout

Soundcheck closes at `e8970aa212e9464f53264f58f6628cd98254fc30` on a
`display -> window` composition: 14 Rust packages and six renderer packages.
Layout, Surfaces, transfer, commands, history, bridge, notifications,
backing-surface, child-view, remote capabilities, and plugin-helper Tauri
windows remain absent.

The refreshed private receipt packs nine TypeScript and 18 Rust artifacts,
installs all three frozen consumer shapes outside workspace resolution, keeps
one Svelte runtime, and publishes nothing. Soundcheck's isolated product
profile verifies 6 portable, 1 hosted, and 3 native products with 7/4/1 host
plans. The complete product gate passes 318 Rust and 30 renderer tests.

The pre-migration build at `f4544e621b8ba3f4558c6b068db1cb36d6ef161f`
reached a stable run loop against the retained isolated profile. Database
semantic digest `64bee46a2f239fce6f6113bb063c866c1bb32426b641a9df2ef6cb42d26e4a15`
and settings digest
`ca3d163bab055381827226140568f3bef7eaac187cebd76878e0b63e9e442356`
were unchanged afterward. No dual write, fallback, cleanup, or live-library
mutation occurred; the temporary worktree was removed.

The macOS acceptance bundle started through the proof-root bypass and hidden
window restore converged on attempt one. Computer control timed out before it
could enumerate apps, so live plugin start/resize/focus/close clicks are
classified environment-unmet. Product-boundary tests cover helper start
contract, resize, focus, close, crash, stale generation, request gaps, and
bounded history. Windows and Linux remain explicitly unsupported.

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

Bovine is the minimal no-service, no-Surface case. Card 120 replaces its
unversioned `workspace.json` writer and sibling aliases with one versioned
preference authority and exact source-linked development packages backed by
produced-artifact proof.

### Selected shared composition

- `platform-native-v1`, canonical id
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

### Card 120 checked cutover

Commit `6afc8da9f4ccc2989541312560eaeb4a154057d2` selects canonical leaf
`com.acowtancy.bovine-accelerator` under `platform-native-v1`, with config,
cache, state, logs, runtime, and backups kept in their native lifecycle roots.
No stable-name override exists.

`bovine.workspace-preferences` schema 1 is the sole UserConfig authority for
workspace root, split ratio, expanded nodes, and selected node. Mutations are
store-coordinated and durable. The legacy file is imported backup-first with
source and target SHA-256 receipts, restart revalidation, conflict refusal,
interrupted resume, retained source, and no cleanup authority.

The sealed modal registry contains product Workspace plus shared Storage.
Workspace root is projected read-only; navigation ratio is the one staged
apply unit. Storage diagnostics is the only config-operation capability. Four
Longhorn TypeScript packages, six Rust crates, and five public Poodle packages
resolve. No optional system edge enters either graph. All 19 unrelated dirty
docs/CHANGELOG paths remain outside the cutover commit.

### Card 121 checked closeout

Bovine commit `8b3c19e8d1e96ce81d1dbbaf4769c9e72648dd28` adds a debug-only,
absolute, confinement-checked acceptance root without changing release storage
selection. A fresh native macOS launch and restart preserve canonical root,
ratio, valid expanded and selected ids, and an unknown product field. Retained
legacy source and backup bytes share SHA-256
`acceef92e04b80327617afaba4fc64e510df85449dd7ec81b1d18590ff1388c6`.

Restart verification now distinguishes valid post-import target evolution
from corrupt recovery state. Exact backup, source, receipt paths, and retained
source bytes remain immutable. Fresh, legacy, interrupted, drift, valid
evolution, corrupt evolution, failed publication, and restart cases pass. The
previous `b713644` workspace suite passes seven tests from a removed temporary
worktree; rollback uses the retained legacy document, not a second writer.

The real modal settings component now owns one session per opening. Apply,
close, listener teardown, reopen, and retained value pass in the renderer.
Bovine QA reports 63 Rust tests, one renderer test, and zero Svelte errors or
warnings. Produced-artifact proof retains four TypeScript, six Rust, and five
public Poodle packages with one Svelte runtime. Layout, Surfaces, transfer,
bridge, commands, history, operations, notifications, and native content are
absent. Native GUI attachment remains environment-unmet because installed and
debug bundles share an identifier; native startup/restart and actual component
behavior are proved separately. All 19 unrelated docs/CHANGELOG paths remain
untouched.

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

### Card 122 checked bridge and command cutover

Jetstream commit `2a8afbb749965cdfe295b8b6be77de4ba9e00256`
replaces the raw whole-state event with a listener-first checked bridge
session. Longhorn owns negotiation, epochs, ordering, gap recovery, discovery,
fresh availability admission, and physical-key resolution. The bridge exposes
only `editor_state.read` and `editor_state.subscribe`; it has no write or
execution authority. Sixteen product commands and eight default bindings stay
defined by Jetstream and execute through typed product routes.

The cutover initially selects four TypeScript packages and four Rust crates.
It adds no generic execute-by-string bridge endpoint, remote backend, config,
settings, layout, Surface, history, operation, or notification edge.

### Card 123 checked backing-surface cutover

Jetstream commit `e9a54daacdec1f5c6573687a5543e9ffb2dae2b0`
adds the stable `island:jetstream.editor.viewport` identity under
`window:editor`. Longhorn owns desired/observed coordination, client epoch,
attach generation, full-host storage versus physical clip planning,
presentation and input gates, apply receipts, and Svelte viewport lifetime.

Jetstream retains the NSView, WGPU surface, renderer, render thread, scene,
world, camera, picking, gizmos, and semantic input. Teardown joins the render
thread, drops WGPU, removes the native view, and releases it. The no-surface
diagnostic remains protocol-visible as absent. Windows and Linux are explicit
unsupported outcomes; live scale-transition host evidence remains unmet.

### Card 124 checked closeout

Jetstream closes on exactly six TypeScript packages and six Rust crates. The
hierarchy is `window -> native-content-island -> backing-surface`; display,
shared windowing, layout, Surfaces, transfer, config, settings, history,
operations, notifications, isolated-window, and child-view remain absent.

The immutable Card 114 artifact ids remain the production boundary. All
Jetstream-selected Longhorn and Poodle paths are unchanged and clean relative
to the frozen artifact sources, even though broader Poodle work has advanced.
The renderer graph has one Svelte runtime. Peer evidence is Svelte
`>=5.38.6 <6`, Tauri API `^2.10.1`, optional Poodle Svelte `0.1.0`, resolved
Svelte 5.56.8 and Tauri API 2.11.1. The Rust artifact graph proves Rust 1.85.0
and Tauri 2.11.5.

No raw state event, raw viewport endpoint, generic bridge execution bus,
renderer shortcut table, or process-lifetime native leak remains. Engine,
renderer, world, gizmo, and platform source paths are unchanged from the
pre-migration commit. That prior source passes 19 Rust unit, four IPC, and 65
renderer tests in disposable sibling worktrees. Rollback needs no dual write,
fallback, or second authority. Package-manager publication remains deferred.

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

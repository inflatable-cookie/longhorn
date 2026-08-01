# Nucleus Migration Map

Status: active migration baseline
Owner: Tom
Updated: 2026-08-01
Governing refs: contracts 003, 004, 009, 012-014, and 017

## Baseline

The read-only audit used Nucleus commit
`c084d57ca15f9e4276f49a9b6b2923f5d10e7313` on `main`. The worktree was
clean at audit start. Card 095's final compatibility receipt uses clean commit
`df5af3da03f8392f948ff65d0a3cf16c36cb6c12`, which adds only operator
acceptance documentation.
Card 096's storage cutover is Nucleus commit
`1c0f8e225849718888afdba63cee4745f623e906`. It uses Longhorn commit
`973c89f260804a777747ead3218b50d146d16118` as its shared-library source.
Card 097's protected-window cutover is Nucleus commit
`fa7f06e7dfdf4a8bde7f4ab48df360a3087a05e7`. It uses Longhorn commit
`32f4bd66e7df813af07514c654ce3b03ddc21ccd`, including sole-primary Tauri
display observation.
Card 098's project-layout cutover is Nucleus commit
`fc083647f1bad76a7f544efe0f1644b66c042571`. Its exact Longhorn shape freeze
is commit `01b9c0a79a8af9214984c29c3969db8a8dc790d3`.
Card 099's renderer cutover is Nucleus commit
`74ca4e7c72f447e064419de6dc72502265cbbf49`. It uses Longhorn Poodle commit
`ca755cbd332260abd971d86509f6190a0e76d269`.
Card 100's native Browser cutover is Nucleus commit
`ce71af24c7f042c16d0f44ee4f13332cb4fdfa98`. Its native policy seam is
Longhorn commit `920bb8c1f19e0ce3a6a5766598d2bfa488ddde63`.
Card 098's unrelated `TerminalPanel.svelte` edit was independently committed
before Card 099 and remained outside both migration diffs.

Nucleus has removed the inherited hosted-Surface layer. Its current hierarchy
is:

```text
display -> window -> region -> panel
```

The primary window contains one project-selected five-region layout. One
checked client session projects the selected project and dispatches exact
revisioned commands. Native window placement remains global. Product use has
therefore strengthened, not weakened, the Longhorn no-Surface composition.

Nucleus roadmap `g05/001` is complete. The operator accepted cross-project
layout retention and confirmed a previously unseen project opens with Agent
Chat only. Card 095 admits bounded donor writes from exact clean receipts.

## Current Donor Authorities

| Concern | Current Nucleus authority | Migration target | Retained Nucleus authority |
| --- | --- | --- | --- |
| storage roots | `desktop_profile.rs` | `longhorn-config`, `longhorn-tauri-config` | app identity, domain registration, retention |
| primary window placement | registered Longhorn host | complete | main-window role, defaults, close policy |
| five-region document | registered Longhorn layout domain | complete | schema registration, project scope, panel catalogue |
| renderer layout state | checked Longhorn client and public Poodle bindings | complete | panel bodies, labels, icons, resources, frame composition, native-handle cleanup |
| native Browser viewport | registered native-content host, child-view adapter, checked client, and Svelte session | complete | browser policy, toolbar, messages, source and lifecycle choices |
| project/task/runtime state | Nucleus server and product crates | none | all authority remains in Nucleus |

`nucleus-workspaces` now retains server-facing product planning records only.
Its unused display, geometry, window, region, local-layout, project-panel, and
fallback-planning modules were removed. Product panel registration, project
scope, presentation/resource bindings, and runtime cleanup remain Nucleus
authority outside the retained crate and shared layout document.

## Storage Baseline And Target

The audited legacy desktop default was `~/.nucleus`. It contains:

- `config/ui.json`
- `state/nucleus.sqlite`
- `state/task-review-snapshots`
- `state/editor-drafts`

`NUCLEUS_DESKTOP_DATA_ROOT` replaces the whole root for proof runs. The CLI
default `.nucleus/local/nucleus.sqlite` is a separate current-working-directory
policy and is outside the first desktop cutover.

The first desktop cutover uses:

- canonical app id `com.inflatablecookie.nucleus`
- no stable storage-name override
- default profile `platform-native-v1`
- canonical id as the storage leaf
- `portable-v1` for explicit isolated proof roots

Durable SQLite data belongs below the data root, conventionally
`databases/nucleus.sqlite`. Window placement is machine state. Project-keyed
workspace layouts, editor drafts, and task-review snapshots are workspace-local
state. UI preferences are user config. Snapshot detail is immutable host-local
review evidence: it remains resolvable while a work item is active or awaiting
review, then enters a seven-day cleanup grace. The storage transition preserves
the live snapshot tree and retention records through a Nucleus adapter. Normal
backup excludes it so backup retention cannot resurrect expired review detail.

The `.nucleus` root is an explicit legacy candidate. Import must inventory it
read-only, use the SQLite snapshot adapter for the live database, split the
combined `ui.json` authority into distinct window and layout domains, commit
the fixed canonical-id locator last, and retain the source until receipted
cleanup. There is no dual-write or silent old-path fallback.

Card 096 implements that target. Startup selects the fixed locator before it
considers import, so a committed transition is not replayed. Missing legacy
storage leaves the native target unselected. Corrupt or future UI state,
occupied targets, overlap, and interrupted transitions fail or recover without
creating a second authority. The source root and unknown files remain intact;
cleanup is deferred.

Card 097 converts the raw window file once into registered machine-state
domain `nucleus.window-placement`. The legacy bytes are copied to the backup
root and digest-verified before the raw target is removed. Receipt interruption
resumes against the published envelope without replaying old placement. Normal
startup has one writer: `ConfigWindowPlacementSink`.

Logical id `window:primary` remains distinct from Tauri label `main`. The
predeclared protected host has no factory and no Surface dependency. Restore
uses canonical display correlation, saved/intersection/main/deterministic
fallback, checked logical geometry, and hidden apply. Reveal requires both
native convergence and `desktop_window_page_ready`. The old geometry listener,
worker, raw read/write helpers, and unconditional `show()` are removed.

## Layout Baseline

The registered Nucleus shape has regions `left`, `center_top`,
`center_bottom`, `right_top`, and `right_bottom`. It has four fixed-point
sizing slots corresponding to the current left/center, center/right,
center-stack, and right-stack ratios.

The migration preserves:

- project-keyed complete layouts
- stable panel instance identity separate from panel kind
- active tabs, close, reorder, cross-region move, and sizing
- one minimal Agent Chat panel for a previously unseen project
- product-owned allowed-region and instance-count policy
- resource targets outside the shared layout document

Card 098 adds expected-revision mutation and separate registered persistence.
Card 099 removes the transition DTO and whole-snapshot save. The renderer now
registers its listener before the first snapshot, rejects older projection
revisions and foreign projects, scopes optimism by request id, and invalidates
late results by client generation. Project switch and unmount destroy the
listener, binding, snapshot, pending presentation map, and optimistic state.
No Surface package, state, protocol, or compatibility alias enters Nucleus.

## Browser Policy Map

The native child remains a mechanism under Nucleus browser policy.

| Policy | Frozen first-cutover choice |
| --- | --- |
| identity | one child island per browser panel instance; stable process-lifetime reuse |
| parent | the protected primary Nucleus window |
| initial source | Nucleus supplies `https://example.com` until product policy changes |
| navigation | operator and page navigation accept only HTTP/HTTPS URLs with a host; missing scheme becomes HTTPS |
| popup | deny and report in trusted Nucleus chrome |
| download | deny and report in trusted Nucleus chrome |
| permission | no Nucleus permission-prompt surface; engine requests gain no app authority |
| data store | preserve the platform engine's normal shared store; no explicit macOS store id in the first cutover |
| persisted browser data | Nucleus config, state, and backups store no cookies or credentials |
| capability | only the bundled main controller gets Nucleus/Longhorn commands; remote child gets no Tauri capability |
| controls | back, forward, reload, URL entry, and explicit system-browser open remain trusted Nucleus UI |
| cursor | retain the macOS allowlisted cursor bridge as Nucleus adapter policy |
| mount | listener before snapshot/ensure; reuse a current child; stale completion cannot reveal it |
| visibility | final visibility requires mounted viewport, nonzero bounds, active panel/workspace, and no intersecting overlay or workspace gesture |
| close | panel close destroys the child; ordinary unmount or project switch hides it for reuse |
| focus | construction stays unfocused; focus behavior must be observed, not fabricated |

Card 099 removes the private selector
`.poodle-popover__surface, [role="menu"]` and all Browser viewport DOM
discovery. Popover and Menu publish public surface-geometry changes into a
Nucleus-owned overlay map. Browser panels publish their viewport rectangles
into the same adapter. Any change on either side recomputes the exact final set
of intersected Browser panel ids. Full-screen project management retains its
separate explicit hide-all policy.

The adapter imports only public Poodle geometry types. It knows no Poodle
class, role, portal structure, generated id, MIME, Tauri type, or Longhorn
native-content mechanism. Mounted and unit tests cover movement between two
Browser viewports, nested Menu surfaces, Browser movement, project switch,
late result rejection, teardown, and remount.

Poodle g12.018 implements the public snapshot seam at
`ef41f412ad7b45c2ee760c1da9bf41ef876855e8`. Its clean Svelte artifact proof is
`ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d`.
Nucleus may consume the exact clean sibling source during private development;
the artifact proof remains the compatibility evidence.

The renderer lock at Card 099 pins local private Longhorn and Poodle sources
through one override graph. Frozen install, Svelte check, mounted tests, and
production build pass without a duplicate Svelte runtime. Package-manager
publication remains deferred.

The current Tauri capability names only the bundled `main` webview. That is
useful negative evidence: remote children receive no app capability. Cutover
must retain an exact capability audit and label validation; a broad wildcard
is not an acceptable substitute.

Card 100 completes that cutover. Stable panel ids map to native-content
islands; destroy/replacement advances attach generation while ordinary unmount
hides for process-lifetime reuse. The Svelte session measures the exact
viewport, supplies explicit device scale, and resolves mounted, active,
nonzero, gesture, and overlay inhibitors. The raw renderer Webview API and its
show/hide/close capability grants are removed.

The child adapter's bounded initialization script and native policy observer
preserve page-load state, popup/download notices, and the macOS cursor bridge
without admitting browser payloads into the shared protocol. Nucleus keeps
HTTP/HTTPS admission, normal shared engine data-store selection, trusted
toolbar, system open, and the rule that no cookies or credentials enter its
stores or backups. Packaged macOS evidence proves attach, physical bounds,
show, inactive hide, same-generation reuse, and remote-capability closure.

## Cutover And Rollback

Each donor-writing card begins from a recorded clean or non-overlapping
Nucleus worktree and records exact clean Longhorn and Poodle source commits
plus their produced artifact identities.
Overlapping unrelated edits stop the card.

Nucleus already uses explicit sibling `file:` dependencies for private Poodle
development. Longhorn may use the same private source-link posture. A source
link is never artifact evidence: Card 095 installs the matching produced graph
in clean temporary consumers. That proof and both Nucleus layout checks pass.
Package-manager publication is deferred to the later release lane.

Cutovers are vertical and single-authority:

1. resolve storage and import legacy state
2. transfer protected-window mechanics
3. transfer project-keyed layout mechanics
4. transfer checked renderer lifetime and public Poodle bindings
5. transfer native child coordination

Cards 096-100 complete all five authority slices. Card 101 owns final restart,
rollback, artifact, duplicate-code, capability, and no-Surface conformance.

Before a slice changes authority, freeze its donor fixtures and preserve its
source data. After it passes, remove the superseded active mechanism in that
slice. Rollback uses the recorded migration receipt and retained source with
the previous app build. It never means live dual-write, a silent legacy read,
or two active implementations.

Final cleanup requires restart evidence across the old and new stores, exact
dependency and capability inventories, duplicate-code searches, and explicit
retained-policy records. Only then does Longhorn become mechanism authority
for the migrated systems.

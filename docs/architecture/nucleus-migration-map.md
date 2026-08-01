# Nucleus Migration Map

Status: active migration baseline
Owner: Tom
Updated: 2026-08-01
Governing refs: contracts 003, 004, 009, 012-014, and 017

## Baseline

The read-only audit used Nucleus commit
`c084d57ca15f9e4276f49a9b6b2923f5d10e7313` on `main`. The worktree was
clean at audit start.

Nucleus has removed the inherited hosted-Surface layer. Its current hierarchy
is:

```text
display -> window -> region -> panel
```

The primary window contains one project-selected five-region layout. Layouts
are local client state keyed by project id. Native window placement remains
global. Product use has therefore strengthened, not weakened, the Longhorn
no-Surface composition.

Nucleus roadmap `g05/001` still has operator-held project-layout validation.
No donor cutover may reinterpret that unaccepted behavior. It must close, or
the operator must explicitly supersede it, before the first overlapping donor
write.

## Current Donor Authorities

| Concern | Current Nucleus authority | Migration target | Retained Nucleus authority |
| --- | --- | --- | --- |
| storage roots | `desktop_profile.rs` | `longhorn-config`, `longhorn-tauri-config` | app identity, domain registration, retention |
| primary window placement | `window_geometry.rs`, `workspace_ui.rs` | `longhorn-windowing`, `longhorn-tauri-windowing`, `longhorn-windowing-config` | main-window role, defaults, close policy |
| five-region document | `workspace_ui.rs`, `nucleus-workspaces` | `longhorn-layout`, `longhorn-layout-config` | schema registration, project scope, panel catalogue |
| renderer layout state | `workspaceUi.ts`, `ProjectWorkspaceStage.svelte` | `@longhorn/layout`, `@longhorn/svelte/layout` and public Poodle bindings | panel bodies, labels, resources, frame composition |
| native Browser viewport | `browser_panel.rs`, `browserPanel.ts`, `BrowserPanel.svelte` | native-content kernel, Tauri child-view adapter, checked client and Svelte session | browser policy, toolbar, messages, source and lifecycle choices |
| project/task/runtime state | Nucleus server and product crates | none | all authority remains in Nucleus |

`nucleus-workspaces` is mixed. Generic display, geometry, window, region, and
panel mechanics are cutover candidates. Project-panel planning, resource
targets, panel kinds, and product defaults stay Nucleus-owned. Migration must
split those responsibilities rather than delete the crate wholesale.

## Storage Baseline And Target

The current desktop default is `~/.nucleus`. It contains:

- `config/ui.json`
- `state/nucleus.sqlite`
- `state/task-review-snapshots`
- `state/editor-drafts`

`NUCLEUS_DESKTOP_DATA_ROOT` replaces the whole root for proof runs. The CLI
default `.nucleus/local/nucleus.sqlite` is a separate current-working-directory
policy and is outside the first desktop cutover.

The first desktop cutover uses:

- canonical app id `dev.nucleus.desktop`
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

It adds expected-revision mutation and separate coordinated persistence.
Renderer whole-snapshot writes cease at cutover. No Surface package, state,
protocol, or compatibility alias enters Nucleus.

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

Current overlay discovery queries the private selector
`.poodle-popover__surface, [role="menu"]`. Poodle publicly exposes open-state
callbacks plus `anchored` and `portal` for consumer-built overlays, but its
built-in Popover and Menu surfaces expose neither their element nor geometry.
The helper queries below the component's former DOM ancestor, while Poodle
portals both anchored surfaces to the theme root. The live surface is therefore
structurally outside the query root. This is a latent donor defect: migration
preserves the stated exact-intersection policy, not the unreachable lookup.
The private selector cannot move into Longhorn. Before renderer cutover,
Nucleus must supply final visibility through explicit consumer state and the
Poodle-owned snapshot seam defined in
`poodle-overlay-geometry-boundary.md`. If that seam cannot preserve
intersection behavior, stop rather than hiding native content for every open
overlay.

Poodle g12.018 implements the public snapshot seam at
`ef41f412ad7b45c2ee760c1da9bf41ef876855e8`. Its clean Svelte artifact proof is
`ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d`.
Nucleus may consume the exact clean sibling source during private development;
the artifact proof remains the compatibility evidence.

The current Tauri capability names only the bundled `main` webview. That is
useful negative evidence: remote children receive no app capability. Cutover
must retain an exact capability audit and label validation; a broad wildcard
is not an acceptable substitute.

## Cutover And Rollback

Each donor-writing card begins from a recorded clean or non-overlapping
Nucleus worktree and records exact clean Longhorn and Poodle source commits
plus their produced artifact identities.
Overlapping unrelated edits stop the card.

Nucleus already uses explicit sibling `file:` dependencies for private Poodle
development. Longhorn may use the same private source-link posture. A source
link is never artifact evidence: Card 095 must also install the matching
produced graph in clean temporary consumers. Package-manager publication is
deferred to the later release lane.

Cutovers are vertical and single-authority:

1. resolve storage and import legacy state
2. transfer protected-window mechanics
3. transfer project-keyed layout mechanics
4. transfer checked renderer lifetime and public Poodle bindings
5. transfer native child coordination

Before a slice changes authority, freeze its donor fixtures and preserve its
source data. After it passes, remove the superseded active mechanism in that
slice. Rollback uses the recorded migration receipt and retained source with
the previous app build. It never means live dual-write, a silent legacy read,
or two active implementations.

Final cleanup requires restart evidence across the old and new stores, exact
dependency and capability inventories, duplicate-code searches, and explicit
retained-policy records. Only then does Longhorn become mechanism authority
for the migrated systems.

# Client, Svelte, Poodle, And Shell Boundary

Status: promoted
Owner: Tom
Updated: 2026-07-29

## Question

What client, reactive, Poodle, drag, and shell behavior is reusable after
g01.006, and which donor behavior still depends on private UI structure?

## Repositories

- `longhorn`
- `poodle`
- `loophole/aura`
- `nucleus/apps/desktop`
- `acowtancy/bovine-accelerator-desktop`

Inspection was read-only. Donor and Poodle worktrees were not modified.

## Delivered Client Evidence

Longhorn already supplies:

- Rust-generated layout, Surface, transfer, and Surface-transfer protocols
- compatibility guards and golden fixtures
- checked Surface and transfer clients
- listener-before-snapshot Surface synchronization
- transfer client-epoch connection state
- one raw Tauri invoke/listen edge

The delivered clients also expose consolidation work:

- Surface and transfer connections implement similar lifetime mechanics
  separately
- late unlisten is asynchronous in one client and synchronous in another
- the Tauri package depends directly on transfer despite being a structural
  transport
- layout has checked data and pure projections but no invented host endpoint

A shared client primitive may unify lifetime and transport mechanics. It must
not fabricate a layout command/event service or erase domain freshness rules.

## Svelte Evidence

Current consumers use Svelte 5. Their exact versions range from `^5.38.6`
through `^5.56.7`. Reactive ownership remains app-local:

- Loophole carries the full Surface and region hierarchy
- Nucleus uses regions and split panes without making Surfaces mandatory
- Bovine uses a smaller shell and no shared workspace host

The reusable boundary is per-window reactive state, explicit start/stop,
authoritative reconciliation, request-keyed optimism, and injected transport
and scheduling. Product registries, panel bodies, labels, icons, and shell
policy remain consumer inputs.

## Poodle Evidence

Current Poodle publicly exports `Tabs`, `DockRegion`, and `SplitView`.
Their controlled APIs cover:

- tab value, reorder, close, and drag-start callbacks
- dock selection, collapse, reorder, eligibility, and panel-drop callbacks
- split ratio, collapsed posture, and resize callbacks
- snippets for panel and pane bodies

That is sufficient for checked same-window layout bindings.

Cross-window drag is not yet a clean public binding. `DockRegion` owns a
private HTML5 payload and does not expose the full drag start, end, and
external-session drop seam. Current Loophole compensates with capture handlers
that inspect generated tab ids and private Poodle class names. Longhorn must
not promote those selectors or payloads.

Host session creation is asynchronous while native `DataTransfer` writes are
valid only during `dragstart`. The public seam must support pre-drag session
arming. A session that is not ready cannot be replaced by a renderer-created
payload or attached after the event.

The reusable transfer binding therefore waits for a public Poodle extension
seam. Poodle remains authority for drag interaction and affordances. Longhorn
owns session ids, target eligibility, leases, and authoritative commits.

## Package Evidence

`@inflatable-cookie/poodle-svelte` currently identifies itself as a private `0.1.0` preview and
peers Svelte `^5.38.6`. Loophole and Bovine still use source aliases; Nucleus
uses file dependencies. These are useful API evidence, not install or release
evidence.

Longhorn cannot claim a broad Poodle compatibility range from that state.
Before public release:

- adapter packages remain private
- validation uses one exact, packable Poodle preview artifact
- Poodle package metadata and public drag hooks must pass an install proof
- registry ownership and the published prerelease range remain a later release
  gate

## Shared Shell Evidence

Loophole and Nucleus contain the same titlebar drag helper except for the log
namespace. A shared helper can reject non-primary or modified gestures and
interactive descendants, then call an injected native drag function and
report through an injected sink.

Theme and presentation bootstrap, readiness reveal ordering, capability
diagnostics, and explicit error surfaces are reusable guidance and small
helpers. A single application shell component is not. Minimal Bovine and full
Loophole examples must remain visibly different compositions.

## Promoted Decisions

- `@inflatable-cookie/longhorn-core` owns structural transport and checked subscription lifetime
  primitives.
- Domain packages retain compatibility guards, commands, and freshness rules.
- `@inflatable-cookie/longhorn-tauri` implements structural invoke/listen transport and carries
  no domain dependency.
- `@inflatable-cookie/longhorn-svelte` has a Surface-free root and optional domain subpaths.
- Optional domain peers are marked optional and are never root re-exports.
- `@inflatable-cookie/longhorn-poodle` binds public components; it does not persist
  `WorkspaceLayoutSnapshot` or infer product metadata.
- Poodle public drag extension and packable preview evidence form a named
  upstream checkpoint.
- cross-window sessions are armed before native dragstart and cancelled when
  unused, superseded, unmounted, or destroyed
- Longhorn drag payloads use only the shared transfer protocol. Private Poodle
  MIME values, DOM ids, and CSS classes are forbidden dependencies.
- titlebar, readiness, theme, capability, and error guidance stays
  compositional.

## Promotion

Promoted into:

- `../../architecture/package-topology.md`
- `../../architecture/repo-authority-map.md`
- `../../contracts/012-distribution-and-compatibility.md`
- `../../contracts/013-svelte-and-poodle-adapter-lifecycle.md`
- `../../roadmaps/g01/007-typescript-svelte-poodle-and-app-shell.md`
- Cards 036 through 041

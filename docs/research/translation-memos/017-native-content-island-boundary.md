# 017 Native Content Island Boundary

Status: complete and promoted  
Owner: Tom  
Updated: 2026-07-31  
Promotes: `../../contracts/017-native-content-island-coordination.md`

## Prompt

Revalidate the current Nucleus, Soundcheck, and Jetstream native-content
mechanisms. Decide whether Longhorn should own one host implementation, a
smaller coordination protocol, several mechanism adapters, or no shared API.

## Sources

All donor audits were read-only. Each worktree had local changes, so the
observed source is evidence rather than stable donor authority.

Nucleus at `8c95c9c9eae5d340cf2f5faf0a3c3d4743059d29`:

- `docs/contracts/028-browser-panel-runtime-contract.md`
- `apps/desktop/src-tauri/src/browser_panel.rs`
- `apps/desktop/src/lib/BrowserPanel.svelte`
- `apps/desktop/src/lib/browserPanel.ts`
- `apps/desktop/src/lib/nativePanelVisibility.ts`
- `apps/desktop/src-tauri/src/tests/panel_guards.rs`
- `apps/desktop/src-tauri/tauri.conf.json`

Soundcheck at `aa749d1e577e7956b75a892cdf589304d23b7186`:

- `docs/architecture/003-system-architecture.md`
- `docs/contracts/014-minimal-product-surface-contract.md`
- `src-tauri/src/plugin_inspection.rs`
- `src-tauri/src/plugin_inspection_process.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/Cargo.toml`

Jetstream at `95222397974caa276123fa52c284024d18eadb3c`:

- `crates/jetstream-editor-tauri/README.md`
- `crates/jetstream-editor-tauri/src/surface.rs`
- `crates/jetstream-editor-tauri/src/lib.rs`
- `crates/jetstream-editor-tauri/src/commands.rs`
- `crates/jetstream-editor-tauri/src/scene.rs`
- `editor-ui/src/bridge.ts`
- `editor-ui/src/Editor.svelte`
- `editor-ui/tests/viewport-input.test.ts`
- `crates/jetstream-editor-tauri/tauri.conf.json`

Current Tauri 2.11.5 references:

- `WebviewBuilder` and `Window::add_child`
- `Webview` bounds, focus, visibility, close, and reparent operations
- `WindowBuilder`
- `WindowEvent`

The current `WindowEvent` surface exposes resize, move, focus, scale change,
close, and destroy. It does not expose a portable occlusion event. Native
occlusion therefore needs injected platform evidence or an explicit
consumer-computed visibility target.

Nucleus and Soundcheck Effigy graphs reported refresh recommended because of
dirty worktrees. They were not refreshed because that would write donor-local
index state. Exact source searches supplied final evidence. Jetstream's graph
was current and corroborated by exact source reads.

## Current Mechanisms

| Concern | Nucleus | Soundcheck | Jetstream |
| --- | --- | --- | --- |
| host form | child Tauri webview in main window | isolated Tauri native window in a helper process | sibling `NSView` below a transparent webview |
| content | remote HTTP page | third-party CLAP, VST3, or AU editor | Jetstream WGPU scene |
| lifetime | one child per panel id; hide/reuse; close with panel | fresh process and one plugin per launch; window close exits helper | one leaked retained view and render thread for process lifetime |
| geometry source | DOM viewport `getBoundingClientRect()` | plugin size plus user window resize | DOM viewport rect plus native window physical size |
| geometry effect | child position and size | content-window resize negotiation | full-window backing surface; render/blit clipped to viewport |
| scale | logical child bounds; no scale-change proof | scale passed at attach; resize converts physical to logical | CSS rect converted to physical; no scale-change handler |
| visibility | manual hide/show for inactivity, drag, and overlapping overlays | plugin show/hide/close events drive native window | zero viewport suppresses render; webview remains above surface |
| focus | child starts unfocused; native interaction takes over | window shown and focused; focus loss releases MIDI notes | webview owns focus; viewport pointer input is forwarded |
| input | native webview input; macOS cursor sentinel workaround | native plugin view plus host keyboard/MIDI adapters | semantic pointer/button/wheel forwarding from webview |
| product authority | URL, navigation, panel policy | binary selection, process isolation, plugin ABI, audio/MIDI, presets | renderer, scene, camera, gizmo, frame loop |
| platform proof | intended through Tauri engines; current source not packaged here | macOS only | macOS only; other targets return unsupported |

## Donor Findings

### Nucleus

Nucleus creates an unprivileged remote child with `WebviewBuilder` and
`Window::add_child`. Trusted Svelte chrome listens before creation, measures a
DOM viewport, and sends logical position and size to Rust. The child keeps its
native session while hidden and closes only with its Browser panel.

The implementation has useful generic evidence:

- one opaque content instance per panel id
- renderer-owned desired viewport, host-owned native mutation
- hide before inactive layout, drag, or overlay composition
- listener-before-create runtime observation
- explicit close instead of unmount-implies-destroy

Its URL policy, browsing data, popup/download behavior, cursor sentinel, and
remote-content capability policy remain Nucleus behavior. Overlay collision is
computed from Poodle/DOM presentation and cannot become Rust host authority.

Current gaps are scale-change proof, native focus observation, minimize or
occlusion readback, ordered apply receipts, and mounted behavioral tests. The
source-shape guard proves wiring, not packaged native behavior.

### Soundcheck

Soundcheck's desktop process never loads a plugin. It launches the same binary
as a disposable helper, clears configured webview windows, creates one native
Tauri window, inserts a host `NSView`, and asks Signal's plugin adapter to
attach the editor. Closing the inspection window exits the helper immediately
so unsafe third-party unload paths do not run.

The host supports both directions of geometry:

- plugin-requested resize, show, hide, close, and resize-hint changes
- user window resize converted from physical size through current scale
- startup resize settling and recentering
- plugin-specific size acceptance and cycle suppression

This is strong lifecycle and geometry-negotiation evidence. It is not a
generic native-view implementation. Plugin ABI selection, process isolation,
unsafe unload policy, audio/MIDI, preset state, screenshots, titlebar controls,
and native root-view normalization remain Soundcheck or Signal authority.

Current gaps are non-macOS adapters, explicit scale-change handling,
occlusion/minimize evidence, and a product-neutral packaged fixture that can
exercise hostile resize and teardown behavior without a proprietary plugin.

### Jetstream

Jetstream attaches one engine-owned `NSView` below the webview, retains it for
the process lifetime, and creates a WGPU surface from its raw handle. The view
tracks the full native content size. Poodle/Svelte leaves a transparent hole,
reports the viewport in CSS pixels, and forwards pointer semantics because the
webview remains the input target.

The render thread converts viewport geometry to physical pixels, clamps it to
the current surface, renders to an offscreen viewport texture, then blits into
the named rectangle. Chrome input never reaches the engine. The engine owns
camera, picking, gizmo, renderer, and frame timing.

This proves that one desired content viewport does not imply moving or sizing
the native view. A backing surface may fill the host while using the viewport
as a clip and interaction region.

Current gaps are non-macOS attach, scale-factor changes after attachment,
visibility/minimize/focus observation, reversible detach, and packaged proof
across display scales. The process-lifetime retained `NSView` is valid donor
policy but cannot be a generic teardown contract.

## Decision

### Promote shared coordination, not one host implementation

The reusable seam is revisioned desired and observed state around an opaque
native content instance:

- island identity and consumer-owned kind
- host-window binding and attach generation
- host-local `ClientCssPx` viewport plus explicit `ScaleFactor`
- desired presence and visibility
- focus intent and input-routing mode
- observed attachment, effective visibility, focus, and geometry
- ordered attach, update, detach, and failure receipts
- stale-generation rejection and idempotent teardown

The viewport means "where native content is presented and interactive." Its
mechanism-specific effect differs:

- child view: move and size the child
- isolated window: size the window's content area and use Longhorn windowing
  for outer placement
- backing surface: retain full-host geometry and clip rendering/input to the
  viewport

This semantic definition is the common denominator. A generic API must not
pretend all three are movable child views.

### Keep mechanisms separate

The prototype must use separate adapters or injected ports for:

- child-webview construction, navigation, data store, and capability policy
- isolated native-window construction and content-size negotiation
- backing-surface creation, render ownership, and semantic input forwarding

No adapter may depend on another. A consumer selects only its mechanism.
`longhorn-windowing` remains outer-window authority. Poodle remains layout and
overlay presentation authority. The consumer remains content, security,
execution, and input-semantic authority.

### Visibility is explicit policy

Longhorn may combine stable visibility inhibitors, but it cannot discover DOM
overlay intersection or infer platform occlusion from elapsed time. Consumers
submit a final desired visibility and reason. Adapters report actual
visibility when the platform can observe it. Unknown is distinct from hidden.

### Input payloads do not become generic

The common protocol records `native_direct`, `renderer_forwarded`, or
`disabled`. It does not standardize browser events, plugin events, MIDI,
camera controls, picking, or gizmo payloads. Focus requests are receipted host
operations, not proof that product input authority moved into Longhorn.

## Provisional Package Direction

If the prototypes pass:

- `longhorn-native-content`: pure identities, desired/observed state, planning,
  revisions, and receipts
- narrow Tauri mechanism adapters selected independently
- `@longhorn/native-content`: checked coordination client
- optional per-instance Svelte lifecycle and Poodle layout binding only after
  public-seam proof

Names and adapter count remain provisional. No package should enter the
workspace until the three mechanisms prove that the pure vocabulary remains
lossless and each unused adapter stays absent from the dependency graph.

## Lossless Donor Map

| Donor behavior | Shared coordination | Consumer or specialist owner |
| --- | --- | --- |
| stable browser child per panel | island id, presence, host binding | Nucleus panel and navigation policy |
| DOM viewport measurement | typed desired client viewport | Svelte/Poodle layout composition |
| overlay-driven hide | visibility inhibitor/result | Nucleus overlay intersection |
| plugin process isolation | attach generation and terminal failure | Soundcheck helper supervision |
| plugin resize request | bounded content-size proposal and receipt | Signal ABI adapter and Soundcheck acceptance |
| unsafe plugin unload avoidance | teardown outcome only | Soundcheck disposable-process policy |
| WGPU sibling view | backing-surface mechanism descriptor | Jetstream surface and renderer |
| viewport input forwarding | input-routing mode | Jetstream camera, picking, and gizmo protocol |
| scale conversion | explicit scale evidence and typed rounding | adapter observation and mechanism apply |
| Poodle overlay or panel body | none | Poodle and consumer renderer |

## Prototype Requirements

- one pure desired/observed trace must represent all three shapes without
  donor payloads
- Nucleus proof covers active/inactive, panel close, overlay hide, focus, and
  1x/2x geometry
- Soundcheck proof uses a controllable fake native child and covers host- and
  content-driven resize, show/hide/close, helper loss, and teardown timeout
- Jetstream proof covers viewport move/collapse, window resize, scale change,
  focus/input gating, destroy, and explicit detach policy
- stale renderer observations and stale adapter completions cannot mutate the
  current attach generation
- partial native apply returns exact attempted and skipped operations plus
  fresh observation
- macOS packaged proof is required for all three; Windows and Linux support or
  explicit unsupported evidence is required before a cross-platform claim
- minimal dependency graphs omit child-webview, plugin, GPU, Svelte, and
  Poodle edges unless selected

## Rejected

- one universal native-view trait that contains browser, plugin, and GPU hooks
- assuming desired viewport always equals native child bounds
- renderer-owned durable native truth
- untyped CSS, logical, and physical rectangles
- treating Tauri capabilities as product authorization
- inferring occlusion from focus or elapsed time
- generic pointer, MIDI, plugin, or render payloads
- Poodle overlay inspection inside Rust host code
- copying plugin ABI or Jetstream renderer behavior into Longhorn
- claiming Windows or Linux support from macOS source shape

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../architecture/system-inventory.md`
- `../../contracts/017-native-content-island-coordination.md`
- `../../specs/001-shared-desktop-system-suite.md`
- `../../roadmaps/g01/013-native-content-islands-prototype.md`

## Card 086 Promotion Outcome

Decision: `Promote`.

Cards 082-085 show one lossless product-neutral coordination vocabulary and
three dependency-isolated mechanisms. Promote the pure kernel, separate child
view, isolated-window, and backing-surface layers, a checked TypeScript client,
and a per-instance Svelte lifecycle package. Do not add a Poodle-specific
package.

The initial native-host claim is macOS-only. Child-view Windows and Linux are
unproved. Isolated-window and backing-surface Windows and Linux are
unsupported. The packaged child and backing runs had one attached 2x display:
deterministic 1x/2x conversion passes, but live native scale switching remains
unproved. Child-view focus and visibility may truthfully remain `unknown`.

Browser policy, plugin ABI and helper ownership, GPU/render authority,
semantic input, raw native handles, and outer-window placement remain outside
the promoted graph. Poodle remains a public layout and presentation seam.

Retain all four prototype workspaces as non-publishable evidence. g01.018
Cards 087-093 implement and artifact-prove production packages before any
donor migration.

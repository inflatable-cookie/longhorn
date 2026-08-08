# 017 Native Content Island Coordination

Status: active promoted production boundary  
Owner: Tom  
Updated: 2026-08-03
Evidence: `../research/translation-memos/017-native-content-island-boundary.md`

## Boundary

Longhorn may provide optional coordination for host-owned native content that
is presented through a desktop window but is not ordinary renderer DOM.

The first mechanisms are:

- a child webview positioned inside a host window
- an isolated native content window
- a native backing surface beneath a transparent webview

The shared boundary owns desired and observed coordination. It does not own
browser policy, plugin hosting, render loops, product input, or outer-window
placement.

## Package Gate

Card 086 selects `Promote` from Cards 082-085 evidence. The production graph
is one pure kernel, three independently selectable mechanism layers, a checked
TypeScript client, and a separate Svelte lifecycle package:

- `longhorn-native-content`
- `longhorn-tauri-native-content-child-view`
- `longhorn-native-content-isolated-window`
- `longhorn-native-content-backing-surface`
- `@inflatable-cookie/longhorn-native-content`
- `@inflatable-cookie/longhorn-native-content-svelte`

These working names are fixed for g01.018 but are not released registry or
compatibility promises. There is no Poodle-specific native-content package.

Cards 082-085 remain non-publishable evidence. Cards 087-093 complete the
production package and isolated-artifact gate. Donor migration remains
separately gated by contract 003 and the exact consumer authority map. Native
host support starts on macOS. Child-view Windows and Linux are unproved;
isolated-window and backing-surface Windows and Linux are unsupported.

## Authority

Longhorn may own:

- opaque bounded island identity and revision
- consumer-supplied bounded kind identity
- one current host-window binding and attach generation
- typed desired and observed coordination state
- pure update planning and ordered operation receipts
- stale-generation rejection
- exact detach and failure evidence
- child-view-specific, generation-checked navigation mechanics and exact
  native-side receipts

Consumers own:

- content creation, security, authorization, and product identity
- browser URL, navigation, data-store, download, popup, and permission policy
- plugin discovery, ABI, process isolation, unload, audio, MIDI, state, and
  screenshot policy
- GPU device, surface, renderer, frame loop, scene, camera, picking, and gizmo
- overlay intersection, panel activity, and final visibility policy
- semantic input payloads and command execution
- platform support policy and outer-window placement

## Child-view Document Navigation

A retained child webview may navigate without changing island identity or
attach generation. This is a child-view mechanism operation, not a common
`NativeContentOperation` and not part of the renderer protocol.

The child-view adapter accepts one exact attach generation and one parsed URL.
It must:

1. reject stale, future, retired, attaching, or absent generations before
   native work;
2. evaluate the consumer-supplied `ChildViewSpec` navigation policy before
   reading or mutating the native webview;
3. read the fresh native URL and return `unchanged` without navigation when
   the requested document is already current;
4. otherwise submit exactly one native navigation and return `submitted`;
5. report the previous and requested URLs without claiming that the load
   completed.

Page-load start clears adapter readiness for the current generation. Page-load
finish restores readiness. Fresh current-document observation remains
native-side and generation-checked. URLs, browser history, selection identity,
and navigation policy never enter the mechanism-neutral desired/observed
protocol.

Policy denial, URL observation failure, native navigation failure, and stale
authority are typed. They do not recreate the child, advance attach
generation, or mutate common coordinator state. Consumer Tauri commands may
expose this native operation under their own authorization. Global label
lookup and raw-handle escape are not canonical control paths.

Back, forward, reload, browser-history persistence, redirects, downloads,
popups, and permissions remain outside this operation.

Poodle owns layout and visual primitives. `longhorn-windowing` owns desired and
observed outer-window placement. Native-content coordination may bind to a
`WindowId`; it does not duplicate window authority.

## Identity And Generation

An island has one opaque `NativeContentIslandId`. Tauri labels, panel ids,
plugin ids, process ids, raw pointers, window handles, and render-target ids do
not become canonical identity implicitly.

Every attach attempt has a monotonically advancing generation. Renderer
measurements, platform events, adapter completions, and teardown reports name
that generation. Older or future generations fail typed and leave current
state unchanged.

Registration does not create content. Unregistration does not imply that a
consumer-owned process, plugin, or renderer shut down successfully.

## Mechanism Description

The shared descriptor distinguishes:

- `child_view`: native child bounds follow the desired viewport
- `isolated_window`: native content fills an independently planned window
- `backing_surface`: consumer-rendered native backing storage may fill the
  host while rendering and input are clipped to the desired viewport; it is
  not a second webview

It also declares:

- whether content can request a new content size
- whether attachment and detachment are reversible
- whether input is `native_direct`, `renderer_forwarded`, or `disabled`
- whether effective visibility and focus can be observed

Descriptions are capabilities, not instructions to emulate an unsupported
mechanism. An adapter rejects a desired operation it cannot honor.

## Geometry

The desired content viewport uses host-local `ClientCssPx`. It is the region
where content is presented and interactive, not necessarily the native view's
frame.

Every host apply also names current `ScaleFactor` evidence and an explicit
rounding mode when physical conversion is required. A zero, non-finite,
overflowing, stale, or missing scale fails typed. CSS, screen-DIP, window
frame, and physical geometry are never substituted.

Mechanism mapping is explicit:

- child-view adapters set native child position and size
- isolated-window adapters treat the viewport as the content area and delegate
  outer placement to `longhorn-windowing`
- backing-surface adapters keep their native storage policy and apply the
  viewport as render and interaction clipping

A content-driven resize is a proposal tied to current island revision and
attach generation. Consumer policy accepts, constrains, or rejects it. The
adapter cannot directly rewrite durable desired geometry.

Fresh observation decides convergence. A successful mutation call does not
fabricate observed bounds.

## Presence And Lifecycle

Coordination lifecycle is separate from product content lifecycle:

- `absent`
- `attaching`
- `attached`
- `detaching`
- `failed`

Readiness is consumer-reported evidence within one attached generation. It
does not mean a page loaded, a plugin initialized, or a renderer produced a
frame unless the selected adapter defines and proves that condition.

Attach, update, and detach plans are ordered. Every operation returns applied,
failed, and dependency-skipped outcomes. A failure leaves desired state
inspectable and cannot silently advance observed state.

Detach is idempotent. When safe in-process detach is unavailable, an adapter
may require owner-process termination and report that policy. Longhorn does
not call unsafe third-party unload paths to simulate a generic detach.

Host-window destroy invalidates its island generations before later native
events are admitted. Shutdown has a bounded timeout and returns unresolved
islands explicitly.

## Visibility And Occlusion

Desired visibility is explicit. A consumer may combine stable inhibitors such
as inactive panel, collapsed layout, drag, overlay, hidden host, or shutdown,
then submit the final result and a bounded reason id.

Longhorn does not inspect DOM or Poodle overlays. It does not infer occlusion
from focus, elapsed time, or a missing frame. Effective visibility is
`visible`, `hidden`, or `unknown` from adapter observation.

Hide and show are ordered native operations. A renderer unmount does not imply
destroy. A backing surface may suppress render for an empty viewport without
detaching its native storage.

Portable occlusion is not claimed from Tauri 2.11 `WindowEvent`; a platform
adapter must inject stronger evidence or retain `unknown`.

## Focus And Input

Focus intent is `unchanged`, `request`, or `release-if-owned`. A focus request
returns an operation result; it does not prove the product accepted input.

The common protocol records only the input-routing mode:

- `native_direct`
- `renderer_forwarded`
- `disabled`

Browser events, plugin GUI events, keyboard mappings, MIDI, pointer gestures,
camera movement, picking, and gizmo samples remain typed mechanism or consumer
data. They never enter a generic JSON payload or native-content command bus.

Focus loss may gate or release consumer input through an injected callback.
Longhorn does not synthesize product input.

## Tauri Boundary

Current Tauri child webview construction uses the unstable `WebviewBuilder`
surface. A Tauri adapter must isolate that feature and its platform limits from
the pure package.

Capabilities admit renderer access to Tauri commands. They do not authorize
content, navigation, plugin loading, rendering, or product mutation. Remote
child webviews receive no consumer capabilities unless the consumer contract
grants them explicitly.

Tauri window and webview labels remain transport identifiers. Raw native
handles and pointers stay inside the selected adapter and never cross the
renderer protocol.

## Svelte And Poodle

An optional Svelte adapter may own one per-instance coordination session,
measure a consumer-supplied viewport element, combine explicit visibility
inhibitors, reject stale async results, and tear down listeners and observers.

Poodle integration is layout binding only. Poodle owns panels, overlays,
menus, focus visuals, and presentation timing. Longhorn must not inspect
private Poodle DOM, copy components, or treat a Poodle panel as native host
authority.

Card 086 promotes the per-instance Svelte lifecycle because viewport
measurement, explicit visibility/focus gates, stale async rejection, and
listener/observer teardown recur across child-view and backing-surface shapes.
Poodle stays a public consumer composition seam; no native-content package may
depend on it without a later public-seam proof and contract change.

## Validation

- one product-neutral desired/observed trace represents all three mechanisms
- child-view proof covers reuse, hide/show, close, focus, overlay inhibition,
  policy-admitted idempotent navigation, readiness transitions, and
  deterministic 1x/2x viewport geometry; live scale switching was unavailable
  and focus/visibility observation may remain `unknown`
- isolated-window proof covers content- and host-driven resize, show/hide/
  close requests, helper loss, focus loss, and bounded teardown
- backing-surface proof covers viewport move/collapse, window resize,
  deterministic 1x/2x scale conversion, input gating, destroy, and selected
  detach policy; live scale switching was unavailable
- stale renderer measurements and native completions leave exact state
  unchanged
- partial apply exposes attempted, failed, and dependency-skipped operations
- macOS packaged proof covers all three shapes within the recorded live-scale
  environment limit
- Windows and Linux are proved, explicitly unsupported, or explicitly unproved
  per mechanism; no support follows from source portability
- minimal graphs omit child-webview, plugin, GPU, Tauri, Svelte, and Poodle
  dependencies when not selected
- donor product payloads and raw native handles never cross shared fixtures

## Rejected

- one universal host trait containing browser, plugin, and GPU methods
- assuming viewport and native-view bounds are identical
- renderer-authoritative durable native state
- untyped geometry or ambient scale conversion
- capability permission as product authorization
- focus as inferred visibility or occlusion
- generic pointer, MIDI, plugin, or render payloads
- automatic plugin unload during generic detach
- Rust inspection of Poodle overlay DOM
- required Tauri, Svelte, Poodle, plugin, or GPU dependencies in the pure root
- cross-platform claims from one macOS implementation

## Promotion Record

| Gate | Evidence | Decision |
| --- | --- | --- |
| common vocabulary | Card 082 has 21 passing pure tests and lossless traces for all three mechanisms | promote pure kernel |
| dependency isolation | pure and each packaged graph omit other mechanisms and product stacks | promote separate mechanism layers |
| child view | packaged macOS attach, retained policy-admitted navigation, reuse, geometry, visibility commands, focus command, security, close, and teardown pass | promote macOS-first adapter; keep Windows/Linux unproved |
| isolated window | packaged macOS 11/11 resize, lifecycle, helper-loss, timeout, and teardown matrix passes | promote macOS-only adapter |
| backing surface | packaged macOS clip, render/input gating, resize, destruction, and reversible detach pass | promote macOS-only adapter |
| live scale | host exposed one attached 2x display; deterministic 1x/2x conversion passes but live transition is unmet | exclude mixed-display proof from claims |
| UI lifecycle | measurement and mounted gating repeat across retained mechanisms | promote Svelte layer; retain Poodle as public external seam |
| authority | no browser, plugin, GPU, semantic input, raw-handle, outer-placement, or private-Poodle authority leaked | keep all listed boundaries external |

The canonical composition and migration prerequisites are in
[Native-content Island Composition](../architecture/native-content-island-composition.md).

## Artifact Gate Record

Card 093 proves the production boundary from produced artifacts rather than
workspace source:

- five Rust source inventories pass `cargo package --list` and compile in
  four isolated offline consumer graphs
- three TypeScript packages install into Nucleus-, Soundcheck-, and
  Jetstream-shaped roots without sibling or workspace resolution
- Rust and renderer traces agree for child view, isolated window, and backing
  surface
- optional mechanisms, browser/plugin/GPU payloads, raw native handles,
  private Poodle structure, and unselected Svelte/Poodle graphs remain absent
- fresh macOS packaged lifecycle/teardown reruns preserve the exact support
  ledger and do not manufacture an unavailable live scale transition
- retained prototypes remain non-publishable, outside production workspace
  membership, and without compatibility authority

The gate admits Nucleus migration planning only. Native browser cutover still
requires an explicit browser construction, navigation, data-store,
capability, and visibility policy map. Soundcheck and Jetstream remain behind
the sequential consumer runway and their named plugin/helper or GPU/input
authority maps.

Cards 132-134 extend the child-view artifact without changing the common
protocol. The staged private source artifact compiles generation-checked
current-URL observation and navigation from an isolated consumer. The common
fixture and generated TypeScript digests remain exact. Packaged macOS evidence
proves submitted navigation, same-URL idempotence, policy denial, readiness,
and retained generation. Figmatic owns its preview URLs and command policy;
Nucleus owns its browser actions and later removal of global-label lookup.

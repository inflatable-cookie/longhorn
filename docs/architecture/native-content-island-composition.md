# Native-content Island Composition

Status: canonical promoted direction
Owner: Tom
Updated: 2026-08-03
Governing contract: [017 Native Content Island Coordination](../contracts/017-native-content-island-coordination.md)

## Decision

Card 086 selects `Promote`.

Promote one pure coordination kernel and three independently selectable
mechanism layers. Promote checked TypeScript and per-instance Svelte lifecycle
support. Keep Poodle as a public layout and presentation seam without a
Longhorn native-content package dependency.

The Card 082-085 workspaces remain non-publishable evidence. Cards 087-093
implement the production pure kernel, generated checked renderer protocol,
framework-neutral client, isolated Tauri child-view mechanism, generic
process-isolated window mechanism, generic backing-surface mechanism, and
per-instance Svelte lifecycle. Card 093 proves isolated produced artifacts and
records the consumer gates. Donor cutover still requires a migration card and
the named consumer-owned policy map.

## Package Graph

Working production names are fixed for g01.018. Registry ownership remains a
release gate.

```text
longhorn-core
  └─ longhorn-native-content
       ├─ longhorn-tauri-native-content-child-view
       ├─ longhorn-native-content-isolated-window
       └─ longhorn-native-content-backing-surface

longhorn-native-content
  └─ @inflatable-cookie/longhorn/native-content
       └─ @inflatable-cookie/longhorn-poodle-svelte/native-content

Poodle public layout seam ─ consumer composition only
```

`longhorn-native-content` owns identity, generation, typed state, planning,
proposals, receipts, and stale-result rejection. It has no Tauri, browser,
plugin, GPU, Svelte, or Poodle dependency.

The child-view package isolates Tauri child-webview construction and native
operations. The isolated-window and backing-surface packages define generic
coordination and injected host ports. Product helpers, plugins, native views,
GPU surfaces, and renderers stay outside Longhorn.

The child-view seam accepts a bounded optional initialization script and a
native observer for page-load, denied-popup, denied-download, and supported
document-title events. These hooks preserve consumer trusted-chrome and
construction policy without adding browser payloads to the shared renderer
protocol. The adapter also exposes generation-checked native-side URL
observation and policy-admitted navigation. Navigation admission, notices,
cursor interpretation, data-store choice, and all persisted browser state
remain consumer authority.

Navigation is deliberately absent from `longhorn-native-content` and
`@inflatable-cookie/longhorn/native-content`. It is specific to the retained child-view
mechanism. A consumer-owned Tauri command may call the adapter without
exporting a raw webview handle or granting the remote child a capability.
Repeated navigation to the fresh current URL is unchanged; a submitted receipt
means only that the native runtime accepted the request. Page-load callbacks
separately drive not-ready/ready evidence.

`@inflatable-cookie/longhorn/native-content` is generated from Rust authority. The Svelte
package owns mounted connection lifetime, viewport measurement, explicit
visibility and focus gates, stale async rejection, and teardown. It accepts a
consumer-supplied element and policy; it does not inspect Poodle internals.

## Authority

| Concern | Shared owner | External owner |
| --- | --- | --- |
| identity, attach generation, revisions | pure kernel | consumer supplies product identity mapping |
| desired and observed coordination state | pure kernel | adapter supplies native observations |
| viewport and explicit scale | pure kernel types and plans | renderer measures; adapter converts and applies |
| outer-window placement | `longhorn-windowing` | consumer placement policy |
| child browser construction and policy | child-view adapter seam | consumer browser/security policy |
| isolated child process and native content | isolated-window injected ports | consumer helper, plugin ABI, authorization |
| backing storage and render/input execution | backing-surface injected ports | consumer renderer and semantic input protocol |
| Svelte mounted lifetime | Svelte adapter | consumer visibility and focus policy |
| panels, overlays, presentation | none | Poodle and consumer renderer |

## Mechanism Mapping

| Shared operation | Child view | Isolated window | Backing surface |
| --- | --- | --- | --- |
| attach | create or bind child webview | bind consumer-created content owner | bind consumer-created native storage |
| viewport | set child position and size | propose content-area size; outer placement stays external | set render and interaction clip; storage may fill host |
| visibility | native show or hide | native show or hide | render/input gate plus adapter observation |
| focus | request or release child focus | request or release isolated content focus | gate forwarded input; consumer owns semantic focus |
| detach | remove or close child | detach or terminate owner under declared policy | reversible detach or declared destruction policy |

All commands and observations name the current attach generation. Partial
apply returns exact attempted, failed, and dependency-skipped operations.
Fresh observation, not mutation success, decides convergence.

## Evidence And Support

| Layer | macOS | Windows | Linux | Production claim |
| --- | --- | --- | --- | --- |
| pure coordination | deterministic 1x/2x traces pass | portable code; target artifact pending | portable code; target artifact pending | semantics promoted |
| child view | packaged proof passes with live scale switch unavailable; focus and visibility may be `unknown` | unproved | unproved | macOS first; no other host claim |
| isolated window | packaged 11/11 proof passes | unsupported | unsupported | macOS only |
| backing surface | packaged proof passes with live scale switch unavailable; content is consumer-rendered, not another webview | unsupported | unsupported | macOS only |

The child and backing proofs ran on one attached 2x display. Deterministic 1x
and 2x conversion passes, but no live native display transition was available
or simulated. g01.018 must not advertise mixed-display proof until a suitable
host run exists. `unknown` visibility and focus are valid observations, not
fabricated success.

## Explicit Exclusions

- browser navigation policy, history, downloads, permissions, data stores,
  and popup policy
- plugin discovery, ABI, unload, audio, MIDI, screenshots, and Signal payloads
- GPU device, render loop, scene, camera, picking, and gizmo authority
- semantic pointer, keyboard, MIDI, plugin, or render payloads
- raw native pointers or handles in shared or renderer protocols
- outer-window placement
- private Poodle DOM or a Poodle-specific native-content adapter
- Windows or Linux native host support without target evidence

## Adoption Gates

| Consumer | Required production cards | Consumer-owned prerequisites |
| --- | --- | --- |
| Nucleus | 087, 088, 089, 092, 093 | browser construction, navigation, capability, and visibility policy mapping |
| Soundcheck | 087, 090, 093 | Signal/plugin/helper lifecycle and authorization remain consumer-owned |
| Jetstream | 087, 088, 091, 092, 093 | WGPU renderer, surface construction, and semantic input mapping remain consumer-owned |
| Loophole | none for current migration | adopt only if a later native-content use is proved |

The production artifact gate now passes. No donor write starts before a
migration card explicitly authorizes that repository and its consumer-owned
prerequisites are mapped.

## Prototype Disposition

Retain `prototypes/native-content*` in place as non-workspace, non-publishable
evidence. Card 093 proves that no prototype package enters a production graph.
The first consumer cutover may later record whether any evidence can be
archived. Do not copy prototype source blindly, publish prototype manifests,
or treat prototype API shape as compatibility authority.

## Artifact And Adoption Checkpoint

Card 093 packs `@inflatable-cookie/longhorn/core`, `@inflatable-cookie/longhorn/native-content`, and
`@inflatable-cookie/longhorn-poodle-svelte/native-content`, inventories the five Rust crates, and runs
four offline Rust consumers plus three isolated renderer consumers. The pure,
Nucleus, Soundcheck, and Jetstream graphs contain exactly their selected
mechanism. Browser, plugin, GPU, Tauri, Svelte, and Poodle edges remain absent
unless selected by that consumer shape.

Rust-produced snapshots and renderer projections match for child view,
isolated window, and backing surface. Nucleus and Jetstream compile the public
Svelte viewport seam against the exact Poodle preview set. Soundcheck resolves
neither Svelte nor Poodle. Capabilities authorize only native-content protocol
read/mutation and required core event listening.

Fresh macOS bundle-path reruns preserve the production evidence: child view
passes seven checks with focus/visibility observation unknown and live scale
switch unavailable; isolated window passes 11/11; backing surface passes 10/11
with only the unavailable live scale transition unmet. Windows/Linux statuses
remain exact and unchanged.

| Consumer | Artifact gate | Next admission | Native-content cutover blocker |
| --- | --- | --- | --- |
| Nucleus | pass | Card 101 complete | none; product policy remains Nucleus-owned |
| Soundcheck | pass | after g01.014 and g01.015 | Signal/plugin ABI and authorization, helper/process ownership, native content and media policy |
| Jetstream | pass | after g01.014 and g01.015 | native storage/WGPU construction, renderer/world/camera/picking/gizmo/frame-loop authority, semantic input mapping |
| Loophole | not applicable | no current native-content migration | none claimed |

Artifact readiness is not donor cutover authority. The exact Nucleus Browser
policy is in the canonical migration map and Card 094 freeze. Card 095 admits
the private graph and both consumer checks. Card 096 completes the bounded
storage slice. Card 097 replaces protected-window mechanics and keeps the
native-content capability unchanged. Cards 098-099 replace project layout
authority and the checked renderer/Poodle edge. Card 100 replaces Browser
mechanics with the production graph without adding Surface state. Card 101
closes conformance with exact artifacts, restart and rollback evidence, and
capability/duplicate-authority audits.

## Production Kernel Checkpoint

Card 087 adds `longhorn-native-content` with only `longhorn-core` and `serde`
as normal dependencies. Shared native-content ids and revisions live in
`longhorn-core`; no second identity grammar entered the production crate.

Production hardens the prototype evidence at the authority seam:

- attach generations and plan steps are nonzero on construction and decode
- one attach generation binds one logical host; host changes advance it
- mechanism capabilities declare the only active input route, with `disabled`
  always available as a gate
- observed detach suppresses repeat detach operations
- host destruction records an idempotent invalidation before late native
  events can enter
- content-size proposals and apply completions must match current generation
  and revisions
- apply completion additionally matches island, desired revision, observed
  revision, and the non-invalidated generation

The public operation vocabulary remains lossless across child bounds,
isolated content size, and backing viewport clip. Backing storage bounds stay
observation evidence and never replace the semantic clip. The full API and
flow are recorded in the
[`longhorn-native-content` README](../../crates/longhorn-native-content/README.md).

## Renderer Protocol Checkpoint

Card 088 keeps Rust as wire authority. Exact-version request/result/event
types cover connection, snapshots, desired updates, observations,
content-size proposals and decisions, apply receipts, and host destruction.
The checked fixture is generated beside TypeScript; drift fails the gate.

`@inflatable-cookie/longhorn/native-content` owns no mechanism. Its root accepts injected direct
or serialized transports. `/tauri` only maps four narrow commands and one
product-neutral event. Connection is listener-first. Each connect issues a
new client epoch while attach generation remains host authority. Independent
revision cursors reject late async results, and pending correlation is finite.

Capability examples authorize protocol reads or coordination mutations. They
do not authorize browser navigation, plugin loading, rendering, or product
content.

## Svelte Lifecycle Checkpoint

Card 092 adds `@inflatable-cookie/longhorn-poodle-svelte/native-content` as a separate package over the
checked client. Each `NativeContentSession` owns one connection per mount and
accepts the current scale plus final visibility, focus, and input-routing
policy. It measures only the exact consumer-supplied viewport element. Host
window, attach generation, rounding, and presence remain checked host state.

Resize and policy changes pass through one serialized pump. Current connection
and generation evidence gate every completion. Stop invalidates pending work,
disconnects the owned observer, disposes the listener-backed connection, and
clears the authoritative projection before remount.

The composition fixture places a consumer-owned viewport inside public Poodle
`Surface` children. The adapter has no Poodle dependency and never queries
classes, ancestors, overlays, device scale, or semantic input events. Child
view and backing surface use the same lifecycle with different explicit input
routing.

## Production Child-view Checkpoint

Card 089 adds `longhorn-tauri-native-content-child-view`. The package depends
only on the pure kernel, `longhorn-core`, `serde`, and Tauri. It isolates
`WebviewBuilder` and `Window::add_child` in one runtime module and wraps the
retained `tauri::Webview` so no raw handle crosses its public protocol.

Construction inputs remain explicit: logical and Tauri labels, source URL,
navigation admission, and optional macOS data-store identity. The built-in
runtime denies popup creation and download persistence. Example capabilities
match only a trusted local controller; the remote child receives none.

The packaged macOS proof passes creation, readiness, fresh 2x bounds,
hide/show reuse, renderer unmount, focus request, injected browser policy,
close, replacement, teardown, and host destruction. Focus and visibility
readback remain `unknown`. Deterministic 1x/2x conversion passes; the one 2x
display could not provide a live scale transition. Windows and Linux remain
unproved.

## Production Isolated-window Checkpoint

Card 090 adds `longhorn-native-content-isolated-window`. Its production graph
depends only on `longhorn-core`, the pure native-content kernel, and `serde`.
Consumers inject process launch, native content creation, transport I/O,
authorization, and safe owner termination. Tauri and the frozen Card 084
process fixture appear only in the packaged proof.

The shared protocol carries generation plus bounded
`NativeContentRequestId` correlation. It contains content-area sizing,
visibility, focus, resize hints, observation, and shutdown. Outer position,
raw handles, plugin ABI, Signal, audio, MIDI, rendering, and semantic input are
absent. Content resize remains a revision-bound proposal; accepted,
constrained, and rejected receipts cannot mutate durable desired state.

The macOS 26.5.2 arm64 package passes the full Card 084 11-check matrix with a
real controlled `NSView`: startup and readiness, host/content resize, echo
suppression, show/hide/focus loss, resize-hint admission, cooperative close,
consumer-owned recentering, timeout, owner termination, helper loss, and stale
generation rejection. Windows and Linux are explicitly unsupported.

## Production Backing-surface Checkpoint

Card 091 adds `longhorn-native-content-backing-surface`. Its production graph
depends only on `longhorn-core`, the pure native-content kernel, and `serde`.
Consumers inject native storage, renderer lifecycle, clipping, observation,
and reversible detach. Tauri, AppKit, raw pointers, and deterministic pixel
production appear only in the packaged proof.

Full-host native storage remains distinct from the physical viewport clip.
Viewport move, resize, collapse, restore, presentation, and input routing do
not silently move or detach storage. Physical input requires presentation,
non-empty clip and storage, containment in both, consumer-supplied host focus,
and `renderer_forwarded` routing before typed semantic dispatch. Host focus is
gate evidence only; native focus and visibility observations remain `unknown`.

Runtime evidence names island, host, attach generation, event sequence, and
renderer frame sequence. Older plans and callbacks leave current state
unchanged. Host destruction invalidates callbacks before reversible detach;
failed detach retains the handle for explicit retry.

The macOS 26.5.2 arm64 package passes 10 checks over a real full-host AppKit
`NSView` and deterministic consumer renderer. The available monitors exposed
no distinct native scale, so live scale transition remains unmet and
unsimulated. Deterministic 1x/2x conversion passes. Windows and Linux are
explicitly unsupported.

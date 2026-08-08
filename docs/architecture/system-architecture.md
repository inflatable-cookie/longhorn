# System Architecture

Status: promoted
Owner: Tom  
Updated: 2026-08-03
Vision: `../vision/001-shared-tauri-systems.md`

## Boundary

Longhorn owns reusable desktop mechanisms and their cross-language contracts.
Consumer apps own product topology, panel catalogues, resource attachments,
workflow state, and presentation composition.

Poodle owns visual primitives including tabs, dock regions, split views,
menus, dialogs, and presentation tokens. Longhorn supplies state machines,
policies, Tauri adapters, and Poodle bindings. It does not copy Poodle
components.

## Composable Hosting Model

Regions belong to an opaque layout container.

```text
display inventory -> window plan
                         |
                         +-> window as layout container -> regions -> panels
                         |
                         +-> hosted surfaces -> surface as layout container
                                                -> regions -> panels
```

This removes Surface from the mandatory core hierarchy:

- Nucleus binds a `WindowId` to a layout-container id.
- Loophole enables the Surface package and binds a `SurfaceId` to a
  layout-container id.
- Panel and region logic does not need to know which hosting choice was made.

## Target Layers

### Foundation model

- opaque stable ids
- logical and physical bounds
- intersection, clamping, and scale conversion
- versioned local-state envelopes
- deterministic normalization

No Tauri, Svelte, Poodle, or product dependency.

### Display inventory

- host monitor probe input
- canonical display records
- current availability
- arrangement signature
- known-display correlation and recovery
- machine and user labels

Tauri monitor APIs are an adapter, not the domain model.

### Window planning and host

- configured target and ordered display fallbacks
- geometry memory per display
- deterministic window plan
- pure live-versus-desired apply plan
- restore, clamp, debounce, and close flush
- user-versus-programmatic move attribution
- dynamic Tauri webview-window adapter

This layer does not depend on Surfaces.

### Layout core

- opaque layout-container and layout-schema ids
- consumer-defined flat region ids, families, order, and empty-region policy
- panel definitions, distinct panel instances, allowed placement, and explicit
  instance policy
- panel ordering and active-panel state
- fixed-point named sizing slots and supported collapse state
- occupancy and transient-reveal visibility projection
- expected-revision mutation and deterministic normalization
- layout-container id as the only parent requirement

The core does not encode a generic split tree. Consumers map named sizing slots
and semantic regions into public Poodle split and dock components. Panel
bodies, labels, and product resource attachments remain consumer-owned.

`longhorn-layout`, `longhorn-layout-config`, `longhorn-bindings`, and
`@inflatable-cookie/longhorn/layout` implement this foundation. Checked Loophole eight-region
and Nucleus five-region fixtures use one public resolver and mutation engine.
Their Surface/window bindings remain outside the layout document.

### Optional Surface hosting

- stable Surface identity and labels
- distinct Surface-to-layout-container bindings
- window hosting preferences and fallback
- ordered hosted Surfaces per window
- active Surface per window
- consumer-resolved presence input
- create, duplicate, close, move, and reorder lifecycle
- expected-revision mutation and registered persistence
- explicit layout-container cleanup intent

Product presence predicates, layout cloning, cleanup execution, window roles,
and product resources remain consumer-owned. Surface, layout, and window
geometry persist as distinct domains. Apps that omit this package carry no
Surface state.

### Local state and persistence

- storage classes for user config, machine state, workspace-local state,
  project-shared state, secrets, cache, and runtime material
- immutable canonical identity with one optional stable human-readable storage
  name, separate from the mutable product display name
- versioned platform-native, unified-root, and explicit portable profiles
- pure config, data, state, cache, log, runtime, and backup root resolution
  from injected platform-directory facts
- platform root facts and profile bootstrap through narrow host adapters
- fixed native locator outside profile-selected configuration authority
- inspectable, journaled profile transition with locator commit last
- declarative read-only legacy-root discovery
- registered versioned domains and ordered migrations
- one injected store-wide local coordinator using a process mutex and stable
  advisory file lock
- capability-confined atomic file replacement and explicit durability receipts
- merge-safe partial updates and coordinated multi-domain load sets
- debounced scheduling and explicit flush
- bounded backup capture, verified archive publication, and retention
- capability-declared custom capture and restore adapters with confined
  payloads, truthful external consistency groups, separately explicit
  nontransactional receipts, and opt-in grouped failure-atomic participation
- optional whole-archive binary age v1 encryption through an injected
  noninteractive provider or explicit recipient/passphrase export
- non-mutating restore inspection, exact conflict planning, current-evidence
  confirmation, and complete private current-schema staging
- verified safety archives, bounded exact rollback, durable restore journal,
  per-file atomic publication, full-set verification, and crash rollback
- grouped custom-adapter planning, complete private target/rollback staging,
  explicit present/absent evidence, durable multi-adapter journalling, exact
  deletion and group rollback, and catalogue-bound boot recovery
- active/recovery-required load states and coordinated multi-domain load-sets
- destructive migration rewrite through the same safety transaction
- corruption, future-schema, backup, restore, and receipt policy

Storage locations are injected into pure code. Product schemas register with
the store but do not become Longhorn types. Secrets use a separate secure-store
adapter. Database placement follows the data lifecycle; database-native
snapshot and migration adapters own live database consistency.

Grouped custom restore remains in `longhorn-config`. It does not create a
Nucleus or SQLite package. The pure transaction owns selection, confirmation,
payload bounds, journal phases, ordering, receipts, and recovery. Registered
adapters own domain staging, apply, and semantic observation. The consumer
must quiesce live authorities and schedule boot-time execution before opening
them.

Layout persistence uses a narrow `longhorn-layout-config` adapter. Consumers
inject the exact descriptor and scope. Layout and window geometry remain
separate domains, so renderer layout updates cannot replace host-owned window
state. The adapter binds the complete layout document to a deterministic
definition-registry digest. A changed digest requires a domain schema bump and
an explicit migration hook. Structural requests publish immediately over fresh
coordinated state; sizing and collapse may use bounded explicit-flush debounce.

### Settings and system registration

- one sealed, generation-stamped registry for modal, window, or panel
  presentation
- app and module registration of stable pages, renderer keys, scopes, apply
  units, keywords, anchors, and composed capabilities
- pure Rust settings authority plus a config adapter that binds one
  failure-atomic apply unit to one registered domain
- immediate or staged mutation timing; restart is a separate activation result
- configured/effective values, managed-policy provenance, editability, and
  recovery state projected by host authority
- deterministic search, structural deep links, dirty navigation guards, and
  explicit conflict/error state
- storage profile, backup, restore, and diagnostics as registered modules over
  the existing configuration receipts
- no empty UI for modules the app did not compose

Poodle supplies the visual primitives. Longhorn supplies registry state,
checked authority, configuration apply units, session state, and Svelte
bindings. Product schemas and specialist page bodies remain in consumers.

The pure `longhorn-settings` slice implements bounded declaration identity,
capability admission, immutable generations, canonical digests, checked
projection, and load/apply/reset protocol types. `longhorn-settings-config`
now adds fresh checked one-domain mutation, managed editability enforcement,
exact conflict and durability evidence, scoped reset, and post-publication
activation. `longhorn-tauri-settings` and `@inflatable-cookie/longhorn/settings` provide the
narrow injected host, checked client, and optional per-instance Svelte and
public-Poodle subpaths. `longhorn-config` projects exact storage and backup
evidence into a renderer-safe protocol. `longhorn-tauri-config` injects
authorization, plan custody, pickers, retention policy, pending flush, and
encryption authority. `@inflatable-cookie/longhorn-poodle-svelte/config/poodle` supplies the optional shared
pages, while `longhorn-settings-config` admits them by capability. Product
pages and schemas remain downstream in consumers.

The canonical bootstrap, mutation, capability, artifact, and migration rules
are in [Settings Composition](settings-composition.md). Card 048 proves modal,
window, and panel hosts across four isolated app shapes.

### Command, action, and input

- one bounded sealed command and context registry
- closed structural v1 arguments, current availability, and fresh execution
  admission
- consumer-injected routes to renderer-local or typed domain operations
- deterministic physical-keyboard resolution over one hot-context path
- immutable presets plus sparse disable, replace, and add overrides
- coordinated config persistence with revision-bound conflict preview
- command palette, menus, shortcuts, help, and keybinding settings as
  projections of the same registry and effective keymap

Consumer commands register through the same seam. Longhorn does not own
product verbs or expose a generic Tauri/bridge execute-by-string bus. V1 is
single-chord keyboard input. Macros, multi-stroke input, extended trigger
families, native accelerators, and automation remain deferred.

### History

- generic typed payload with consumer-owned inverse, coalesce, and no-op policy
- record-after-product-success
- revision-bound plan, atomic apply, and checked commit navigation
- compounds, explicit gesture groups, bounded count and encoded weight
- versioned structural persistence and committed transition receipts
- authoritative metadata pages for past, current, and future entries
- optional checked TypeScript, Tauri, Svelte, and public-Poodle edges
- optional production fork-tree foundation with immutable nodes, stable branch
  refs, checked topology, and lossless divergent record

`longhorn-history` owns structural state. Consumers own payload meaning,
product authorization, atomic model mutation, labels, canonical snapshots,
storage paths, checkpointing, and recovery policy.

Loophole retains its 83-variant Pulse mutation enum, runtime apply match,
tempo/cache reconciliation, project snapshot and version lineage, autosave,
and session-journal file policy. A committed-transition seam preserves its
cross-session undo and crash-recovery flow. Linear mode retains current
successful behavior while replacing move-before-apply and renderer-remembered
redo with checked shared semantics.

The implemented renderer edge is metadata-only: a strict generated protocol,
framework-neutral checked clients, caller-aware Tauri assembly, listener-first
per-instance Svelte state, and one controlled public-Poodle panel. Authority
epoch and history revision invalidate stale work. Events are non-durable
refresh hints, never history authority.

Minimal and Loophole-shaped produced-artifact compositions, recovery rules,
and later donor-cutover gates are in
[History Composition](history-composition.md).

Forkable undo is not live donor behavior. Card 068 proves divergent retention,
stable first-class branch refs, lowest-common-ancestor checkout, protected
pruning, opaque checkpoints, independent migration, and credible document and
Loophole-shaped costs. Card 069 promotes those semantics. Card 070 implements
the production tree identity, topology, branch refs, and divergent-record
foundation. Card 071 adds bounded mixed LCA navigation through one atomic
consumer transaction, current/named/pinned protection, deterministic leaf
pruning, and opaque checkpoint replay accounting. Card 072 adds a strict
dense graph envelope, independent structural and payload migration, explicit
byte bounds, and complete pre-admission validation.

The tree layer composes downward over `longhorn-history`; it does not replace
the linear authority or enter minimal dependency graphs. Immutable nodes own
one payload copy. Stable branch refs own heads and bounded mutable metadata.
Derived paths are bounded opt-in projections, not identity. The default client
projection remains linear. Production persistence must use a dense payload
representation rather than the prototype's expanded JSON byte arrays.

`longhorn-history-tree` now exists as an optional private-workspace production
package through bounded clients. The default renderer load is one linear path;
branch metadata and alternate branch paths are explicit hard-bounded queries.
The caller-aware Tauri host, checked direct/serialized/Tauri clients,
per-instance Svelte state, and controlled public-Poodle panel contain no
product payload. Artifact compatibility remains unproved. Loophole keeps
branch mode disabled. Undo branches remain distinct from project versions,
collaboration, merge, and event sourcing.

### Async operations and notifications

Soundcheck plugin scan and Loophole render queue prove one optional operation
authority. `longhorn-operation` owns stable identity, revisioned queued,
running, cancelling, and sticky terminal transitions, bounded progress,
cancellation request receipts, explicit retention, teardown, and
current/recent projections. Consumers own admission, scheduling, execution,
product progress, reports, artifacts, retry policy, persistence, and recovery.

Cards 075-076 implement the pure authority: bounded identity and progress,
distinct revisions, finite registration, exact lifecycle transitions,
receipted cancellation, count/weight retention, retry lineage, dismissal, and
controlled teardown. Card 077 adds the Rust-generated payload-free protocol,
framework-neutral checked client, direct and serialized adapters, injected
Tauri authority/executor assembly, and optional bridge-domain correlation.
Card 078 adds a framework-neutral presentation controller, per-instance
Svelte sessions, request-keyed cancellation and dismissal, and a controlled
public-Poodle panel. Soundcheck scan and Loophole queue shapes share the same
controller. Product detail stays injected.

Cancellation acceptance is not terminal cancellation. The executor may still
report success or failure when it wins the race. Retry creates a new operation.
Renderer teardown removes listeners and never cancels host work implicitly.

Notifications form a separate optional authority. `longhorn-notifications`
owns a finite retained ledger with explicit seen, dismiss, replace, and prune
transitions. A consumer projector may observe selected operation terminal
transitions, but non-operation domain events use the same ledger and operation
state never depends on notification publication.

Card 079 implements that root over `longhorn-core` alone. Adds always create a
fresh identity; replacement requires an explicit source/key pair; producer
tokens provide separate durable idempotency. Standard records prune oldest
first under count and canonical encoded-weight limits. Consumer-selected
protected records make an unsatisfiable mutation reject atomically. The
feature-gated operation observer receives only an immutable committed record
and transition receipt.

Card 080 adds the Rust-generated exact-v1 protocol, direct/serialized/Tauri
clients, app-wide invalidation hints, listener-first gap/epoch refresh, and
bounded renderer paging. Svelte sessions are per instance. The retained panel
survives remount by reloading host truth; transient toasts do not replay on an
initial snapshot. Both `ToastHost` and controlled `ToastStack` integrations use
public Poodle exports. Every semantic-action click calls a consumer-injected
executor for fresh admission; Longhorn never executes an unchecked string.

Transient toasts are renderer projections. Toast expiry does not dismiss a
retained record. Poodle owns `Progress`, `StatusIndicator`, `ToastHost`, and
`ToastStack`; Longhorn supplies checked state and public-component adapters.
Native OS delivery remains a later privacy, permission, and host-policy edge.

### TypeScript and Svelte

- typed snapshots and commands derived from or checked against Rust authority
- framework-neutral placement and drag helpers
- Svelte stores/actions for subscriptions and command dispatch
- Poodle `Tabs`, `DockRegion`, and `SplitView` adapters
- safe custom-titlebar drag helper
- hidden compatible-region reveal during panel drag

Renderer state stays transient. Durable placement changes return through the
host-authoritative snapshot.

### Tauri bridge

- Rust-owned domain contract registration through generated checked clients
- structural bridge negotiation, correlation, coded errors, retry classes, and
  authority descriptors
- listener lifetime and current-snapshot handshake
- testable command handler assembly
- primary-window coordinator identity
- capability examples for dynamic windows

The implemented window-host assembly and capability posture are in the
[Tauri Window Host Integration guide](tauri-window-host-integration.md).

`longhorn-bridge` now implements the pure exact-v1 negotiation substrate:
bounded bridge, host, session, domain, capability, scope, feature, and
diagnostic identity; checked connection state/reason pairs; five host forms;
authentication posture; separate capability and per-domain authority
descriptors; nonzero authority epochs; and optional authoritative revision
evidence. It imports no Tauri, async runtime, network, renderer, or consumer.

The same pure crate now adds generic query, command, and cancellation
envelopes; typed success, rejection, and indeterminate terminals; coded
failure phase, retry, message, and details; bounded no-eviction deduplication
evidence; session/epoch/sequence stream tracking; and request-correlated
progress, cancellation, and terminal job metadata. Domain routes and payload
types remain external generic parameters.

`longhorn-bindings` generates that protocol and a Rust-owned semantic fixture.
`@inflatable-cookie/longhorn/bridge` validates exact compatibility before exposing a session,
projects capability and authority through injected domain codecs, and runs one
host router through direct or deterministic JSON-loopback adapters. Its
optional stream subpath preserves listener-before-snapshot ordering and exact
late-registration disposal. Query-only root imports include no event, Tauri,
service, Svelte, Poodle, or consumer runtime.

`longhorn-tauri-bridge` now registers typed domain handlers behind stable
generic Tauri commands. It checks caller session, domain metadata, capability,
authority posture, and authority epoch before dispatch. `@inflatable-cookie/longhorn-tauri`
keeps its root invoke-only; optional events live at `/events`.
`@inflatable-cookie/longhorn-tauri/bridge` composes the checked client over that root, while
`/tauri-events` adds checked listener-first resync and correlated job events.
Tauri capabilities admit command reachability but never grant domain
authority.

The pure connection machine now receipts every admitted transition. A
successful checked negotiation becomes ready only after consumer-declared
domain authority requirements pass. Reconnect clears the current session and
authority map before scheduling injected bounded backoff; old sessions and
old or future unnegotiated authority epochs are classified explicitly.
Transport re-entry before the scheduled deadline is rejected.
Queries use a separate bounded retry controller. Command transport failure
continues through the durable-idempotency and advertised-deduplication gate;
uncertain writes outside that gate are indeterminate.

Optional Rust supervision is feature-gated. The TypeScript implementation is
available only at `@inflatable-cookie/longhorn/bridge/supervision`. Both accept consumer-injected
spawn/attach/readiness/restart/reconnect/shutdown observations, expose stable
receipts, and permit restart or shutdown only for owned local services.
Credential input is an opaque secure-store reference; arbitrary adapter
failure text is not admitted or propagated.

Five clean artifact-installed consumers now prove query-only, ordered
snapshot, correlated-job, embedded/optional-host, and local-first/remote-attach
shapes. Separate offline Rust consumers prove that supervision and Tauri stay
removable graph edges. Composition, migration, and production limits are in
the [Bridge Topology Composition guide](bridge-topology-composition.md).

The bridge must not become a product command bus.

### Optional backend topology

- transport-independent domain command, query, cancellation, snapshot, event,
  and error contracts
- direct and Tauri execution plus serialized-loopback conformance
- injected service transport and supervisor ports without a selected
  production network protocol
- exact-version negotiation, host/session identity, capability advertisement,
  and separate per-domain authority descriptors
- explicit connecting, negotiating, ready, degraded, reconnecting, offline,
  incompatible, unauthorized, failed, and closed state
- one declared write authority per domain and authority scope
- retry only for queries under adapter policy or commands with durable
  idempotency plus advertised deduplication
- no generic offline mutation queue; offline caches remain projections

Host form changes transport and lifecycle ownership, not domain semantics.
Consumers own service acquisition, installation, update policy, endpoint
selection, and remote lifecycle. Longhorn may project injected supervision,
readiness, reconnect, and shutdown state.

Local configuration, windowing, and layout do not require a service.

### Native content islands

- optional coordination across host-owned child webviews, isolated native
  windows, and embedded backing surfaces
- opaque island identity, host binding, attach generation, revisions, desired
  and observed state, and exact apply/teardown receipts
- one host-local `ClientCssPx` presentation and interaction viewport with
  explicit `ScaleFactor`
- explicit presence, visibility, focus intent, and input-routing mode

The desired viewport has mechanism-specific effect. A child view moves and
resizes to it. An isolated window treats it as content size and delegates outer
placement to windowing. A backing surface may fill the host while clipping
rendering and forwarded input to it.

Child-webview creation and security, plugin ABI and helper isolation, and GPU
surface/render ownership remain separate adapters or consumer ports. Browser,
plugin, render, pointer, and MIDI payloads do not enter the shared protocol.
Poodle retains layout and overlay presentation.

Visibility policy is explicit. Longhorn does not inspect DOM overlays or infer
occlusion from focus or time. Unknown native visibility stays unknown unless a
platform adapter supplies stronger evidence. The common coordination boundary
is promoted through contract 017. Cards 082-085 prove the private pure model
and three independently packaged macOS mechanisms. Card 086 promotes the split
production graph: pure kernel, three opt-in mechanism layers, checked
TypeScript, and per-instance Svelte support. Poodle remains a consumer
composition seam. Initial native-host claims are macOS-only and preserve
explicit unknown observations. Production artifacts and donor cutover remain
gated by g01.018. See
[Native-content Island Composition](native-content-island-composition.md).

## Drag And Drop

- Same-webview tab and region movement uses Poodle's HTML5 payload contract.
- Cross-webview/window movement uses bounded host-created single-use sessions
  and complete replacement drop-zone leases in `ScreenDip`.
- Drag payloads contain only protocol version and an unguessable session id.
- Session ids contain exactly 128 injected entropy bits. Monotonic time and
  entropy have no ambient implementation in the transfer session core.
- One current renderer client epoch owns each window lease. Epoch advance,
  window destroy, expiry, and host shutdown invalidate process-local authority.
- The Rust host re-resolves source, target, revision, window presence, and
  eligibility before commit.
- First-line panel transfer supports move within one registered layout
  document. Cross-document and copy transfer fail before mutation.
- Direct-window and Surface-container hosts project through fresh opaque
  bindings. The same expected-revision `MovePanel` path owns publication.
- Whole-Surface transfer mutates one Surface document and retains its layout
  binding.
- Empty-display window creation is explicit consumer policy with provision and
  cleanup receipts.
- Overlapping eligible targets abort as ambiguous; enumeration order never
  chooses.
- Pointer-math editing gestures remain consumer or specialist-library work.

## Authority

- machine adapter observes displays and native windows
- Longhorn resolves generic placement and lifecycle rules
- consumer host owns persistence location and product policy
- consumer renderer owns presentation and transient interaction
- Poodle owns component behavior and visual semantics

## Package Shape

`package-topology.md` is canonical. It defines narrow Rust domain and host
crates, owning TypeScript domain packages, and separate Svelte/Poodle adapters.
No g01 umbrella package or empty optional-package placeholders are allowed.

## Validation Strategy

- pure Rust fixtures for display/window/layout resolution
- serialization fixtures shared across Rust and TypeScript
- Tauri mock-runtime command tests
- Vitest coverage for drag, reorder, subscription, and adapter logic
- consumer conformance fixtures from Loophole and Nucleus
- packaged-app proofs for native window and cross-window behavior
- failure-injection fixtures for config writes, backup, migration, and restore
- direct-versus-serialized backend adapter conformance
- consumer-neutral history apply/failure fixtures
- Soundcheck-scan versus Loophole-render operation fixtures
- retained-ledger versus transient-toast notification fixtures

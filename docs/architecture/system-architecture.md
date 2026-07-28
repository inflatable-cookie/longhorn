# System Architecture

Status: promoted first pass  
Owner: Tom  
Updated: 2026-07-28
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

- consumer-defined region ids and families
- panel ids, definitions, allowed regions, and instance policy
- panel ordering and active-panel state
- region sizing, collapse, visibility, and normalization
- mutation commands and deterministic resolution
- layout-container id as the only parent requirement

Panel bodies and panel resource attachments remain consumer-owned.

### Optional Surface hosting

- stable Surface identity and labels
- window hosting preferences and fallback
- ordered hosted Surfaces per window
- active Surface per window
- presence gates
- create, duplicate, close, move, and reorder lifecycle
- focused-panel and regional habitat policy

This is a separate package/module. Apps that omit it carry no Surface state.

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
- merge-safe partial updates and cross-domain transactions
- debounced scheduling and explicit flush
- bounded backup capture, verified archive publication, and retention
- capability-declared custom capture and restore adapters with confined
  payloads, truthful external consistency groups, and separately explicit
  nontransactional receipts
- optional whole-archive binary age v1 encryption through an injected
  noninteractive provider or explicit recipient/passphrase export
- non-mutating restore inspection, exact conflict planning, current-evidence
  confirmation, and complete private current-schema staging
- verified safety archives, bounded exact rollback, durable restore journal,
  per-file atomic publication, full-set verification, and crash rollback
- active/recovery-required load states and coordinated multi-domain load-sets
- destructive migration rewrite through the same safety transaction
- corruption, future-schema, backup, restore, and receipt policy

Storage locations are injected into pure code. Product schemas register with
the store but do not become Longhorn types. Secrets use a separate secure-store
adapter. Database placement follows the data lifecycle; database-native
snapshot and migration adapters own live database consistency.

### Settings and system registration

- one registry-driven shell for modal, window, or panel presentation
- app and module registration of pages, keywords, scopes, and capabilities
- staged/immediate application, validation, dirty state, reset, and deep links
- backup, restore, and storage diagnostics as registered pages
- no empty UI for modules the app did not compose

Poodle supplies the visual primitives. Longhorn supplies registry state,
configuration transactions, and Svelte bindings.

### Command, action, and input

- one namespaced command catalogue
- typed arguments, context, capabilities, availability, and execution routes
- deterministic input resolution, keymaps, sparse overrides, and conflicts
- command palette, menus, keybinding settings, and automation as projections
  of the same registry
- optional macros that still pass command validation

Consumer commands register through the same seam. Longhorn does not own
product verbs.

### History

- generic linear navigation mechanics
- inverse/apply adapter seam
- compound and gesture grouping
- coalescing policy, limits, persistence, and UI projections
- optional branch-tree research lane

Loophole's Pulse mutations and runtime apply logic remain app-specific.
Branching is not part of the current extraction claim.

### Async operations and notifications

- operation ids, state transitions, progress, cancellation, and terminal
  outcomes
- listener/subscription lifetime and stale-request protection
- bounded notification records and presentation projections

These remain incubating until a second consumer confirms the common shape.

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

- command/event contract registration
- listener lifetime and current-snapshot handshake
- testable command handler assembly
- primary-window coordinator identity
- capability examples for dynamic windows

The bridge must not become a product command bus.

### Optional backend topology

- transport-independent domain command/query/event contracts
- direct, Tauri-local, local-service, remote, and local-first adapters
- capability and version negotiation
- explicit readiness, failure, reconnect, and offline state
- one declared write authority per domain

Local configuration, windowing, and layout do not require a service.

### Native content islands

- host-owned child webviews, isolated native windows, or embedded render views
- checked geometry and input forwarding seams
- lifecycle and occlusion/visibility coordination

The current app implementations are too different for a promoted common API.
This layer remains prototype-first.

## Drag And Drop

- Same-webview tab and region movement uses Poodle's HTML5 payload contract.
- Cross-webview/window movement uses host-created id-only sessions and leased
  drop zones in `ScreenDip`, never serialized model state.
- The Rust host re-resolves source, target, revision, and eligibility before
  one transactional commit.
- Empty-display window creation is explicit consumer policy.
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

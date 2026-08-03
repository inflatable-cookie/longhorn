# 001 Shared Desktop System Suite

Status: active  
Owner: Tom  
Updated: 2026-08-03
Evidence: `../research/translation-memos/001-tauri-application-extraction-audit.md`,
`../research/translation-memos/002-shared-desktop-systems-follow-up.md`,
`../research/translation-memos/015-history-kernel-and-fork-boundary.md`,
`../research/translation-memos/016-async-operation-and-notification-boundary.md`,
`../research/translation-memos/017-native-content-island-boundary.md`

## Purpose

Keep the complete candidate suite and its maturity in one durable place.
Architecture and contracts own promoted behavior. This spec owns unresolved
shape and admission questions.

## Suite

| Capability | Intended reusable unit | State | Main evidence |
| --- | --- | --- | --- |
| configuration storage | Rust domain stores, roots, migrations, safe writes | promoted first pass | Loophole, Soundcheck, Bovine, Nucleus |
| backup and recovery | inventory, snapshot, verify, rotate, restore | promoted first pass | cross-app need; partial Loophole evidence |
| display inventory | pure model plus Tauri probe adapter | contract 009 | Loophole, Nucleus, Soundcheck |
| geometry/window planning | pure planner plus native host | contract 009 | Loophole, Nucleus, Soundcheck |
| lifecycle and app chrome | small Tauri/Svelte helpers and guidance | provisional | all five apps |
| layout containers | Rust/TS region and panel state | promoted first pass | Loophole, Nucleus |
| hosted Surfaces | optional layout-container host | promoted first pass | Loophole |
| cross-window drag | id/coordinate protocol and host transaction | contract 011 | Loophole, Nucleus |
| Svelte/Poodle bindings | stores, actions, adapters; no component fork | architecture | Loophole, Nucleus |
| settings | registry-driven dialog shell and domain pages | promoted first pass | Loophole plus cross-app demand |
| typed bridge | checked commands, events, errors, subscriptions | contract 010 | all five apps |
| optional backend topology | local, embedded, or remote authority adapters | promoted first pass | Nucleus, Loophole |
| commands/actions | sealed registry, bounded arguments, fresh admission, injected execution | compiled boundary | Loophole, Jetstream |
| input/keymaps | physical keyboard, contexts, conflicts, durable sparse overrides | compiled boundary | Loophole, Jetstream |
| command palette | Poodle-backed projection of command registry | compiled boundary | Loophole, Jetstream |
| history | typed linear kernel plus later optional fork-tree layer | promoted fork semantics; tree implementation planned | Loophole plus non-editor fixture |
| async operations | finite lifecycle authority, progress, cancellation receipts, retention | lifecycle foundation implemented | Soundcheck scan, Loophole render queue |
| notifications | independent retained ledger plus transient projection | compiled boundary | Loophole domain records; Soundcheck outcome need |
| native content islands | pure coordination plus separate host mechanisms | promoted split graph; production runway compiled | Nucleus, Soundcheck, Jetstream |
| greenfield starter | composition examples and release guidance | runway | all consumers |

## Cross-System Shape

- Rust owns durable state, validation, mutations, and host authority.
- TypeScript owns checked projections and framework-neutral interaction.
- Svelte owns subscriptions and presentation composition.
- Poodle owns visual primitives.
- Apps register product definitions and policy.
- Optional systems depend downward; the foundation never depends on Surface,
  Poodle, a specific server, or a product domain.

## Remaining Configuration Question

Local domain registration, transaction boundaries, multi-process coordination,
backup/encryption, storage profiles, and managed-policy projection are promoted
through contracts 004 and 005. Server-synchronized configuration and remote
conflict semantics remain deferred after g01.009; no production authority has
proved that contract.

## History Decisions

- The Rust payload is consumer-owned and typed.
- Consumers inject inverse, coalesce, and no-op policy.
- Navigation uses revision-bound plan, atomic product apply, then checked
  commit. Failure leaves model and history unchanged.
- Longhorn owns structural envelopes. Consumers own payload codecs, model
  snapshots, storage, journals, checkpoints, and recovery choices.
- Clients receive authoritative metadata pages, not product payloads or
  renderer-reconstructed redo entries.
- Linear mode is public first and remains the current compatibility promise.
- Card 068 proves stable branch refs, divergent retention, atomic LCA checkout,
  pruning, checkpoints, migration, performance, and linear-default UI.
- Card 069 promotes those semantics. Card 070 implements the optional
  production Rust graph foundation without publishing the prototype or making
  a compatibility claim.
- Card 071 implements bounded atomic LCA navigation, current/named/pinned
  protection, deterministic leaf pruning, and opaque checkpoint replay cost.
- Alternate projections must be bounded and lazy or paged. Production
  persistence must avoid expanded JSON numeric byte arrays.
- Undo branches remain distinct from project versions, collaboration, and
  event sourcing.

## Promotion Rules

- No roadmap may treat an open question as implied behavior.
- A generic system needs two materially different fixtures or one strong
  greenfield invariant before implementation.
- A Poodle component remains in Poodle. Longhorn may supply its state and
  registration model.
- An optional system stays a separate dependency and feature boundary.
- A donor implementation is evidence, not the public API.

## Operation And Notification Decisions

- Operation authority owns identity, transitions, progress bounds,
  cancellation receipts, retention, and projections; consumers own scheduling
  and execution.
- Cancellation acceptance does not promise terminal cancellation.
- Retry creates a new operation and never reopens a terminal record.
- Product phases, warnings, reports, logs, artifacts, and approval workflows
  stay outside the generic protocol.
- Notifications are retained user records, not operation states.
- Selected terminal outcomes may produce notifications through consumer
  policy; publication failure cannot alter the operation outcome.
- Seen, dismissed, pruned, and toast-expired are distinct.
- Poodle owns feedback visuals and toast timers. Native OS delivery remains a
  later adapter contract.

## Native Content Island Decisions

- Share opaque island identity, attach generations, typed desired/observed
  viewport state, visibility, focus intent, input-routing mode, and receipts.
- Treat the viewport as presentation and interaction geometry. It may drive a
  child frame, isolated-window content size, or backing-surface clip.
- Keep child-webview, isolated native-window, and backing-surface mechanisms
  separate and independently selectable.
- Keep browser policy in Nucleus, plugin ABI and process isolation in
  Soundcheck/Signal, and GPU/render authority in Jetstream.
- Use explicit scale evidence. Preserve unknown when portable occlusion cannot
  be observed.
- Card 082 proves the product-neutral pure model across all three donor shapes
  without Tauri, host mechanism code, donor payloads, or raw native handles.
- Cards 083-085 prove three isolated packaged macOS mechanisms. Card 086
  promotes the pure kernel, separate mechanism layers, checked client, and
  Svelte lifecycle into g01.018.
- Initial native-host support is macOS-only. Child-view Windows/Linux are
  unproved; isolated-window and backing-surface Windows/Linux are unsupported.
- Live native scale switching remains unproved for child-view and
  backing-surface. Deterministic 1x/2x conversion is proved.
- Retain private prototypes as evidence. Do not publish them, treat them as
  production source, or migrate donors before g01.018 artifact proof.

## Rejected Shapes

- one all-app framework
- one `settings.json` containing unrelated domains
- configuration files in the current working directory by default
- secrets mixed into ordinary config or backups
- renderer-authoritative durable state
- Surface as a mandatory parent of regions
- palette definitions separate from the command registry
- claiming branch history from a linear undo/redo donor
- cancellation receipts treated as terminal outcomes
- one shared state machine for operations and notifications
- toast expiry treated as retained-record dismissal
- one universal browser/plugin/GPU native host trait
- assuming the desired content viewport is always a native child frame

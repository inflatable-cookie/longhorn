# 008 History Kernel Boundary

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-03
Evidence: `../research/translation-memos/015-history-kernel-and-fork-boundary.md`

## Boundary

Longhorn may provide an optional generic history kernel. It owns structural
history state and checked transitions. Consumers own payload meaning, product
mutation, canonical model state, and recovery policy.

The first compatibility-proved mode is linear. Card 070 implements the pure
graph foundation of the optional production tree layer. Navigation,
persistence, clients, artifact proof, and any release claim remain later work.

## Package Shape

- `longhorn-history`: pure generic entries, policy seams, linear state,
  navigation plans, persistence envelopes, projections, and receipts
- `longhorn-history-tree`: optional immutable-node graph, stable branch refs,
  canonical child indexes, checked structural import, and divergent record
- `longhorn-tauri-history`: optional narrow handler assembly over an injected
  history authority
- `@longhorn/history`: checked metadata client
- `@longhorn/history/svelte`: optional per-instance reactive state
- `@longhorn/history/poodle`: optional linear panel composition over public
  Poodle primitives

The linear crate depends only on `longhorn-core` plus bounded serialization.
It imports no config, bridge, Tauri, async runtime, Svelte, Poodle, or consumer
package.

The tree crate depends downward on the proven linear entry and sequence types.
It does not replace linear authority. No tree renderer package or
compatibility-proved tree artifact exists yet.

## Authority

Longhorn owns:

- bounded entry, group, history, and plan identities
- history-state revision
- linear past/current/future topology
- retention and deterministic projection
- structural persistence compatibility
- checked plan and commit transitions

Consumers own:

- typed payload enum or struct
- inverse, coalesce, and no-op policy
- domain validation and authorization
- atomic payload application and rollback
- labels, kind mapping, and product metadata
- product-model revision and canonical snapshot
- storage location, save, autosave, journal, checkpoint, and replay policy

Renderer state is a projection. Tauri capability reachability is not product
authorization.

## Entry Model

Each entry carries:

- stable opaque entry id from an injected source
- bounded label
- optional bounded kind id
- monotonic sequence
- committed history revision
- typed consumer payload
- optional explicit group identity

Wall-clock time is optional presentation evidence from an injected source. It
never orders or authorizes history.

History revision, product-model revision, bridge authority revision, and
configuration revision are distinct types.

## Payload Policy

The consumer supplies pure policy operations:

- produce the inverse payload or reject
- classify a payload as no-op
- coalesce adjacent compatible payloads into merge, removal, or no-merge

Longhorn defines when policy runs and validates its structural result.
Coalescing cannot cross a navigation boundary, a committed group boundary, or
different history authority.

Arbitrary untyped JSON is not the public Rust payload contract. Serialized
payload bytes exist only behind a registered consumer codec family and
version.

## Recording

A product mutation applies successfully before the history record commits.
Record admission checks:

- current history revision
- payload policy result
- identity and metadata limits
- entry and encoded-weight limits

In linear mode, a committed record after undo clears the redo path. The
transition receipt lists every added, replaced, removed, or pruned entry.

No failed or rejected product mutation records history.

## Navigation Protocol

Undo, redo, and checkout use plan, apply, commit:

1. plan from one exact history revision
2. return the target plus ordered inverse and forward payloads
3. apply the complete batch through the consumer transaction
4. commit the unchanged plan against the same history revision
5. publish one authoritative receipt and projection

A stale plan rejects before commit. Apply failure leaves history position and
revision unchanged. Commit cannot run after a failed or partial apply.

Entry-id checkout is canonical. Indexes are presentation-local and cannot
authorize a mutation.

## Compound And Multi-entry Atomicity

A compound is one typed payload or one atomic consumer batch. Undo applies its
parts in reverse semantic order.

A consumer may expose multi-entry checkout only when it provides:

- atomic batch application, or
- complete rollback with verified restoration

Partial compound or checkout success is forbidden. Longhorn does not infer a
transaction from a sequence of fallible calls.

## Grouping And Coalescing

Grouping is explicit:

- caller-provided group token, or
- injected monotonic clock plus consumer duration and group key

No ambient clock exists in the pure core. The public contract does not fix a
750 ms duration.

Group close, timeout, navigation, authority replacement, and host teardown end
the active group. Persistence restores committed entries, not an open gesture.

## Limits And Pruning

Linear history supports finite:

- entry count
- encoded payload weight
- label and kind lengths
- projection page size
- navigation batch size

Limit changes return exact pruning receipts. The current applied state remains
valid when old entries prune; the new baseline position is explicit. Integer
overflow and impossible limits fail closed.

## Projections

The authoritative projection exposes:

- history revision and mode
- undo and redo depth
- current entry id
- next undo and redo labels
- bounded entry pages with past, current, or future position
- truncation and retained-baseline evidence

Clients do not reconstruct future entries from renderer memory. A projection
gap or newer authority epoch requires a fresh snapshot under contracts 007 and
010.

## Persistence

The structural envelope stamps:

- format family and structural version
- payload codec family and version
- history mode and revision
- limits and retained baseline
- entries and current position

Load validates all bounds, identities, topology, policy compatibility, and
payload decoding before acceptance. Future structural or payload versions
reject explicitly.

A consumer migration may choose a visible discard-history outcome. Silent
fallback to empty history is forbidden.

Persisted history compatibility is separate from canonical product-state
compatibility. Loading entries never replays them into the canonical snapshot.

## Committed Transition Stream

Every committed record, coalesce, navigation, limit change, prune, import, and
reset emits one structural transition receipt.

An optional consumer journal may combine that receipt with:

- typed payload
- product-model revision
- checkpoint lineage
- consumer durability evidence

The kernel owns no path, file format, fsync cadence, snapshot cadence,
autosave, crash-recovery choice, or replay acceptance. Journal failure is
reported separately from a committed in-memory transition.

## TypeScript, Svelte, Poodle, And Tauri

Generated TypeScript contains metadata projections, commands, receipts, and
errors. Generic renderer messages never contain the product payload.

The framework-neutral client supports injected direct, bridge-domain, or
Tauri transport. `/svelte` owns per-instance lifecycle and stale-result
rejection. `/poodle` uses public controlled Button, list, filter, status, and
dialog primitives; it does not fork a visual component.

The optional Tauri adapter exposes only registered history authorities and
checks caller capability before dispatch. The consumer authority still owns
product authorization and the atomic apply transaction.

Card 066 implements this edge as a strict version-1 metadata protocol,
framework-neutral direct and serialized clients, caller-aware Tauri commands,
listener-first per-instance Svelte state, and a controlled public-Poodle
panel. Authority epoch plus history revision invalidate stale pages and
navigation. Live events remain non-durable refresh hints; publication failure
cannot disguise an already committed navigation result.

## Promoted Fork-tree Semantics

Card 068 proves and Card 069 accepts:

- immutable single-parent entry nodes
- stable branch and current-node identity
- divergent record after undo preserves the former future
- deterministic preferred redo child
- checkout through lowest common ancestor
- atomic navigation failure invariance
- named and pinned retention
- bounded entry-count and encoded-weight pruning
- opaque checkpoints and replay-cost accounting
- structural and payload migration
- linear-default projection with optional alternate-path metadata

First-class branch references are structural authority. Derived root-to-leaf
paths are optional read models: they have no stable identity and cannot own
names, pinning, selection, or retention policy.

A production tree layer must preserve these rules:

- immutable single-parent nodes hold the only payload copy
- stable injected branch ids point to mutable heads
- branch name, annotation, and pin metadata stay outside nodes
- divergent record preserves the prior future and creates or advances one
  explicit branch reference
- preferred redo is deterministic and changes only through committed record
  or navigation
- checkout plans inverse steps to the lowest common ancestor, then forward
  steps to the target, through one atomic consumer transaction
- current, named, and pinned lineage is protected from pruning
- count and exact encoded-weight pruning terminates or returns an impossible
  protected-budget result without mutation
- checkpoints contain bounded opaque consumer references, not snapshot data
- structural and payload versions migrate independently and reject future or
  corrupt input visibly
- the default projection remains one linear past/current/future path
- alternate projections are opt-in, bounded, and lazy or paged

The production persistence format must avoid the prototype's JSON numeric-byte
array expansion. The implementation lane must prove a dense payload encoding,
strict topology validation, and deterministic encode/load behavior.

The package boundary is optional and downward-only. `longhorn-history-tree`
depends on `longhorn-history`; the linear crate does not depend on tree state.
Renderer and Poodle edges remain metadata-only and optional.

Until Card 074 artifact proof:

- no released tree package or compatibility promise is claimed
- linear mode remains the only compatibility promise
- Loophole migration does not depend on branching
- project versions, collaboration, and event sourcing stay separate

The Card 068 prototype remains executable research evidence. It is not a
release package or a donor dependency.

## Loophole Admission

The migration fixture must retain:

- current successful undo, redo, record, automatic coalesce, limit, and
  checkout behavior
- exact typed inverse behavior through a Loophole-owned adapter
- persisted cross-session undo
- mutation, undo, and redo journal integration
- recovery from checkpoint plus valid journal suffix
- current history labels and panel capability
- branch mode disabled

The 83-variant Pulse mutation vocabulary, tempo/cache reconciliation, runtime
apply match, project version lineage, autosave, and journal file policy remain
in Loophole.

The donor currently mutates stack position before a fallible apply and exposes
only eight applied entries to the renderer. Longhorn must improve those seams;
they are evidence gaps, not parity requirements.

## Acceptance For Linear Kernel

- arbitrary typed consumer payload through pure policy hooks
- record, inverse, compound, coalesce, explicit group, limit, undo, redo, and
  entry-id checkout fixtures
- stale or failed apply cannot change history position or revision
- compound and multi-entry failure restore the exact prior model and history
- versioned envelope rejects future and corrupt payloads visibly
- committed transition records can drive a Loophole-shaped journal adapter
- authoritative projection includes real past and future entries
- Loophole-shaped fixture retains every claimed live mechanic
- a non-editor fixture proves the abstraction
- root packages remain free of optional host and UI dependencies

## Stop Conditions

- product payload meaning enters Longhorn
- renderer state becomes durable history authority
- a consumer cannot provide atomic apply or verified rollback
- persisted compatibility needs a silent empty-history fallback
- branch prototype changes the public linear contract before promotion
- undo branching becomes project versioning, collaboration, or event sourcing

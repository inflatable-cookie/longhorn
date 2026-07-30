# 015 History Kernel And Fork Boundary

Status: complete and promoted  
Owner: Tom  
Updated: 2026-07-30  
Promotes: `../../contracts/008-history-kernel-boundary.md`

## Prompt

Revalidate Loophole's current history implementation. Extract a general-purpose
boundary without reducing Loophole's undo, persistence, recovery, or history
panel behavior. Decide what evidence exists for forkable history before
compiling g01.011.

## Sources

Read-only Loophole audit at
`da08b50e7cc69b7d13636c94fc571a64db4ae8ca`:

- `pulse/crates/pulse-history/src/lib.rs`
- `pulse/src/history_apply.rs`
- `pulse/src/dispatch/history.rs`
- `pulse/src/session_journal.rs`
- `pulse/crates/pulse-persistence/src/lib.rs`
- `pulse/crates/pulse-persistence/src/journal.rs`
- `pulse/crates/pulse-project-model/src/history.rs`
- `aura/src-tauri/src/services/pulse_authority.rs`
- `aura/src/renderer/workspace/HistoryPanel.svelte`
- `aura/src/renderer/workspace/history-entries.ts`
- `chorus/architecture/decisions/adr-002-persistence-model.md`
- `chorus/architecture/a07-project-versions-and-variants.md`
- `chorus/specs/ipc/pulse/history.md`
- archived history architecture and research

The donor worktree was not modified. Its Effigy graph reported
`refresh-recommended`; the audit used exact source reads rather than mutating
the donor's graph cache.

## Corrected Donor Characterization

| Area | Live behavior | Boundary consequence |
| --- | --- | --- |
| payload | `PulseHistoryMutation` has 83 DAW-specific variants with inverse, coalesce, and no-op logic | payload and semantic policy stay in Loophole |
| linear stack | separate undo/redo vectors, stable entry labels and ids, revisions, default limit 100 | generic sequence mechanics are extractable |
| recording | successful product mutations call `record_history`; a new record clears redo; adjacent compatible payloads coalesce | record-after-apply and linear divergence need explicit shared semantics |
| grouping | a 750 ms keyed grouping API exists and is unit-tested, but no live Pulse call site invokes it | preserve the capability, not a false live-parity claim |
| jump | the pure stack can move to an index; live Aura instead loops undo/redo by visible entry id with a 512-step guard | shared navigation must use stable ids and one atomic plan |
| projection | Pulse exposes depths, next labels, and eight recent applied entries; Aura remembers disappeared redo entries client-side | shared projections must expose authoritative past and future data |
| persistence | full typed undo/redo stacks sit inside the canonical project snapshot; incompatible history payloads silently drop the whole history | shared envelopes need explicit compatibility and recovery receipts |
| recovery | a disposable one-app-version JSONL journal records mutation/undo/redo transitions, skips checkpointed revisions, tolerates a torn tail, and replays a valid prefix | committed transition records are reusable; checkpoint and file policy are consumer concerns |
| apply | undo/redo moves the stack before applying the payload; compounds apply sequentially | donor does not prove failure invariance or atomic compounds |
| branching | the live stack clears redo after divergent edits | Loophole does not currently ship forkable undo |
| versions | project versions are designed as immutable snapshot lineage above working-tree undo | project versions and undo branches must remain separate systems |

The earlier memo overstated two points. Gesture grouping is not wired into live
Pulse call sites. Forkable history exists in archived research and competitor
analysis, not in the live donor.

## Recommendation

### Pure linear kernel

Create optional `longhorn-history` over a generic typed payload. Longhorn owns:

- bounded history, entry, group, and plan identities
- monotonic history-state revision
- past/current/future topology for the linear mode
- record, replace, remove, retention, and projection mechanics
- deterministic navigation planning and checked commit
- versioned structural persistence envelope
- committed transition receipts

The consumer supplies typed payloads plus pure inverse, coalesce, and no-op
policy. The consumer also supplies the atomic apply transaction. Longhorn
never interprets DAW mutations or mutates the product model.

### Plan, apply, commit

Undo, redo, and checkout follow one protocol:

1. plan against the current history revision
2. return ordered inverse and forward payloads plus the exact target
3. let the consumer validate and apply the complete batch atomically
4. commit the plan only after product success
5. emit one transition receipt from the committed state

A failure before commit leaves history unchanged. Stale plans reject. A
consumer without an atomic batch or rollback contract cannot expose
multi-entry checkout or compound navigation.

This improves on the donor's current move-before-apply behavior without losing
successful behavior.

### Coalescing and grouping

Coalescing is consumer policy over two typed payloads. Longhorn owns when that
policy may run and how a merge, no-op removal, or separate entry changes the
sequence.

Gesture grouping uses explicit group tokens or an injected monotonic clock and
consumer duration. No ambient `Instant` or hard-coded 750 ms enters the public
contract. Loophole can select 750 ms when it wires the capability.

### Persistence and recovery

The history envelope stamps:

- format family and structural version
- payload codec family and version
- history revision and mode
- limits
- entries and current position

Future or incompatible payloads reject with an explicit recovery outcome.
Consumer migration hooks may deliberately discard history, but never silently.
The canonical product snapshot remains separate.

Every committed record, navigation, coalesce, and prune produces a structural
transition receipt. A consumer journal may persist those receipts plus its
typed payload and model revision. Longhorn does not choose paths, fsync,
checkpoint cadence, autosave, replay acceptance, or canonical project format.
This seam can preserve Loophole's journal and cross-session undo.

### Client and presentation

Generate metadata-only snapshots, paged entry projections, commands, receipts,
and errors. Product payloads do not cross into a generic renderer protocol.
The client knows authoritative position, past/future membership, truncation,
and current history revision.

`@longhorn/history` remains framework-neutral. Optional `/svelte` state is
per-instance. Optional `/poodle` composition uses public controlled primitives
for a linear history panel. Poodle remains visual authority.

### Fork prototype

Forkable history stays experimental. The prototype must prove:

- immutable single-parent entry nodes
- a stable current node and branch references
- divergent record after undo without deleting the former future
- checkout through the lowest common ancestor
- one atomic plan/apply/commit protocol
- deterministic preferred-redo behavior
- named or pinned branch retention
- bounded pruning by entry count and encoded weight
- opaque consumer checkpoints and replay-cost accounting
- versioned graph persistence and payload migration
- a linear-default projection with optional alternate-path metadata

The prototype compares derived branches with first-class branch references.
No public branch package or Loophole adoption follows until the promotion
decision.

## Loophole Lossless Migration Map

| Loophole behavior | Shared owner | Loophole owner after cutover |
| --- | --- | --- |
| entry sequence, cursor, limit | `longhorn-history` | configured limit |
| inverse/coalesce/no-op decisions | invocation seam | all 83 mutation cases |
| 750 ms gesture grouping capability | explicit shared group policy | chosen keys, duration, live call sites |
| apply and rollback | checked shared plan | Pulse transaction over runtime state |
| labels and mutation kinds | bounded metadata carrier | DAW wording and icons |
| persisted sequence envelope | structural shared format | payload codec and project migration |
| mutation/undo/redo journal records | committed transition seam | project revision, file, checkpoint, fsync, replay policy |
| history panel state | checked TS/Svelte/Poodle projection | placement, product copy, command composition |
| project versions and variants | none | Pulse snapshot lineage |
| branch mode | prototype only | disabled unless separately promoted |

The later g01.015 migration must import existing Pulse history explicitly,
retain successful undo/redo/coalesce/jump behavior, keep journal recovery and
cross-session undo, and remove duplicated generic stack mechanics. It may
leave the mutation enum and apply adapter in `pulse-history`.

## Second Shape

A small non-editor state document supplies the second fixture:

- typed set, insert, remove, and compound payloads
- explicit transaction rollback
- different labels and coalescing policy
- no Tauri, Svelte, Poodle, project file, or journal requirement

This proves the abstraction without pretending another audited app already has
Loophole-class history.

## Rejected

- arbitrary JSON as the core payload
- product apply logic in Longhorn
- stack mutation before product success
- partial compound success
- silent history loss on compatibility failure
- renderer-owned durable position or remembered redo authority
- event sourcing as the canonical product model
- branch mode by default
- unifying undo branches with project versions or collaboration
- copying a Poodle component

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../architecture/system-inventory.md`
- `../../contracts/008-history-kernel-boundary.md`
- `../../specs/001-shared-desktop-system-suite.md`
- `../../roadmaps/g01/011-history-kernel-and-branching-prototype.md`

## Remaining Gate

Linear implementation is ready. Forking remains a prototype and decision
gate. It cannot become a public package or a Loophole migration dependency
until Cards 068-069 close the graph, persistence, pruning, checkpoint, and
performance questions.

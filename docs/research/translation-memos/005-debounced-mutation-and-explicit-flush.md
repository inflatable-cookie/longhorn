# 005 Debounced Mutation And Explicit Flush

Status: promoted  
Owner: Tom  
Updated: 2026-07-28  
Promotes to: contract 004; g01.002 batch 2

## Question

How should Longhorn coalesce high-frequency configuration changes while
preserving fresh-value mutation, bounded memory, truthful failure reporting,
and explicit close/shutdown behavior?

## Donor Evidence

No audited donor supplies the complete generic contract.

| Donor | Proven behavior | Generic gap |
| --- | --- | --- |
| Loophole | `echo-configuration/src/workspace_manager.rs` uses a 200 ms trailing debounce, caller-driven elapsed-time checks, effective staged reads, semantic dedupe, and forced flush | stages a whole layout snapshot; removes it before I/O, so failure loses intent; an intervening process update can be overwritten |
| Nucleus | `apps/desktop/src-tauri/src/window_geometry.rs` uses a 300 ms worker-thread debounce, latest-value coalescing, close flush, and a one-second acknowledgement wait | acknowledgement does not carry the write result; failed writes are logged and lost; thread and window-close policy are app-owned |
| Soundcheck | `src-tauri/src/app_settings.rs` uses a 300 ms worker-thread debounce and rereads the settings document before changing only `main_window` | worker errors are discarded; flush acknowledgement still reports completion; the two-second fallback covers channel failure, not an ordinary write failure |
| Bovine | `src/App.svelte` uses a 200 ms renderer timer plus gesture-end, blur, and teardown flushes; `src-tauri/src/workspace.rs` rereads preferences and changes the presentation projection | rejected saves lose the scheduled attempt; teardown cannot await durability; overlapping promises have no sequencing receipt; backend writes are not coordinated or atomic |
| Jetstream | no reusable configuration debounce mechanism found | negative evidence only |

The useful intersection is trailing-edge coalescing, explicit forced flush,
and projection-sized mutation. Worker ownership, whole-snapshot staging,
failure handling, and close policy conflict across donors.

## Promoted Decision

### Opt-in, process-local lanes

Debounce is an opt-in latency and crash-loss trade. It is suitable for
high-frequency reconstructable state such as window geometry, presentation
layout, or recent UI preferences. A stage receipt does not claim persistence.
Critical settings use immediate mutation unless the consumer accepts loss of
the unflushed interval after process failure.

One scheduler lane is keyed by store and domain. The lane contains at most one
coalesced typed intent. It does not contain a desired whole-domain snapshot,
unchecked bytes, or an unbounded closure queue.

Store reads remain authoritative disk reads. Pending UI state stays with the
consumer. Longhorn exposes pending metadata, not a hidden staged override from
`ConfigStore::load`.

### Typed intent and coalescing law

Each debounced domain use supplies:

- an owned typed intent
- a deterministic coalescer
- an applicator from intent to the consumer-owned domain value
- a deterministic pending-weight function and configured maximum
- debounce delay and card-002 mutation options

The coalescer preserves ordered application:

```text
apply(coalesce(a, b), fresh) == apply(b, apply(a, fresh))
```

for every valid fresh domain value. Last-value replacement is valid only when
the applicator replaces the same bounded projection, such as window geometry.
An ordered command batch is valid only while it remains within the configured
weight limit.

Coalescing is transactional. Longhorn checks the new intent and candidate
coalesced intent before replacing pending state. A coalescer error or weight
overflow rejects the new stage, retains the previous intent, and does not move
its deadline.

Accepted staging uses trailing-edge timing: each accepted intent receives a
monotonic generation and resets the due time. One immediate stage receipt
states whether it opened a lane or coalesced into an existing generation.
Longhorn does not retain one future, waiter, or completion object per stage.

### Runtime ownership

Longhorn owns the deterministic scheduling state machine. It does not create a
thread, async runtime, timer, or Tauri task.

The state machine uses an injected monotonic clock, exposes the next deadline,
and supports due and forced flush calls. A host adapter chooses how to wake at
that deadline and runs blocking filesystem work away from the UI thread.
Tests use a fake clock; they do not sleep.

This shape supports Loophole-style event-loop polling, native worker hosts, and
later Tauri adapters without putting one runtime into `longhorn-config`.

### Fresh-value flush

A flush never publishes the staged representation directly. It enters the
card-002 coordinated mutation path, rereads the authoritative value under the
store-wide lock, applies the coalesced intent, validates, encodes, and
publishes while retaining that guard.

Two processes may stage independently. Their flushes serialize in lock
acquisition order, and each intent sees the last published value. Advisory
lock and remote-authority limits from contract 004 still apply.

After applying an intent, Longhorn compares the current and candidate encoded
domain values while still holding the guard. Equal values clear pending state
and return an unchanged receipt without replacing the file.

### Failure and retry

Longhorn performs no hidden retry.

An uncommitted failure retains the exact coalesced intent and enters
`retry-required`. Due polling does not attempt it again or busy-loop. The
consumer may:

- call forced flush to retry
- stage another intent, which coalesces and starts a new debounce interval
- explicitly discard pending state and receive a discard receipt

Failures state whether retry is likely useful, but even a non-retryable
validation or authority failure retains intent until explicit replacement or
discard.

Atomic replacement is the commit boundary. A failure before replacement
retains pending intent. A failure after replacement, such as required
directory durability not being established, clears the intent and returns a
`published-with-durability-failure` result. Retrying that intent could apply a
non-idempotent command twice.

### Receipts and observability

Stage and flush results carry the domain id and monotonic generation.

Required flush outcomes are:

- no pending work
- not yet due
- unchanged and cleared
- published, including the card-002 mutation receipt
- uncommitted failure with pending retained and retry classification
- published with durability failure and pending cleared

A published or unchanged receipt covers every stage through its generation.
Superseded or merged stages do not receive later individual callbacks. A lane
snapshot exposes pending generation, due time, retry-required state, and last
terminal result with bounded bookkeeping.

### Explicit close and shutdown

Longhorn exposes forced flush for one lane and an aggregate flush for all
lanes. Aggregate flush:

- visits domains in stable domain-id order
- attempts every lane even after one fails
- is not a cross-domain transaction
- returns one terminal result per attempted lane
- retains only work that failed before publication

The host must invoke and await it before tearing down the runtime or storage
authority. Drop and destructors perform no I/O. A Tauri adapter may delay,
cancel, or continue close after failure, but that product policy is not part
of `longhorn-config`.

An unawaited renderer teardown call cannot claim a successful flush. A finite
host shutdown deadline may stop waiting, but timeout is a visible failed close
result, not an acknowledgement.

## Rejected Options

| Option | Reason |
| --- | --- |
| stage a whole domain snapshot | overwrites an intervening fresh mutation |
| queue patch closures | hard to inspect, bound, coalesce, and retry safely |
| one timer/thread inside the config crate | couples pure storage to one host runtime |
| discard pending before I/O | loses intent on pre-publication failure |
| retry every due poll | can hot-loop and hides product retry policy |
| retain after known replacement | can apply non-idempotent intent twice |
| acknowledge close without the write result | overstates persistence |
| perform I/O from `Drop` | cannot await, report, or reliably order shutdown |
| staged values shadow store reads | hides the authoritative persistence state |

## Required Proof

- fake-clock trailing-edge timing has no sleeping tests
- sequential patches coalesce with the declared ordered-application law
- last-value projection replacement stays bounded
- overflow rejects only the new intent and preserves the old deadline
- another store mutation between stage and flush remains intact
- equal fresh and candidate encodings skip publication
- every pre-publication failure retains the exact pending generation
- due polling after failure does not retry automatically
- forced retry publishes retained intent once
- a known post-replacement durability failure clears pending intent
- stage receipts and terminal receipts reconcile through generation
- aggregate close flush attempts all lanes and reports partial failure
- dropping a scheduler performs no write
- package dependencies remain free of Tauri and async-runtime requirements

## Promotion Result

Contract 004 now owns typed staged intent, ordered coalescing, bounded pending
state, host-driven scheduling, fresh-value flush, commit-aware retry,
generation receipts, and explicit aggregate shutdown flush.

Card 003 is ready as one bounded implementation card. Tauri lifecycle wiring,
TypeScript/Svelte bindings, backup, restore, and consumer migration remain
later work.

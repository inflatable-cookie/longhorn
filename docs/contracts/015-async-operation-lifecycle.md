# 015 Async Operation Lifecycle

Status: active; authority and presentation implemented
Owner: Tom
Updated: 2026-07-31
Evidence: `../research/translation-memos/016-async-operation-and-notification-boundary.md`

## Boundary

Longhorn may provide an optional product-neutral authority for long-running
desktop work. It owns operation identity, legal lifecycle transitions,
bounded progress, cancellation request receipts, terminal outcomes, finite
catalogue projections, and client reconciliation.

The authority does not execute work, schedule a queue, interpret product
progress, or decide which operation may start.

## Package Shape

- `longhorn-operation`: pure operation authority and projections
- `longhorn-tauri-operation`: narrow handler/event assembly over an injected
  authority
- `@inflatable-cookie/longhorn-operation`: generated framework-neutral protocol and client
- `@inflatable-cookie/longhorn-operation/tauri`: optional Tauri transport composition
- `@inflatable-cookie/longhorn-operation/svelte`: optional per-instance reactive session
- `@inflatable-cookie/longhorn-operation/poodle`: optional controlled public-Poodle projection

The pure crate imports no async runtime, bridge, config, Tauri, Svelte,
Poodle, executor, or consumer package.

Cards 075-076 implement the pure authority: bounded identities and progress,
distinct revisions, queued or direct-running registration, the exact sticky
lifecycle, cancellation receipts, count/weight retention, retry lineage,
explicit dismissal, and controlled teardown. Serialization, transports, and
clients remain later card work.

## Authority

Longhorn owns:

- bounded operation, kind, scope, phase, and authority identity
- per-operation and catalogue revision
- transition validation and terminal stickiness
- cancellation admission and receipts
- bounded current/recent projections
- explicit retention and teardown outcomes

Consumers own:

- admission, queue order, concurrency, deduplication, and pause policy
- executor, resource locking, cleanup, and authorization
- product payloads, warnings, logs, reports, and artifacts
- outcome evidence and failure wording
- retry policy and command mapping
- persistence, restart resumption, and recovery

## Identity And Registration

Every operation carries:

- stable opaque operation id from an injected source
- bounded operation-kind id
- optional bounded consumer scope id
- authority epoch
- monotonic operation revision
- monotonic catalogue sequence
- bounded presentation label
- optional opaque `retry_of` operation id

Identity is not a wall-clock timestamp, array index, window id, bridge request
id, or renderer-generated optimistic key. A bridge correlation id may map to
an operation id but does not replace it.

Registration admits a consumer-provided initial state of `queued` or
`running`. Registration policy decides whether a same-kind request reuses an
active operation, creates another, or rejects. The core validates the result;
it does not choose it.

## Lifecycle

The closed state set is:

- `queued`
- `running`
- `cancelling`
- `succeeded`
- `failed`
- `cancelled`
- `interrupted`

Allowed forward transitions are explicit. `queued` may start, cancel before
start, fail admission after registration, or become interrupted. `running`
may request cancellation or reach any executor-proven terminal outcome.
`cancelling` may become `cancelled`, `succeeded`, `failed`, or `interrupted`.

Every terminal state is sticky. A second terminal claim, later progress, or
attempt to reopen the operation rejects without changing state.

`interrupted` records consumer-proven loss of the executor or host. It does
not claim automatic persistence, restart recovery, or resumability.

## Cancellation

Cancellation is a request protocol:

1. caller submits operation id, authority epoch, and expected revision
2. authority checks cancellation support and current state
3. authority returns `accepted`, `already_requested`, `unsupported`,
   `terminal`, or a checked rejection
4. accepted running work enters `cancelling`
5. executor later reports the actual terminal outcome

Acceptance never claims that work stopped. A success or failure that wins the
race after cancellation acceptance remains truthful. Only executor-confirmed
stop becomes `cancelled`.

Repeated requests are idempotent. Stale epoch or revision rejects. Cancelling
queued work may become terminal immediately only when consumer admission
confirms execution never started.

## Progress

The common projection supports:

- indeterminate progress
- optional overall completed/total units
- optional normalized overall fraction
- optional bounded phase id and presentation label
- monotonic progress sequence

Overall progress cannot regress. Units must be finite, non-negative, and not
exceed a declared total. Phase-local units may restart only under a new phase
id. A changed total must preserve the already-reported overall fraction.

Product step payloads, warnings, logs, artifact paths, reports, approval
requests, and recovery instructions stay in typed consumer contracts. The
generic TypeScript protocol carries no arbitrary JSON product detail.

## Catalogue And Retention

The authority exposes bounded active and recent projections. Active operations
cannot be evicted. Terminal retention uses explicit finite count and encoded
metadata-weight limits. Optional age policy uses injected monotonic time; wall
clock is presentation evidence only.

Eviction returns exact ids and advances catalogue revision. A caller may
explicitly dismiss a terminal projection without deleting consumer artifacts
or logs. Unknown eviction and silent unbounded growth are forbidden.

## Retry

Retry creates a new operation. It may carry `retry_of` and copy bounded
presentation metadata. It never reopens or mutates the terminal operation.

Eligibility, arguments, authorization, backoff, and whether retry is shown
remain consumer policy. The operation package does not execute a command.

## Teardown And Recovery

Renderer unmount only disposes listeners. It does not cancel host work.

Authority teardown produces an explicit outcome for every non-terminal
operation. The consumer may complete cancellation, mark interruption, or
transfer to another live authority. Silent disappearance is forbidden in a
controlled shutdown.

The v1 core is not a durable scheduler. A consumer that persists operations
must reconcile each recovered non-terminal record before publication. It may
resume under a new authority epoch or publish `interrupted`; it may not replay
stale `running` state as live fact.

## Projection And Events

Snapshots contain authority epoch, catalogue revision, bounded active and
recent entries, and truncation evidence. Events contain request correlation,
authority cursor, optional exact operation id, previous and committed
catalogue revisions, and change kind.

Clients listen before taking a current snapshot. Duplicate and stale events
are ignored. A gap, epoch change, newer snapshot, or unknown operation forces
refresh. Events are non-durable refresh evidence, not catalogue authority.

Card 077 implements this seam. Public events carry request correlation,
authority cursor, previous and committed catalogue revisions, optional
operation id, and change kind. They carry no product result, artifact, report,
or log payload.

The same semantics must work through direct, Tauri, and bridge-domain
transports. `longhorn-bridge` may carry correlation, progress, cancellation,
and terminal metadata for one request. It does not own the operation catalogue
or become a pure-core dependency.

## Svelte And Poodle

The Svelte session is per-instance. It owns subscription lifetime, current
snapshot reconciliation, request-keyed pending state, and teardown. Multiple
windows may observe the same host authority without sharing renderer globals.

The Poodle adapter maps authoritative state to public `Progress`,
`StatusIndicator`, buttons, lists, and dialogs. Poodle owns visuals. Longhorn
does not fork feedback components or place product copy in the generic layer.

Card 078 implements this boundary. A framework-neutral controller owns one
listener-first subscription, monotonic snapshot installation, selection, and
request-keyed cancellation or dismissal per renderer instance. Svelte mirrors
that controller. Unmount clears renderer state and listeners only. Delayed
results cannot replace a newer revision or authority epoch. The public-Poodle
panel accepts consumer detail as a snippet and carries no product payload.

## Validation

- Soundcheck scan and Loophole render fixtures use one public transition API.
- Queued and direct-running registration both pass.
- Cancellation acceptance followed by success, failure, and cancellation is
  deterministic.
- Late progress, duplicate terminal, stale revision, and stale epoch leave
  exact state unchanged.
- Retention never evicts active work and reports every terminal eviction.
- Listener-first reconnect and renderer remount preserve current truth.
- Minimal Rust and TypeScript graphs omit bridge, Tauri, Svelte, and Poodle.

## Rejected

- generic queue scheduler
- cancellation receipt as terminal cancellation
- ambient wall-clock ordering or randomness
- arbitrary product progress JSON
- renderer-authoritative operation truth
- retry by reopening terminal state
- silent loss of live work on controlled teardown
- bridge job tracking as catalogue authority

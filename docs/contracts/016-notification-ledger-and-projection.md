# 016 Notification Ledger And Projection

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-31
Evidence: `../research/translation-memos/016-async-operation-and-notification-boundary.md`

## Boundary

Longhorn may provide an optional retained notification ledger plus checked
renderer projections. Notifications are user-facing records of domain facts.
They are not operation states, logs, toasts, or native OS delivery requests.

An optional consumer projector may observe operation terminal transitions and
produce notification drafts. The notification ledger remains independently
usable for non-operation events.

## Package Shape

- `longhorn-notifications`: pure ledger, projections, receipts, and optional
  operation-observer seam
- `longhorn-tauri-notifications`: narrow handler/event assembly over an
  injected ledger
- `@inflatable-cookie/longhorn/notifications`: generated framework-neutral protocol and client
- `@inflatable-cookie/longhorn-tauri/notifications`: optional Tauri transport composition
- `@inflatable-cookie/longhorn-poodle-svelte/notifications/svelte`: optional per-instance reactive session
- `@inflatable-cookie/longhorn-poodle-svelte/notifications/poodle`: optional panel and toast projection over
  public Poodle primitives

The pure notification root does not require the operation package. Optional
integration composes at an adapter edge. Operations never depend on
notifications.

## Authority

Longhorn owns:

- bounded notification, source, replacement-key, and action-reference ids
- ledger revision and deterministic newest-first order
- severity, bounded title and summary, and optional causal reference
- unseen/seen state, dismissal, and finite retention
- exact add, replace, mark-seen, dismiss, clear, and prune receipts
- checked snapshot/event reconciliation

Consumers own:

- which facts create records
- wording, redaction, localization, and sensitive detail
- action authorization and execution
- operation-to-notification policy
- native OS delivery, permission, sound, badge, focus, and privacy policy

## Record

Each active record carries:

- stable opaque notification id from an injected source
- bounded source-kind id
- severity: `info`, `success`, `warning`, `error`, or `critical`
- bounded title and summary
- monotonic ledger sequence
- injected optional wall-clock presentation time
- `unseen` or `seen`
- optional opaque causal reference
- zero or more bounded semantic action references and labels
- optional replacement key

The causal reference may identify an operation or another consumer domain
fact. Longhorn does not dereference or authorize it.

Actions are data, not closures or unchecked command strings. The consumer maps
an action reference into its command registry or another typed executor and
performs fresh authorization at invocation.

## Add And Replace

Add always creates a new record. Optional replace-by-key is an explicit
command. It updates one active matching source/key record, assigns a new ledger
revision, and returns the retained notification id. The ledger does not infer
duplicates from title text.

Replacement may mark the record unseen only when the command requests that
behavior. Severity escalation and de-escalation are consumer decisions.

## Seen And Dismissed

Seen and dismissed are distinct:

- marking seen retains the record and reduces unseen count
- dismiss removes the record from the active ledger and returns an exact
  receipt
- clearing may target an explicit bounded set or the complete current
  projection under one expected revision

A toast becoming visible may mark a record seen through an explicit command.
Toast expiry never dismisses the retained record. Notification-panel read
state cannot be derived from retained count.

## Retention

The ledger has finite record-count and encoded metadata-weight limits. An
optional age limit uses injected monotonic time. Admission or replacement
prunes according to explicit policy and returns every pruned id.

Critical records may be protected only by an explicit consumer-selected
retention class. The ledger never silently grows without bound and never
silently drops a newly admitted record.

Dismissal and pruning remove notification metadata only. Product logs,
artifacts, operation results, and recovery evidence remain consumer-owned.

## Operation Observation

An optional projector receives one committed operation transition and returns
zero or one notification draft. It may inspect bounded operation metadata and
consumer-supplied typed policy input. It does not mutate the operation or
assume every success, failure, cancellation, or interruption should notify.

Projection is idempotent under an explicit source/replacement key or producer
deduplication token. Notification publication failure cannot change an
already-committed operation terminal outcome.

## Snapshot And Events

Snapshots expose authority epoch, ledger revision, unseen count, bounded
newest-first pages, and truncation evidence. Events carry previous and next
ledger revision plus exact affected ids.

Clients listen before taking a current snapshot. Duplicate and stale events
are ignored. Gaps and authority-epoch changes force refresh. Renderer memory
does not reconstruct dismissed or unseen authority.

## Svelte, Poodle, And Presentation

The Svelte session is per-instance and owns subscription lifetime, paging,
mark-seen requests, dismissal requests, and stale-result rejection.

The Poodle adapter uses public `ToastHost`, `ToastStack`, status, list, button,
and dialog primitives. It may project selected new records into transient
toasts and expose retained records in a controlled panel.

Toast duration, sticky tones, animation, and visible-stack deduplication are
renderer presentation policy. Poodle owns those mechanics. Longhorn maps
notification ids, tone, title, summary, and semantic action callbacks without
copying the component.

Native OS notifications are out of scope for v1. A later adapter requires
permission, foreground/background, redaction, focus, and platform-delivery
contracts.

## Validation

- Loophole-shaped render and reliability records share one ledger.
- A non-operation notification requires no operation dependency.
- Seen, dismiss, clear, replace, prune, and toast expiry remain distinct.
- Duplicate and stale commands leave exact state unchanged.
- Optional terminal-operation observation is idempotent and failure-isolated.
- Svelte remount and multi-window observation preserve host truth.
- Poodle integration uses the public feedback primitives.
- Minimal package graphs omit operation, bridge, Tauri, Svelte, and Poodle.

## Rejected

- one shared state machine for operations and notifications
- unread count derived from retained record count
- toast expiry as dismissal
- title-text deduplication
- closures or untyped commands as durable actions
- renderer-owned retained ledger
- native OS delivery without privacy and permission policy
- notification publication changing operation success or failure


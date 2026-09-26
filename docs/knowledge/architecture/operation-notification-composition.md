# Operation And Notification Composition

Status: promoted  
Owner: Tom  
Updated: 2026-07-31  
Contracts: `../contracts/003-extraction-and-consumer-migration.md`,
`../contracts/007-optional-backend-topology.md`,
`../contracts/010-rust-typescript-ipc-and-events.md`,
`../contracts/012-distribution-and-compatibility.md`,
`../contracts/013-svelte-and-poodle-adapter-lifecycle.md`,
`../contracts/015-async-operation-lifecycle.md`, and
`../contracts/016-notification-ledger-and-projection.md`

## Selection Rule

Choose operations and notifications independently. An app may use either,
both, or neither.

| Need | Rust | TypeScript | Optional edge |
| --- | --- | --- | --- |
| Finite long-work truth | `longhorn-operation` | `@inflatable-cookie/longhorn/operation` | direct or serialized root |
| Tauri-local operations | add `longhorn-tauri-operation` | add `/tauri` | injected authority and executor |
| Service-backed operations | add `longhorn-bridge` | add `/bridge` | one typed bridge domain |
| Per-window operation state | unchanged | add `/svelte` | one session per mount |
| Operation panel/progress | unchanged | add `/poodle` | public Poodle primitives |
| Retained user outcomes | `longhorn-notifications` | `@inflatable-cookie/longhorn/notifications` | independent ledger |
| Tauri-local notifications | add `longhorn-tauri-notifications` | add `/tauri` | injected ledger |
| Per-window notification state | unchanged | add `/svelte` | local selection and toasts |
| Notification panel/toasts | unchanged | add `/poodle` | public Poodle primitives |
| Terminal-operation projection | notifications with `operation` feature | unchanged | consumer observation policy |

The minimal operation graph contains core and operation only. The
notification-only graph contains core and notifications only. Svelte, Poodle,
Tauri, bridge, config, settings, commands, and history enter only when chosen.

## Authority Split

Longhorn owns:

- operation identity, revisions, finite lifecycle, progress, cancellation
  request receipts, retry lineage, retention, and teardown accounting
- notification identity, ledger revisions, seen state, replacement,
  dismissal, pruning, finite retention, and bounded pages
- checked protocol compatibility, correlation, listener-first refresh, and
  stale-event rejection
- per-instance renderer projection and transient toast lifetime

The consumer owns:

- executor implementation and work lifetime
- queue order, scheduling, concurrency, pause, resume, and durable restart
- product phases, reports, warnings, artifacts, logs, and result payloads
- retry admission and the act of creating a retry operation
- retention limits and notification publication policy
- all user-facing wording, icons, detail content, and panel placement
- semantic-action meaning and fresh authorization at invocation
- caller capabilities, authority construction, storage, migration, and
  product telemetry

Cancellation acceptance means the executor was asked to stop. Only an
executor-supplied terminal transition means work ended. Renderer teardown
releases observation; it never cancels host work.

## Composition Paths

Direct and Tauri-local operation paths share one authority:

```text
consumer executor → operation authority → snapshot/receipt/event
                         ↑                     ↓
                 Tauri handler          checked TS client
```

A service-backed app substitutes one typed bridge-domain port. Bridge owns
transport session and domain authority evidence, not operation catalogue
truth. Do not mirror one operation across local and service authorities
without a consumer reconciliation contract.

Notifications remain a second authority:

```text
domain fact ───────────────┐
operation terminal receipt ├→ consumer publication policy → notification ledger
service reliability fact ──┘
```

Notification publication failure cannot alter the operation result. The
operation observer is optional and failure-isolated. Use stable producer
tokens for idempotent projection and replacement keys for one live status per
source. Do not use notification IDs as operation state.

## Renderer And Actions

Create one session per mounted window or panel. Each session installs the
listener before its first snapshot, refreshes from authoritative truth, and
releases exactly its own listener on teardown. Selection, pending UI, and
toasts are renderer-local. Retained operation and notification records reload
on remount.

Toast expiry removes only the transient projection. It does not mark seen or
dismiss the ledger record. Seen, dismiss, clear, replace, prune, and toast
expiry are distinct outcomes.

Notification action references are data, not executable authority. Resolve
the reference through a consumer executor and repeat context, capability, and
product-availability admission when invoked. A Tauri permission or an action
rendered earlier is not sufficient authorization.

`/poodle` uses public progress, status, controlled-list, and toast primitives.
Poodle owns visuals. Consumers own wording, icons, detail snippets, shell
layout, and action mapping.

## Proven Shapes

| Shape | Rust graph | TypeScript graph | Evidence |
| --- | --- | --- | --- |
| Minimal operation | core, operation | core, operation root | direct/serialized parity; no optional peers |
| Soundcheck-shaped | core, operation, Tauri operation | core, operation root plus `/tauri` | running, accepted cancellation, repeated cancellation, terminal cancellation, rejected late progress |
| Loophole-shaped | core, bridge, operation, notifications, both Tauri adapters | core, bridge, Tauri, operation and notification optional subpaths | queued render, progress, terminal result, direct/Tauri/bridge parity, two windows, remount, retained panel, toast expiry, fresh action rejection |
| Notification-only | core, notifications, Tauri notifications | core, notifications root plus `/tauri` | add, seen, dismiss; no operation or command package |

All TypeScript shapes install packed artifacts in clean temporary roots. Rust
fixtures run offline from private source inventories. Native and renderer
public traces match. This is conformance evidence, not donor migration.

## Migration Prerequisites

| Consumer | Before cutover | Consumer-owned decisions | Shared target |
| --- | --- | --- | --- |
| Soundcheck | inventory scan start/reconnect/cancel/terminal paths; capture current capabilities and legacy state | scanner executor, same-active policy, reports, warnings, restart recovery, wording | operation authority, Tauri port, optional Poodle status |
| Loophole | inventory render queue, cancellation races, progress phases, completion notices, command admission, window mounts | queue/scheduler, render payloads, artifacts, retry policy, notification wording/actions, durable project integration | full operation hierarchy plus independent notification ledger and optional bridge port |
| Nucleus | identify first genuine long task or retained outcome; do not add either system speculatively | executor, placement, wording, retention | select only the needed minimal package |
| Split-shell | separate service reliability facts from task state | backend lifecycle, retry, action mapping | notification-only or operation-only composition as evidence requires |
| Jetstream | identify batch/CLI work that benefits from desktop projection | process execution, logs, cancellation mechanics | framework-neutral operation root; UI edges optional |

For every migration:

1. Freeze native and renderer lifecycle traces before changing authority.
2. Map executor, queue, retry, retention, wording, and action ownership
   explicitly. Missing ownership blocks cutover.
3. Run Longhorn beside copied or disposable donor state. Compare accepted,
   repeated, failed, stale, late, teardown, reconnect, and remount paths.
4. Select one operation authority and one notification authority at bootstrap.
   Avoid dual writes without an explicit transaction and reconciliation plan.
5. Move protocols and capabilities before presentation. Mount Poodle only
   after semantic parity.
6. Keep legacy durable state until publication, restart, and rollback checks
   pass. Migration receipts and source cleanup belong to the consumer lane.
7. Narrow capabilities and repeat fresh action admission after cutover.

Public registry normalization, native OS delivery, durable schedulers, and
consumer-repository edits remain later release or migration work.

# 016 Async Operation And Notification Boundary

Status: complete and promoted
Owner: Tom
Updated: 2026-07-31
Promotes: `../../contracts/015-async-operation-lifecycle.md` and
`../../contracts/016-notification-ledger-and-projection.md`

## Prompt

Characterize long-running desktop work and user-visible outcomes across the
audited Tauri apps. Decide whether a shared operation lifecycle is justified,
whether notifications share that lifecycle, and which donor behavior must stay
product-owned.

## Sources

Read-only Soundcheck audit at
`aa749d1e577e7956b75a892cdf589304d23b71862`:

- `docs/contracts/015-plugin-scan-lifecycle-contract.md`
- `src/lib/api.ts`
- `src/lib/daw-state-store.ts`
- `src/lib/types.ts`
- `src-tauri/src/library_commands.rs`
- `src-tauri/src/sync_commands.rs`
- `src-tauri/src/assistant_tagging.rs`
- `src-tauri/src/plugin_inspection_process.rs`
- `crates/soundcheck-core/src/library/inventory.rs`

Read-only Loophole audit at
`da08b50e7cc69b7d13636c94fc571a64db4ae8ca`:

- `pulse/crates/pulse-render-queue/src/lib.rs`
- `pulse/src/dispatch/render.rs`
- `pulse/src/dispatch/notifications.rs`
- `pulse/crates/pulse-project-model/src/notification.rs`
- `aura/src-tauri/src/commands/export_jobs.rs`
- `aura/src-tauri/src/services/pulse_authority.rs`
- `aura/src/renderer/workspace/ExportDialog.svelte`
- `aura/src/renderer/workspace/NotificationsPanel.svelte`
- `aura/src/renderer/chrome/toast-store.ts`
- `aura/src/renderer/chrome/host-notification-toasts.ts`
- `aura/src/renderer/chrome/ToastHost.svelte`

Corroborating read-only audits:

- Nucleus at `8c95c9c9eae5d340cf2f5faf0a3c3d4743059d29`:
  `crates/nucleus-agent-protocol` and desktop agent command/state surfaces
- Jetstream at `95222397974caa276123fa52c284024d18eadb3c`:
  editor and runtime task/progress surfaces
- Bovine Accelerator Desktop at
  `b1958794da2d0080104c9f2146c7f3e3f122249b`:
  `src/App.svelte`, `src/components/GitWorkspaceBar.svelte`, and Git commands
- Poodle public feedback primitives: `Progress`, `StatusIndicator`,
  `ToastHost`, and `ToastStack`

No donor repository was modified.

## Donor Characterization

### Soundcheck

The plugin scan is a strong lifecycle donor:

- one active scan with a stable job id
- `idle`, `running`, `cancelling`, `cancelled`, `succeeded`, and `failed`
- reconnectable current-status query
- cooperative cancellation
- enumerating, introspecting, and reconciling phases
- optional totals, monotonic counts, active-unit labels, and bounded warnings
- remount does not cancel host work
- failed or cancelled scans do not reconcile product inventory

Other Soundcheck work uses different local shapes. Assistant taxonomy uses a
request id, listener-before-invoke progress, `AbortSignal`, and explicit
unlisten. DAW apply uses plan-correlated milestone events. Inspection startup
uses process-boundary progress files and an atomic cancellation registry.
Composer sync is await-only. These prove the need for one shared projection,
not one shared product payload.

Scan phases, plugin records, taxonomy units, DAW plans, rollback meaning,
inspection helpers, warnings, and reconciliation stay in Soundcheck.

### Loophole

The render queue is a materially different strong lifecycle donor:

- stable queued job identity
- `queued`, `rendering`, `cancelled`, `completed`, and `failed`
- separate queue running/paused state
- current, queued, and recent projections
- progress percentage and report/artifact metadata
- cooperative cancellation between execution phases
- terminal-state stickiness and monotonic progress reduction
- explicit separation between queue authority and Aura execution

Aura currently polls its export registry. Its cancellation flag, product
report, partial-file cleanup, polling interval, queue order, pause behavior,
and render executor remain Loophole policy. Unbounded host-registry retention
and process-local loss are donor gaps, not shared defaults.

Loophole notifications are not operation states. Pulse emits records for
render completion, missing media and plugins, audio faults, save/recovery,
and other domain events. Aura retains a bounded host ledger, projects selected
records into transient toasts, and also exposes a notification panel. Toast
expiry and visible-toast deduplication do not delete host records.

### Corroborating apps

Nucleus has stable session and turn states, atomic cancellation, streaming
events, terminal outcomes, and waiting-for-approval or input states. Approval,
input, provider, token, tool, and recovery semantics are product workflow. A
generic operation may carry an opaque phase or blocker reference; it must not
absorb the agent state machine.

Bovine has renderer-local `load`, `switch`, and `sync` busy state around
awaited Git commands. Jetstream has runtime tasks and feedback primitives but
no mature retained desktop operation catalogue. They can consume a small
projection later, but neither is the second strong contract donor.

## Decision

### Shared operation authority is justified

Soundcheck scan and Loophole render queue share identity, revisioned
transitions, progress, cancellation request, terminal outcome, current
snapshot, and finite retention. They differ in scheduling, execution, product
payloads, retry, and persistence. That is the correct generic boundary.

Create optional `longhorn-operation` as a pure catalogue and transition
authority. It does not execute work or choose queue policy. Consumers inject
identity, admission, executor integration, product progress, and outcome
policy.

Core states are `queued`, `running`, `cancelling`, `succeeded`, `failed`,
`cancelled`, and `interrupted`. `interrupted` is a truthful terminal
classification for a consumer-reconciled host loss; it is not automatic
durable recovery.

Cancellation acceptance means only that a request reached the authority. The
executor still owns the terminal fact. Success or failure may win a race after
the state entered `cancelling`; only an executor-confirmed cancellation becomes
`cancelled`.

### Progress is bounded projection, not product workflow

The shared projection supports indeterminate work and optional overall units
or fraction. An opaque bounded phase id and presentation label may accompany
it. Phase ordering, step payloads, warnings, reports, artifacts, approval
requests, and product details remain typed consumer data.

Operation revision and event sequence protect against late progress. Overall
progress cannot regress. Phase-local units may restart only when the phase id
changes. Terminal operations reject later progress and terminal changes.

### Scheduling and retry stay outside

Loophole needs a queue. Soundcheck scan needs same-active-job reuse. Bovine
needs neither. The pure authority records `queued` but does not choose
concurrency, ordering, deduplication, pause, or start policy.

Retry creates a new operation with an optional `retry_of` reference. It never
reopens a terminal operation. Whether retry is offered is consumer policy.

### Notifications are a separate authority

Create optional `longhorn-notifications`. A notification is a retained user
record, not a job state. It may reference an operation or any opaque domain
cause, but it can also exist without an operation. A consumer projector may
turn selected terminal operation transitions into notification drafts.

The ledger owns bounded identity, revision, order, severity, safe summary,
semantic action references, seen state, dismissal, and retention. Seen and
dismissed are distinct. Dismissal cannot be inferred from toast expiry.

Poodle owns `Progress`, `StatusIndicator`, `ToastHost`, and `ToastStack`.
Longhorn supplies checked state and adapters over those public primitives.
Transient toast timers remain renderer-local. A notification panel reads the
host ledger. Native OS notifications, permission policy, and privacy redaction
remain later host adapters.

## Operation Boundary

Longhorn owns:

- bounded operation, kind, scope, and authority identity
- operation revision and catalogue revision
- legal state transitions and terminal stickiness
- cancellation request receipts and race semantics
- bounded progress projection
- current/recent projections and explicit retention
- listener-first snapshot/event reconciliation
- teardown and interruption receipts

Consumers own:

- work admission, queue ordering, concurrency, and executor
- product payloads, phases, warnings, logs, reports, and artifacts
- authorization and resource locking
- success, failure, cancellation, and interruption evidence
- retry eligibility and command mapping
- persistence, restart resumption, cleanup, and recovery policy

`longhorn-bridge` may carry correlated events for one operation. It does not
own the catalogue. Direct, Tauri, and bridge transports must expose the same
operation semantics without making bridge mandatory.

## Notification Boundary

Longhorn owns:

- bounded notification, source, deduplication, and action-reference identity
- severity, title, summary, ordering, seen state, and dismissal
- finite retained ledger and authoritative projections
- explicit add, replace-by-key, mark-seen, dismiss, and clear receipts
- optional operation-outcome observation seam
- checked client and transient-presentation mapping

Consumers own:

- which domain facts produce records
- safe wording and redaction
- semantic action execution and authorization
- whether an operation outcome notifies
- native OS delivery, sound, badges, focus, and privacy policy

## Package Shape

- `longhorn-operation`: pure lifecycle authority
- `longhorn-tauri-operation`: injected handler and event assembly
- `@longhorn/operation`: generated checked client with optional `/tauri`,
  `/svelte`, and `/poodle` edges
- `longhorn-notifications`: pure retained ledger and optional operation
  projector seam
- `longhorn-tauri-notifications`: injected handler and event assembly
- `@longhorn/notifications`: generated checked client with optional `/tauri`,
  `/svelte`, and `/poodle` edges

The two pure crates remain independent. The notification crate may expose an
optional operation-integration module without making operations a dependency
of its root or making notifications a dependency of operation authority.

## Lossless Donor Map

| Behavior | Shared owner | Consumer owner after adoption |
| --- | --- | --- |
| stable scan/render id | operation identity | id source and domain correlation |
| lifecycle transition checks | operation authority | executor fact and admission |
| progress snapshot | bounded projection | phase and product details |
| cancel request | checked receipt | cooperative stop and cleanup |
| current/recent list | finite catalogue | retention settings and product filters |
| queue order/pause | none | Loophole render queue |
| same-active scan reuse | none | Soundcheck admission policy |
| result/report/artifact | opaque reference only | donor domain |
| retained notification record | notification ledger | production and wording policy |
| terminal-to-notification mapping | optional projector | per-operation-kind policy |
| toast expiry/stickiness | Poodle adapter session | renderer policy |
| action execution | semantic reference | command registry and consumer executor |

## Rejected

- treating a cancellation receipt as terminal cancellation
- one state machine containing scans, renders, agent approvals, and Git sync
- queue scheduling in the operation core
- arbitrary product progress JSON in the generic renderer protocol
- reopening a terminal operation for retry
- operation state as notification history
- toast expiry as ledger dismissal
- closures or unchecked commands as notification actions
- renderer-owned retained operation or notification truth
- making bridge, Tauri, Svelte, Poodle, or an async runtime a pure-core
  dependency
- copying Poodle feedback components

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../architecture/system-inventory.md`
- `../../contracts/015-async-operation-lifecycle.md`
- `../../contracts/016-notification-ledger-and-projection.md`
- `../../specs/001-shared-desktop-system-suite.md`
- `../../roadmaps/g01/012-async-operations-and-notifications.md`


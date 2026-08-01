# g01.012 Async Operations And Notifications

Status: complete
Owner: Tom
Updated: 2026-07-31
Governing refs: contracts 001, 003, 007, 010, 012, 013, 015, and 016;
research memo 016

## Outcome

Ship separate optional operation and notification authorities. Preserve
Soundcheck's reconnectable scan lifecycle and Loophole's queued render plus
retained notification behavior without importing either product workflow.

## Generation Runway

This lane advances the optional-system branch after the completed bridge,
command, and linear-history foundations. It supplies reusable long-work and
user-outcome systems before consumer migration milestones. It does not start a
Soundcheck or Loophole cutover, production backend transport, native OS
notifications, or the optional history-tree implementation.

## Execution Plan

### Batch 1: Pure operation authority

- [x] Card 075: donor fixtures, identity, and lifecycle authority
- [x] Card 076: progress, cancellation, retention, and teardown

### Batch 2: Operation clients and presentation

- [x] Card 077: generated operation protocol and Tauri/bridge composition
- [x] Card 078: operation Svelte session and Poodle projection

### Batch 3: Notification authority and presentation

- [x] Card 079: retained notification ledger and operation observation
- [x] Card 080: notification clients, Svelte, Poodle, and semantic actions

### Batch 4: Artifact proof and closeout

- [x] Card 081: four-shape operation and notification artifact proof

## Goals

- [x] Fit Soundcheck plugin scan and Loophole render queue through one pure
  lifecycle authority.
- [x] Keep queue ordering, same-active reuse, execution, reports, artifacts,
  and product phases consumer-owned.
- [x] Make cancellation acceptance distinct from terminal cancellation.
- [x] Reject late progress, duplicate terminals, stale revisions, and stale
  authority epochs without changing state.
- [x] Bound active/recent projections and make every terminal eviction
  explicit.
- [x] Keep renderer remount separate from host cancellation.
- [x] Compose direct, Tauri, and bridge-domain transports without making bridge
  catalogue authority.
- [x] Keep retained notifications independent from operation state and toast
  lifetime.
- [x] Separate seen, dismissed, replaced, pruned, and transiently expired
  notification outcomes.
- [x] Use Poodle public feedback primitives without copying components.
- [x] Prove minimal dependency graphs for both systems.

## Promoted Boundary

`longhorn-operation` is a pure finite catalogue and transition authority. It
records `queued`, `running`, `cancelling`, and sticky terminal outcomes. It
does not schedule or execute work. Cancellation is a receipted request; the
executor supplies the terminal fact.

`longhorn-notifications` is a separate retained ledger. A consumer projector
may observe selected operation terminal transitions, but notifications also
represent unrelated domain facts. Operations never depend on notifications.

Generated framework-neutral packages expose checked snapshots, events,
commands, and receipts. Optional Tauri, Svelte, and Poodle edges remain
removable. Product payloads do not enter either generic renderer protocol.

## Acceptance Criteria

- [x] Two materially different strong operation donors share one API.
- [x] Queued and direct-running registration both pass.
- [x] Cancellation-success, cancellation-failure, and confirmed-cancel races
  are deterministic.
- [x] Overall progress cannot regress and terminal state rejects late updates.
- [x] Controlled teardown accounts for every non-terminal operation.
- [x] Listener-first reconnect preserves host truth; renderer remount already
  passes without host cancellation.
- [x] Loophole-shaped operation and non-operation notifications share one
  ledger without sharing an operation state machine.
- [x] Toast expiry does not dismiss retained records.
- [x] Semantic actions require fresh consumer execution admission.
- [x] Isolated packed TypeScript and offline Rust graphs prove optional edges.
- [x] No donor repository changes occur in this milestone.
- [x] Full Effigy QA passes or pre-existing unrelated findings remain exact and
  documented.

## Explicit Non-goals

- consumer repository migration
- generic executor, task runtime, or queue scheduler
- product phase, warning, report, artifact, approval, or log payloads
- durable operation resumption across process restart
- native OS notification delivery
- operation-driven notification policy chosen by Longhorn
- product command execution from notification action strings
- Poodle component forks

## Planning Checkpoint

Card 078 closes operation presentation before notification work starts.
Public Poodle progress and status primitives represent both donor shapes
without extension. Card 080 closes notification presentation before artifact proof.
Stop there if retained records and transient toast behavior cannot remain
separate in one checked client.

## Next Task

Start g01.013 native-content-island characterization. Re-audit current Nucleus,
Soundcheck, and Jetstream mechanisms before compiling a prototype seam.

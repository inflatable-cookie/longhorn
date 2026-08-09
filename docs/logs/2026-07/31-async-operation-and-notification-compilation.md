# Async Operation And Notification Compilation

Date: 2026-07-31
State: complete research and planning batch

## Outcome

- audited Soundcheck scan, sync, taxonomy, inspection, and DAW operation
  surfaces read-only
- audited Loophole render queue, Aura export host, retained notifications,
  notification panel, and transient toasts read-only
- checked Nucleus agent cancellation plus Jetstream and Split-shell weaker busy-state
  evidence
- confirmed Soundcheck scan and Loophole render queue as two strong operation
  donors
- separated queue scheduling, execution, product phases, reports, artifacts,
  retry policy, persistence, and recovery from the shared lifecycle
- made cancellation acceptance distinct from terminal cancellation
- separated retained notification records from operation state and toast expiry
- kept notification production, wording, redaction, and action execution in
  consumer authority
- confirmed Poodle public `Progress`, `StatusIndicator`, `ToastHost`, and
  `ToastStack` primitives
- promoted research memo 016 into architecture, topology, inventory, spec, and
  contracts 015-016
- compiled g01.012 into Cards 075-081
- made Card 075 the sole ready card

## Donor Findings

Soundcheck plugin scan supplies stable identity, reconnectable status,
cooperative cancellation, phases, progress, and sticky terminal outcomes.
Other Soundcheck flows use request-, plan-, or process-correlated local
variants. Product records and reconciliation remain outside Longhorn.

Loophole render queue supplies queued work, separate execution, recent jobs,
progress, cancellation races, reports, and artifact cleanup. Queue order,
pause, polling, and render policy remain outside Longhorn.

Loophole's retained host ledger also records non-operation reliability events.
Aura projects selected new records into transient toasts while the notification
panel keeps retained history. This disproves one combined operation and
notification state machine.

Nucleus corroborates cancellation and terminal behavior. Its approval, input,
provider, tool, and recovery states remain agent workflow. Jetstream and Split-shell
do not add a third strong lifecycle.

## Compiled Runway

1. Card 075 — donor fixtures, identity, and lifecycle authority
2. Card 076 — progress, cancellation, retention, and teardown
3. Card 077 — generated protocol and Tauri/bridge composition
4. Card 078 — Svelte session and Poodle operation projection
5. Card 079 — retained notification ledger and operation observation
6. Card 080 — notification clients, Svelte, Poodle, and semantic actions
7. Card 081 — isolated artifact proof and closeout

Card 075 is ready. Cards 076-081 remain planned. Cards 070-074 remain planned
behind the linear-history adoption checkpoint.

## Limits

- no donor repository changed
- no executor, queue scheduler, durable operation resume, or native OS
  notification delivery is claimed
- no product payload enters a generic renderer protocol
- operation cancellation receipt never claims stop
- notification publication cannot change operation outcome
- Poodle remains visual and toast-timer authority
- no code changed in this batch

## Validation

- focused Northstar structure and link checks
- roadmap ready/planned state checks
- `git diff --check`

## Posture

`strict-ready`

## Next

Execute Card 075. Stop if the two strong donors require queue scheduling,
product payloads, or transport request identity inside the pure authority.


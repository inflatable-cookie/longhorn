# 117 Soundcheck Plugin-scan Operation Adoption

Status: ready
Owner: Tom
Roadmap: g01.016 batch 2
Governing refs: contracts 003, 010, 012-013, and 015; Card 116
Depends on: Card 116
Auto-start next card: no

## Objective

Move the reconnectable plugin-scan lifecycle onto Longhorn operation authority
without moving scanner execution, progress meaning, reports, or recovery.

## Repository Scope

- Longhorn: operation adapters, fixtures, evidence, and docs if needed.
- Soundcheck: operation host, scan adapter, renderer projection, tests, and docs.
- soundcheck-library: read-only public scan-job authority and focused tests.

## Scope

- stable scan operation identity and catalogue registration
- queued/running/cancelling/terminal structural state
- reconnect, remount, progress, cancellation, retry, and retention
- same-active scan reuse and database-backed job reconciliation
- product progress detail and report projection
- Tauri operation capabilities and optional public-Poodle status

## Steps

1. Freeze current start/status/cancel/retry and reconnect traces.
2. Register one scan operation kind and injected executor adapter.
3. Map committed library job evidence into legal operation transitions.
4. Preserve accepted cancellation as non-terminal until the scanner resolves.
5. Reconnect fresh host truth before presenting renderer state.
6. Bind one isolated Svelte session and controlled Poodle projection.
7. Preserve product progress, warnings, unit retry, reports, and wording downstream.
8. Remove the superseded generic renderer scan-lifecycle state.

## Acceptance Criteria

- Longhorn owns structural lifecycle only
- soundcheck-library owns execution, same-active policy, reports, recovery, and database truth
- remount and second-window projections reload one host authority
- cancellation races may still terminate success or failure correctly
- retry creates or admits the intended new product attempt
- stale and late progress cannot reopen a terminal operation
- renderer teardown never cancels work implicitly
- no bridge, notification, or durable scheduler dependency is forced

## Stop Conditions

- operation state would become a second scan-job authority
- product progress payload must enter the generic protocol
- cancellation semantics differ from contract 015
- recovery requires renderer-local truth

## Next Task

Execute Card 118. Adopt isolated-window coordination around Soundcheck's
disposable plugin-inspection helper.

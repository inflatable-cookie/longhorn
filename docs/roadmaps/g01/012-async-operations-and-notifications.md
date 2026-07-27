# g01.012 Async Operations And Notifications

Status: incubation  
Owner: Tom  
Updated: 2026-07-27

## Outcome

Determine and, if justified, ship shared lifecycle primitives for long-running
desktop work and its user-visible outcomes.

## Batches

### 1. Evidence and contract

- characterize Soundcheck scan/sync/inspection flows
- characterize Loophole background operations and notifications
- separate operation state from transport and product progress payloads
- decide whether notifications share or only observe operation lifecycle

### 2. Operation kernel

- ids, queued/running/cancelling/terminal states
- progress snapshots, cancellation, stale request protection
- subscription teardown and bounded retention

### 3. UI and host adapters

- Svelte projections and Poodle progress/notification primitives
- reconnect/current-snapshot behavior
- dismiss, expire, retry, and error-detail policy where contracted

## Acceptance

- two materially different consumers fit one lifecycle
- cancellation races and late events have tests
- terminal outcomes are retained or expired by explicit policy
- no product workflow enters the generic state machine

## Stop Condition

If the second consumer only shares visual progress, keep this as Poodle usage
guidance plus small listener helpers.


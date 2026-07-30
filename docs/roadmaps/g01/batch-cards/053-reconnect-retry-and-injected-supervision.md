# 053 Reconnect, Retry, And Injected Supervision

Status: complete
Owner: Tom
Roadmap: g01.009 batch 2
Governing refs: contracts 001, 004, 007, 010, and 012; research memo 013
Depends on: Card 052
Auto-start next card: no
Completed: 2026-07-30

## Objective

Implement the connection/reconnect state machine, safe retry decisions,
authority-epoch invalidation, and injected service supervision port without
owning executable acquisition, update policy, endpoints, credentials, or
remote lifecycle.

## Scope

- explicit bridge connection state machine and transition receipts
- connect, negotiate, degrade, reconnect, offline, incompatible,
  unauthorized, fail, close, and shutdown paths
- injected clock/backoff and reconnect policy
- safe query retry and idempotent command replay admission
- indeterminate non-idempotent write outcome
- session and authority-epoch invalidation
- injected local-service supervisor port
- spawn, attach, readiness, restart, reconnect, shutdown, and failure
  observations
- opaque credential reference seam compatible with contract 004

## Public Behavior

The runtime exposes actual connection and supervision state. Reconnect creates
or negotiates a current session before accepting authority data. Old session
and authority events are rejected.

The supervisor reports consumer-owned spawn/attach outcomes. Longhorn does not
locate, download, update, or replace a service. Remote attach never claims
remote shutdown ownership.

## Out Of Scope

- production network transport
- executable discovery, installer, updater, or downloader
- endpoint discovery or pairing
- credential provider implementation
- generic offline mutation queue
- background daemon ownership

## Steps

1. Implement the validated connection state machine.
2. Add injected time, backoff, and reconnect policy.
3. Admit query retry and durable-idempotency command replay only.
4. Represent transport loss during non-idempotent write as indeterminate.
5. Invalidate old sessions and authority epochs after reconnect.
6. Define injected local spawn/attach/readiness/restart/shutdown supervision.
7. Keep remote attach lifecycle ownership external.
8. Add opaque credential references with redacted diagnostics.
9. Exercise success, outage, mismatch, unauthorized, crash, and shutdown traces.

## Acceptance Criteria

- ready follows successful negotiation and required authority checks
- reconnect cannot accept old-session or old-authority events
- query retry follows injected bounded policy
- command replay needs durable idempotency and advertised deduplication
- uncertain non-idempotent writes are indeterminate
- local supervisor state is observable and receipted
- remote attach cannot stop or replace its remote host
- service absence cannot block unrelated local Longhorn domains
- credential material appears in no config snapshot, event, error, or diagnostic
- no executable acquisition or update behavior exists

## Evidence Required

- connection transition matrix
- reconnect and stale-authority trace
- retry/idempotency/indeterminate matrix
- local spawn and attach supervision traces
- remote attach ownership trace
- credential-redaction audit
- dependency and policy audit

## Stop Conditions

- reconnect needs silent local authority fallback
- safe retry requires treating request id as idempotency
- supervision requires Longhorn-owned executable acquisition or update policy
- credentials cannot stay opaque and redacted
- a production transport decision becomes necessary

## Next Task

Card 054 is ready. Compose the completed seam into donor-shaped local,
embedded, supervised-local, and remote-attach topology fixtures.

## Result

`longhorn-bridge` now has a pure authority-gated connection machine with
monotonic transition receipts, injected clock and backoff, bounded reconnect,
explicit terminal states, and current-session/authority-epoch classification.
Reconnect invalidates authority before an adapter may retry. Query retry uses
a separate bounded controller. Transport re-entry before the scheduled
monotonic deadline is rejected. Existing command classification remains the
single replay gate: durable idempotency plus finite advertised deduplication,
otherwise an uncertain dispatch is indeterminate.

The feature-gated Rust supervisor and isolated TypeScript `/supervision`
subpath accept only consumer-injected operations. Owned local services may
spawn, restart, and shut down. External local and remote services may attach
and reconnect but cannot be replaced or stopped. Receipts expose state and
stable coded outcomes.

Credential input is limited to `BridgeCredentialRef`. Neither runtime accepts
raw credential material or arbitrary failure messages. The implementation
contains no executable path, acquisition, install, update, endpoint, network,
or remote shutdown behavior.

## Validation

- `effigy test:bridge-core`
- `effigy test:bridge-ts`
- `effigy check:bridge-ts`
- `effigy check:bridge-bindings`
- `effigy check:bridge-package`
- focused Rust Clippy and formatting
- `git diff --check`

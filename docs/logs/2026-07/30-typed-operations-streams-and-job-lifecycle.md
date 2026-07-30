# Typed Operations, Streams, And Job Lifecycle

Date: 2026-07-30
Card: 050
Roadmap: g01.009

## Result

Extended `longhorn-bridge` with the pure semantic layer above negotiation.
Domain operation names, payloads, snapshots, revisions, event meaning, and
write policy remain consumer-owned generic types.

## Operation And Failure Contract

- bounded request, durable idempotency, job, and stable error-code identities
- shared request context: request, session, and domain
- separate generic query, authoritative command, and cancellation envelopes
- optional command authority epoch, expected revision, and durable
  idempotency key
- typed query success or rejection
- typed command applied, rejected, or indeterminate terminal
- bounded failure message, stable code, failure phase, retry class, and
  optional typed detail

No generic operation-name registry or arbitrary product JSON bus was added.

## Retry And Deduplication

Query retry requires explicit adapter policy and a non-never retry class.
Uncertain commands replay only when all three facts hold:

1. the command carries a durable idempotency key
2. the failure retry class permits retry
3. the authority advertises finite deduplication

Every other uncertain command becomes indeterminate. Request correlation alone
never grants replay.

The reference deduplication ledger has validated finite capacity and never
evicts within its session lifetime. A full ledger rejects new records; it does
not silently forget an executed key.

## Ordered Projection

The pure tracker binds one current session and domain. It:

- accepts an authoritative snapshot baseline
- applies only the next sequence in the same authority epoch
- ignores duplicate, stale-epoch, stale-sequence, foreign-domain, and
  superseded-session events
- requires resnapshot after a gap or newer authority epoch
- remembers the newest cursor seen before or during snapshot load
- rejects an older snapshot as insufficient to close a listener-first race

Events remain live projections, not durable delivery.

## Optional Jobs

Progress and terminal events echo the initiating request and job identity.
The tracker accepts progress only before one correctly correlated terminal.
Cancellation receipts distinguish accepted, already terminal, unknown, and
coded rejection. Accepted cancellation does not imply termination.

## Evidence

- 15 semantic tests
- request/reply/failure serialization matrix
- command retry and indeterminate matrix
- finite no-eviction deduplication matrix
- listener-first, duplicate, stale, contiguous, gap, epoch, and superseded
  session stream matrix
- progress, cancel, terminal, and wrong-correlation matrix
- invalid and uncertain fixture trace leaves checked state unchanged
- query-only fixture carries no stream, job, or idempotency metadata

## Validation

- `effigy qa`
- `effigy test:bridge-core`
- `cargo clippy -p longhorn-core -p longhorn-bridge --all-targets -- -D warnings`
- `git diff --check`
- `effigy scan god-files --json`: no Card 050 source or fixture is high;
  Card 049 `negotiation.rs` and `bridge_contract.rs` remain high and should be
  split while Card 051 reorganizes generation surfaces

## Next

Card 051 is ready. Generate checked TypeScript and prove the same semantic
trace through direct and deterministic serialized-loopback adapters.

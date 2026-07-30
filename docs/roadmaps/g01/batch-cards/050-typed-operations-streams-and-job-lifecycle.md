# 050 Typed Operations, Streams, And Job Lifecycle

Status: complete
Owner: Tom
Roadmap: g01.009 batch 1
Governing refs: contracts 001, 007, 010, and 012; research memo 013
Depends on: Card 049
Auto-start next card: no
Completed: 2026-07-30

## Objective

Add domain-generic typed request/reply metadata, coded failures, retry classes,
ordered snapshot/event rules, and optional correlated job lifecycle without
claiming product payload authority or durable event delivery.

## Scope

- generic query, command, and cancellation envelopes
- request ids distinct from optional durable idempotency keys
- typed success, rejection, and indeterminate outcomes
- stable error code, phase, retry class, and details envelope
- authority epoch plus monotonic revision/sequence metadata
- duplicate, stale, gap, and new-epoch classification
- optional progress, cancellation receipt, and terminal job metadata
- finite replay/deduplication evidence interface
- pure semantic conformance fixture model

## Public Behavior

Domain packages instantiate generic metadata with their own payload types.
Queries and commands remain distinguishable. Commands only become replayable
when they carry a durable idempotency key and the authority advertises
deduplication.

Streams accept only current authority/session data. Gaps and epoch changes
request resnapshot. Cancellation receipts describe accepted, already terminal,
unknown, or rejected cancellation without pretending work stopped instantly.

## Out Of Scope

- operation-name registry
- consumer product payloads
- durable offline queues or durable event logs
- TypeScript, Tauri, network, or supervisor implementation
- notification presentation or durable job history

## Steps

1. Define bounded request identity and operation metadata.
2. Define generic query, command, cancellation, reply, and failure envelopes.
3. Separate request correlation from durable idempotency.
4. Define retry and indeterminate-write classification.
5. Define ordered snapshot and event cursors over authority epoch and sequence.
6. Implement duplicate, stale, gap, and new-epoch classification.
7. Define optional progress, cancel receipt, and terminal job metadata.
8. Build a pure semantic trace fixture covering request, stream, and job paths.
9. Prove invalid or uncertain writes cannot mutate the fixture projection.

## Acceptance Criteria

- domain payload types remain generic parameters owned outside Longhorn
- request id alone never enables command replay
- uncertain non-idempotent commands end indeterminate
- stale and duplicate events are ignored
- gaps and new epochs require a snapshot before later updates apply
- a superseded session cannot publish current authority state
- progress and terminal events remain request-correlated
- cancellation receipt does not overstate termination
- query-only fixtures need no stream or job capability
- invalid traces leave authoritative fixture state unchanged

## Evidence Required

- request/reply/error round-trip fixtures
- retry and indeterminate-write matrix
- ordered stream trace matrix
- progress/cancel/terminal trace matrix
- invalid-trace invariance proof
- dependency and payload-authority audit

## Stop Conditions

- generic metadata needs to parse product payload meaning
- replay safety cannot distinguish request and idempotency identity
- durable delivery is required to satisfy the live stream contract
- a job controller requires notification UI or product-specific phases

## Next Task

Card 051 is ready. Generate and validate the bridge protocol in TypeScript,
then run the semantic trace through direct and serialized loopback adapters.

## Result

`longhorn-bridge` now provides generic query, authoritative command, and
cancellation envelopes without operation-name or payload authority. Request
ids, session ids, domain ids, authority epochs, expected revisions, and
optional durable idempotency keys remain distinct types and fields.

Typed query and command replies expose success, stable coded rejection, and
explicit indeterminate command outcomes. Failures carry a bounded message,
stable code, failure phase, retry class, and optional domain-owned detail.

Command transport failure becomes replayable only with a durable idempotency
key, non-never retry class, and finite authority-advertised deduplication.
The reference ledger is bounded and no-eviction: fullness rejects new evidence
instead of making an old key falsely fresh.

The stream tracker rejects superseded sessions and foreign domains, ignores
duplicate/stale positions, applies only contiguous updates, and requires a
fresh snapshot after gaps or newer epochs. It remembers events observed before
or during snapshot load so listener-first races cannot silently lose state.

Optional progress, cancellation receipts, terminals, and the job tracker use
exact initiating request and job correlation. Cancellation acceptance does not
claim that work has stopped.

## Validation

- `effigy qa`
- `effigy test:bridge-core`
- `cargo clippy -p longhorn-core -p longhorn-bridge --all-targets -- -D warnings`
- 15 operation, retry, deduplication, stream, job, and trace fixtures
- exact round-trip, retry/indeterminate, listener-first race, stale/gap/epoch,
  cancellation, terminal, and invalid-trace matrices
- dependency and payload audit: no new dependency; payloads remain generic
- god-file scan: no Card 050 implementation or fixture crosses the high
  threshold; Card 049 negotiation and contract fixtures remain split targets
  for the generation/binding batch

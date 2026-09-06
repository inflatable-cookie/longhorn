# 023 Production Contextual Agent Tool Boundary

Status: proposed; blocked on counterpart revision and Desktop disclosure
Owner: Longhorn maintainers
Depends on: 001, 006, 007, 010, 012, 015, 019, 020, 021, 022
Related: `../specs/002-production-contextual-agent-tool-boundary.md`

## Scope

This contract defines Longhorn's transport-neutral typed host dispatch and
validation boundary. It does not define a Bovine tool catalogue, content
schema, skill, task queue, UX, transport, listener, lease, correlation kernel,
admission issuer, or write transaction. Those remain with Desktop or
Swallowtail as reconciled in `../specs/002-production-contextual-agent-tool-boundary.md`.

## Required shape (proposal)

- Separate production capability and feature boundary; contract 022 remains
  dev-only and unchanged.
- Consume Swallowtail's immutable namespaced registration snapshot and exact
  schema digest/revision; Longhorn has no registration authority.
- Validate the trusted Desktop/Swallowtail host binding: process incarnation,
  operation generation, selected capability, schema/version, and bounded
  request/response fields. PID is diagnostic only.
- Apply contract-012 compatibility rules and return typed unsupported-version
  or schema errors before callback dispatch.
- Resolve only opaque credential references under contract 021; no credential
  material enters the library's public API, records, or diagnostics.
- Dispatch through a transport-neutral typed callback after host admission.
  Task/session/attempt authority comes from Desktop and is bound by Swallowtail,
  never from model arguments.
- Enforce producer-declared numeric bounds and typed deadline, cancellation,
  concurrency, result, and error semantics. Values are producer-settleable, not
  generic operator approval gates.
- A retry creates a fresh Desktop attempt and Swallowtail operation generation;
  mutating or indeterminate calls are never transport-replayed.
- Expose least-access and approval outcomes supplied by Desktop policy without
  approving or manufacturing authority in Longhorn.
- Result and error envelopes distinguish negotiation, authentication,
  identity, admission, approval, validation, execution, cancellation,
  timeout, limit, and shutdown failures. Diagnostics are redacted.

## Attachment seam (proposal)

Swallowtail attaches its operation bridge to a transport-neutral Longhorn host
dispatch interface. Swallowtail owns the listener, operation/lease generation,
correlation, and bridge lifetime. Desktop supplies tool names, I/O schemas,
policy, bounded context, and durable admission. Longhorn supplies only typed
validation and callback dispatch. No second listener, registry, lease,
correlation lifecycle, admission issuer, or standalone daemon is permitted in
slice 1.

## Stop conditions

Stop before promotion if any proposal requires remote access, arbitrary code
execution, shell/input bridging, implicit write authority, secret-bearing
discovery, automatic replay of mutation, or a consumer schema in Longhorn.

## Required evidence

The first implementation slice must include a generic fixture and a release-
built Desktop Tauri consumer proving valid callback dispatch, schema/version
validation, trusted binding checks, credential-reference handling, bounded
cancellation/concurrency/results/errors, stale/foreign refusal, non-replayed
mutation, redacted diagnostics, joined bridge cleanup, and absence of
contract-022 dev code-execution tools. Swallowtail route/client evidence and
Desktop context-disclosure evidence remain in their owning plans.

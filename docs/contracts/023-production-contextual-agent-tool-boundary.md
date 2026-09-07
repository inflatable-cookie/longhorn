# 023 Production Contextual Agent Tool Boundary

Status: promoted for provider-free L1 only; production adoption held
Owner: Longhorn maintainers
Depends on: 001, 006, 007, 010, 012, 015, 019, 020, 021, 022
Related: `../specs/002-production-contextual-agent-tool-boundary.md`

## Scope

This contract defines Longhorn's transport-neutral typed host dispatch and
validation boundary. It does not define a Bovine tool catalogue, content
schema, skill, task queue, UX, transport, listener, lease, correlation kernel,
admission issuer, or write transaction. Those remain with Desktop or
Swallowtail as reconciled in `../specs/002-production-contextual-agent-tool-boundary.md`.

## Required shape

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

## Attachment seam

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

Production adoption, beyond L1, must include a generic fixture and a release-
built Desktop Tauri consumer proving valid callback dispatch, schema/version
validation, trusted binding checks, credential-reference handling, bounded
cancellation/concurrency/results/errors, stale/foreign refusal, non-replayed
mutation, redacted diagnostics, joined bridge cleanup, and absence of
contract-022 dev code-execution tools. Swallowtail route/client evidence and
Desktop context-disclosure evidence remain in their owning plans.

## Promotion authority and dispatch

Operator-confirmed 2026-09-07; independently accepted Desktop PR144 merged
at `2a1bad3f`. Source: Desktop
`docs/handoffs/20260907-contract063-provider-free-consumer-composition.md`.
This promotes only the provider-free L1/D1 composition below. Production
adoption, provider execution, dependency changes and releases remain held.
Coordinator `914728cd` owns dispatch: L1 first, independent exact-head review
and merge, then D1 pinned to that accepted L1 artifact.

Mutable ownership: L1 owns a new opt-in Longhorn host-dispatch source/test
surface, public API baseline and focused evidence; D1 owns Desktop agent-host
fixture source and one evidence log. Shared contracts, roadmap indexes and
dispatch closeout remain Coordinator/Chatterbox reserved. No competing writer.
Worker capability: Rust typed lifecycle and integration testing. L1 and D1
are serial; disjoint Desktop UI and Swallowtail work may continue. Escalation:
Chatterbox for boundary changes; no second registry/listener or fake admission.
Completion requires the acceptance below, focused Effigy checks, API/diff
checks and independent exact-head callback-order review before merge.

The normative L1 scope, acquisition route and acceptance are compiled in
[roadmap g02.036](../roadmaps/g02/036-production-contextual-agent-tools.md).
Contract022 remains byte-for-byte dev-only. L1 neither resolves credentials
nor exposes credential material; credential-reference production proof is held.

# 023 Production Contextual Agent Tool Boundary

Status: proposed; blocked on Swallowtail and Desktop decisions
Owner: Longhorn maintainers
Depends on: 001, 006, 007, 010, 012, 015, 019, 020, 021, 022
Related: `../specs/002-production-contextual-agent-tool-boundary.md`

## Scope

This contract defines Longhorn's production app-tool server semantics. It does
not define a Bovine tool catalogue, content schema, skill, task queue, UX, or
write transaction. Those remain Desktop-owned. Swallowtail supplies reusable
harness transport and lifecycle around the attachment seam.

## Required shape (proposal)

- Separate production capability and feature boundary; contract 022 remains
  dev-only and unchanged.
- Typed namespaced registry entries with stable tool/resource names, schema
  versions, access class, approval class, and replay classification.
- Exact instance identity composed from app identity, PID, process-start/nonce
  evidence, schema version, and protocol version. PID alone is insufficient.
- Versioned discovery and negotiation with explicit unsupported-version errors.
  Discovery contains metadata and a credential reference, never a secret.
- Credential bootstrap resolves a reference through a secure store or harness
  channel and does not write global MCP configuration.
- Every request carries bounded task/session/attempt context, deadline, and
  cancellation identity. Admission precedes registry lookup and execution.
- Bounded request/response bytes, queue depth, per-instance and per-namespace
  concurrency, and execution deadlines. Cancellation and shutdown return
  typed, observable outcomes.
- Mutating calls are not transport-replayed. Consumer policy must provide an
  explicit idempotency key or reject retry.
- Least-access by default. Approval-required calls return a typed handoff or
  refusal; the host and transport cannot approve on Desktop's behalf.
- Result and error envelopes distinguish negotiation, authentication,
  identity, admission, approval, validation, execution, cancellation,
  timeout, limit, and shutdown failures. Diagnostics are redacted.

## Attachment seam (proposal)

Swallowtail attaches a transport-neutral Longhorn server through a typed
`HostToolServer`/registry boundary. Longhorn exposes registration and dispatch
semantics; Swallowtail supplies transport, connection lifecycle, supported
harness adapters, and cleanup orchestration. The exact trait names, ownership
of bind/listen, and shutdown direction are open decisions and must be settled
before implementation.

## Stop conditions

Stop before promotion if any proposal requires remote access, arbitrary code
execution, shell/input bridging, implicit write authority, secret-bearing
discovery, automatic replay of mutation, or a consumer schema in Longhorn.

## Required evidence

The first implementation slice must include a generic fixture and a release-
built Tauri consumer proving valid invocation, schema discovery, negotiation,
identity/PID checks, stale/foreign task-session-attempt refusal,
credential-reference bootstrap, expiry/revocation, bounded cancellation,
concurrency, shutdown cleanup, typed approval handoff, non-replayed mutation,
redacted diagnostics, and absence of contract-022 dev code-execution tools.

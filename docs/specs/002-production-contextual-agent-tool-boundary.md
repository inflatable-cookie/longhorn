# 002 Production Contextual Agent Tool Boundary

Status: active provisional planning
Owner: Longhorn maintainers, with Swallowtail and Desktop decisions required
Updated: 2026-09-06
Provenance: Desktop dispatch manifest refs `d4e56c5a` and `2381579f`, supplied
by the Desktop Coordinator; the manifest is not present in this checkout
Governing evidence: `../triage/20260906-170000-bovine-production-mcp-request.md`,
`../contracts/022-agent-app-control.md`, `../architecture/repo-authority-map.md`

## Purpose

Shape a production-safe authenticated app-tool boundary without promoting the
dev-only agent-control server or moving Bovine product policy into Longhorn.
Longhorn owns app-tool server semantics. Swallowtail owns reusable harness
transport and lifecycle. Desktop owns skill discovery, context, UX, and all
domain/tool implementations.

## Proposed topology

```text
Claude / Codex / Grok / disposable MCP client
                    |
          Swallowtail harness transport
                    |
       Longhorn producer attachment seam
                    |
    Desktop-registered namespaced tools/resources
                    |
        Desktop admission + domain transactions
```

The Longhorn seam is typed and host-facing. It carries registration,
discovery, instance identity, admission context, deadlines, cancellation,
concurrency, and typed results/errors. It does not own Bovine IDs, content
permissions, task scheduling, Save transactions, Git, or skills.

## Proposed boundary

- Production is separately opted in and is never enabled by flipping contract
  022's `dev` feature.
- Every instance has an exact app identity, process identity, instance nonce,
  protocol/schema versions, and lifecycle state. PID reuse must not revive an
  old instance.
- Tools and resources register under a namespace plus stable name and expose
  versioned input/output schemas, access class, approval requirement, and
  idempotency/replay classification.
- Discovery publishes only a credential reference and connection metadata;
  credentials remain in an OS-backed or harness-owned secure store. No secret
  appears in discovery, global MCP config, transcripts, logs, or diagnostics.
- Swallowtail attaches through a transport-neutral `HostToolServer` seam and
  owns socket/process/harness lifecycle. Longhorn owns admission and dispatch
  semantics once a call reaches the host.
- Calls carry task, session, and attempt admission context. The producer
  rejects wrong instance, stale task/session/attempt, revoked capability,
  expired deadline, and foreign context before domain dispatch.
- Deadlines are monotonic and bounded. Cancellation is explicit and observable;
  shutdown drains or refuses new work and leaves typed terminal evidence.
- Concurrency is bounded per instance, namespace, and caller scope. Results
  are typed, size-bounded, redacted where needed, and distinguish transport,
  admission, approval, execution, and cancellation errors.
- Mutating calls are never replayed automatically. Retry requires a new
  attempt and an explicit consumer idempotency policy; transport retries do not
  imply domain re-execution.
- Least-access registration is the default. Approval handoff is a typed
  boundary to Desktop policy; transport cannot approve or manufacture write
  authority.

## Acceptance sequence

1. Longhorn publishes the contract and generic disposable producer fixture.
2. Swallowtail attaches its reusable transport and lifecycle adapter.
3. A release-built Tauri consumer mounts one read-only registered fixture tool.
4. Each real harness client—Claude, Codex, and Grok—connects through its
   supported adapter; a disposable MCP client repeats the same wire checks.
5. The sequence proves discovery by credential reference, version negotiation,
   exact instance/PID matching, valid invocation, wrong-instance and stale
   context rejection, expiry/revocation, bounded cancellation, concurrency,
   shutdown cleanup, schema errors, and no secret leakage.
6. The same consumer proves that mutating calls are not replayed and that an
   approval-required call stops at Desktop's approval boundary.
7. Release absence proves no dev `evaluate`, synthetic input, shell, or
   arbitrary command bridge is pulled into the production artifact.

## Open decisions before promotion

- Swallowtail attachment trait and ownership of listener/process lifecycle.
- Exact credential-reference protocol and secure-store owner.
- App identity format and PID-reuse-resistant instance identity.
- Version negotiation policy and compatibility window.
- Admission-context issuer and authoritative task/session/attempt model.
- Approval handoff vocabulary and whether the harness may surface pending
  approval or only receive a refusal.
- Retry/idempotency rules for read versus mutating tools.
- Per-call and aggregate size, deadline, queue, and concurrency limits.
- Required support matrix for Claude, Codex, Grok, and disposable clients.
- Release artifact shape and consumer Tauri feature wiring.

Until these are decided, this spec is planning authority only and the
milestone below remains blocked.

## Planned producer batches

- Contract and seam settlement: promote the durable boundary and freeze
  Swallowtail/Longhorn ownership.
- Core registry and admission fixture: typed namespaced schemas, discovery,
  identity, negotiation, credentials by reference, admission, limits, errors,
  cancellation, and no-replay semantics.
- Tauri producer mount and release proof: consumer mount, lifecycle cleanup,
  release absence, and one generic read-only tool.
- Cross-repo harness acceptance: Swallowtail adapter plus real clients and a
  disposable MCP client, with Desktop-only domain composition deferred to its
own lane.

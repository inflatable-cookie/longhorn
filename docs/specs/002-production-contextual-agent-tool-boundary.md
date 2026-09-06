# 002 Production Contextual Agent Tool Boundary

Status: active provisional planning
Owner: Longhorn maintainers, reconciled with Swallowtail and Desktop ownership
Updated: 2026-09-07
Provenance: Desktop dispatch manifest refs `d4e56c5a` and `2381579f`; canonical
Desktop decision `30a338f2` in `bovine-accelerator-desktop/docs/specs/010-contextual-chat-and-task-queue.md`
Counterpart: Swallowtail PR [#254](https://github.com/inflatable-cookie/swallowtail/pull/254),
reviewed proposal head `8276f0a99b2f1d24be6edc2d6a9efe38848df399`
Governing Longhorn evidence: `../triage/20260906-170000-bovine-production-mcp-request.md`,
`../contracts/022-agent-app-control.md`, `../architecture/repo-authority-map.md`

## Purpose

Shape a production-safe authenticated app-tool boundary without promoting the
dev-only agent-control server or moving Bovine product policy into Longhorn.
This revision follows the Desktop ownership decision: Swallowtail owns the
registered capability snapshot and operation bridge kernel; Longhorn owns only
transport-neutral typed host dispatch and validation; Desktop owns tool/domain
policy, context disclosure, admission issuance, and packaging.

## Bilateral reconciliation

| Surface | Longhorn proposal | Swallowtail proposal | Desktop statement | Active authority | Decision status |
| --- | --- | --- | --- | --- | --- |
| Registry identity and schema discovery | Consume one immutable typed snapshot; no registry, aliases, or identity issuer | PR254 spec 014 assigns the portable namespaced registration snapshot, schema digest/revision, and selection to Swallowtail | Spec 010 at `30a338f2` assigns the immutable executable catalogue to Swallowtail | Swallowtail; Longhorn validates the admitted snapshot | Aligned by operator direction; PR254 must retain this and not duplicate it |
| Tool input/output schemas | Validate Desktop-supplied schemas and invoke typed callbacks; no schema authority | PR254 spec 014 currently lists input/output schemas under Longhorn, conflicting with Desktop | Spec 010 assigns names, I/O schemas, effects, and business policy to Desktop | Desktop owns schemas; Longhorn owns generic validation only | Genuine source conflict; Swallowtail revision required before promotion |
| Transport, listener, lease, and correlation | Attach to an already authenticated operation binding; no listener, lease, bearer, or correlation lifecycle | PR254 spec 014 proposes one shared Contract060 operation bridge kernel and registered-server profile | Spec 010 assigns provider attachment, listener, transport, operation lease, and bridge lifecycle to Swallowtail | Swallowtail Contract060 kernel; `WatcherBridge` stays closed-compatible | Aligned if PR254 amends C060 without widening WatcherBridge or creating a second listener |
| Replay classification and admission | Consume trusted host binding; validate fields and dispatch; never infer authority from model args | PR254 spec 014 proposes effect/replay posture and exact operation/turn binding | Spec 010 makes Desktop issue durable task/attempt admission; fresh attempt on retry; no mutating replay | Desktop issues admission; Swallowtail binds operation generation; Longhorn validates | Retry/no-replay is already aligned; no new Longhorn authority |
| Desktop context disclosure | Accept only bounded, already-admitted context; do not discover or broaden it | PR254 spec 014 says Desktop chooses context, skills, and references; Swallowtail validates bounds/digest | Spec 010 bounds selected workspace context, viewing/task separation, IDs, revisions, and exclusions | Desktop | Aligned; Desktop must publish the disclosure record and bounds |

The exact incompatibility is the current PR254 spec 014 ownership row for tool
input/output schemas: it assigns them to Longhorn while Desktop `30a338f2`
assigns them to Desktop. The operator direction resolves the intended split but
does not itself amend Swallowtail. PR254 must revise that row before either
producer plan can promote.

Contract 060 is not a consumer-tool transport. Its active contract,
`swallowtail/docs/contracts/060-operation-scoped-watcher-http-bridge.md`, is a
closed WatcherBridge profile. It must remain compatible while its lease,
listener, correlation, and operation lifecycle are reused or factored for a
future registered-server profile. Longhorn must not add a second listener,
registry, lease manager, correlation kernel, or admission issuer.

## Settled Longhorn planning surface

The following six items are Longhorn-owned planning decisions, not cross-repo
operator gates:

1. **Typed host dispatch API:** a transport-neutral callback/validator boundary
   consumes Swallowtail's admitted snapshot and binding; no mutable registry.
2. **Credential references:** use the consumer-scoped opaque slot/reference
   discipline of Longhorn contract 021; Longhorn never receives or publishes
   credential material.
3. **Protocol/schema validation:** apply contract 012 compatibility rules to
   producer-declared versions and schema digests; unsupported versions fail
   typed before dispatch.
4. **Bounded result/error semantics:** define typed validation, execution,
   cancellation, deadline, overflow, and unknown-outcome envelopes; numeric
   limits and protocol evidence are producer-settleable details, not generic
   operator approval gates.
5. **No-replay dispatch rule:** a retry uses a fresh Desktop attempt and fresh
   Swallowtail operation generation; mutating or indeterminate calls are never
   transport-replayed. This is already aligned across the current proposals.
6. **Release absence and API compatibility evidence:** prove the Longhorn host
   library is separately opted in, contains no contract-022 dev code-execution
   surface, and adds no standalone daemon. Desktop owns packaging, distribution,
   and startup composition.

## Identity and lifecycle

Desktop's process incarnation plus Swallowtail's operation/lease generation
are the authority. PID is diagnostic only. One Desktop durable task+attempt maps
one-to-one to one Swallowtail operation/turn attempt. Trusted task/attempt
binding comes from host admission and is never accepted from model arguments or
provider-supplied IDs. Startup opens the Swallowtail bridge before dispatch;
termination joins it through the existing operation lifecycle. A restart or
fresh attempt cannot revive an old binding.

## Distribution and acceptance

Slice 1 is an in-process Desktop host that links/packages the Longhorn host
library. It has no standalone Longhorn daemon. Swallowtail owns bridge lifetime
inside that host; Desktop owns application startup/shutdown and distribution.

Longhorn's producer evidence proves the typed dispatch/validation fixture,
schema/version refusal, bounded results/errors, cancellation/deadline and
concurrency handling, stale/foreign binding refusal, no-replay behavior, and
release absence. Swallowtail owns the route matrix and client support evidence;
do not duplicate it here. The bilateral acceptance sequence is: Desktop
provides one read-only tool and admitted context; Swallowtail attaches its
reviewed bridge; Claude, Codex, Grok, and a disposable MCP client each run the
route-specific supported sequence; then the packaged Desktop proof records
valid result, refusal, cleanup, and no secret leakage.

## Open decisions before promotion

- Swallowtail must amend PR254/spec014 and Contract060 consistently, preserving
  WatcherBridge's closed profile and one listener/lease/correlation kernel.
- Desktop must publish the bounded app-context disclosure record and its exact
  task/session/attempt admission schema through spec 010.
- The counterpart route matrix remains Swallowtail-owned; Longhorn consumes its
  released evidence rather than reopening Claude/Codex/Grok support as a
  Longhorn gate.

Until those bilateral revisions land, this spec and g02.036 remain planning
authority only. No producer contract promotion, runtime implementation, or
release follows from this document.

## Planned producer batches

- Longhorn typed dispatch/validation contract and generic fixture.
- Swallowtail C060-compatible registered-server bridge profile and route
  evidence, owned in Swallowtail.
- Desktop packaged host composition and bounded context/admission fixture,
  owned in Desktop.
- Bilateral real-client/disposable acceptance and independent review.

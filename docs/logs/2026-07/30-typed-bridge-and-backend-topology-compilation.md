# Typed Bridge And Backend Topology Compilation

Date: 2026-07-30
State: complete research and planning batch

## Outcome

- re-audited Nucleus, Loophole, Jetstream, Soundcheck, and Split-shell read-only
- separated structural bridge protocol from consumer domain payload authority
- separated host connection, capability advertisement, authentication posture,
  execution ownership, and domain write authority
- fixed exact v1 negotiation, ordered snapshot/event, correlated job,
  cancellation, retry, idempotency, and indeterminate-write rules
- rejected a generic offline mutation queue
- kept service acquisition, installation, update, endpoint, credential, and
  remote lifecycle policy consumer-owned
- selected direct, Tauri, and deterministic serialized loopback as the v1
  executable/conformance set
- deferred production network transport until consumer security and
  cross-platform evidence exists
- promoted memo 013 into architecture and compiled contracts 007 and 010
- compiled g01.009 into Cards 049-055
- made Card 049 the sole ready card

## Donor Evidence

Nucleus supplies the strongest separation between engine authority, host form,
capability, execution, and per-project domain authority. Loophole proves that
embedded, brokered, and remote forms must preserve Pulse semantics while
lifecycle ownership changes.

Jetstream supplies the listener-before-current-snapshot handshake and coherent
whole-state projection. Soundcheck supplies request-correlated progress,
cancellation, terminal cleanup, optional services, and local-service hosting.
Split-shell supplies the zero-event, zero-service request/reply floor.

The donors do not prove one production local/remote network protocol. Nucleus
lists candidates but has no production remote transport. Loophole remote
profiles are additive while the current local posture is embedded. Soundcheck
uses HTTP for an optional product integration, not a generic authority seam.

## Contract Decisions

- Rust owns generic bridge metadata and generated checked TypeScript.
- Domain packages own operation names, payloads, validation, snapshots,
  revisions, event meaning, and write policy.
- v1 negotiates one exact bridge version.
- capability advertisement never grants authority.
- request ids correlate; durable idempotency keys permit replay only with
  advertised deduplication.
- uncertain non-idempotent writes are indeterminate.
- listener-first streams use authority epoch plus monotonic revision/sequence;
  gaps and epoch changes resnapshot.
- progress and terminal events are optional and request-correlated.
- query-only domains require no event transport.
- offline projections never become write authority.
- Longhorn accepts injected service supervision but owns no downloader,
  installer, updater, endpoint selection, or remote shutdown.

## Compiled Runway

1. Card 049 — bridge identity, negotiation, and authority protocol
2. Card 050 — typed operations, streams, and job lifecycle
3. Card 051 — generated bridge client and direct/loopback conformance
4. Card 052 — Tauri bridge host and client assembly
5. Card 053 — reconnect, retry, and injected supervision
6. Card 054 — five-shape topology conformance
7. Card 055 — artifact proof and closeout

Card 049 is ready. Cards 050-055 remain planned so implementation cannot outrun
the evidence from the preceding card.

## Limits

- no consumer repository was modified
- no product payload or operation was moved into Longhorn
- no production service transport or endpoint-security claim was made
- no authentication provider or discovery system was selected
- no offline mutation queue was invented
- no public package name or compatibility range was claimed

## Validation

- focused g01.009 Northstar path checks passed
- documentation links and indexes passed
- full Northstar QA passed
- `git diff --check` passed
- one ready card and six planned cards are indexed
- no code changed, so the Rust and frontend suites were not repeated

## Posture

`strict-ready`

## Next

Execute Card 049. Stop if authority descriptors require consumer payloads or
if topology selection leaks into the pure protocol.

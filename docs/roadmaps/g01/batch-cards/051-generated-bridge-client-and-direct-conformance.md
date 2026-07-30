# 051 Generated Bridge Client And Direct Conformance

Status: complete
Owner: Tom
Roadmap: g01.009 batch 2
Governing refs: contracts 001, 007, 010, and 012; research memo 013
Depends on: Card 050
Auto-start next card: no
Completed: 2026-07-30

## Objective

Generate the checked TypeScript bridge protocol, implement a framework-neutral
negotiated session client, and prove semantic parity through direct and
deterministic serialized-loopback adapters.

## Scope

- split Card 049 negotiation and contract fixtures along protocol boundaries
  before adding generation code
- `longhorn-bindings` bridge generation and golden fixtures
- `@longhorn/bridge`
- strict compatibility validation for negotiation, authority, operations,
  streams, jobs, and failures
- injected domain codecs and operation clients
- direct in-memory adapter
- JSON serialized-loopback adapter with explicit encode/decode boundary
- listener-first checked stream session
- exact teardown and late-registration disposal
- shared Rust/TypeScript semantic trace

## Public Behavior

The client negotiates before exposing domain capability or authority. Domain
packages inject checked codecs; `@longhorn/bridge` does not accept unchecked
product JSON as authoritative state.

The direct and loopback adapters produce identical replies, stream decisions,
job lifecycle, failures, and connection state. Loopback proves serialization,
not a production network protocol.

## Out Of Scope

- Tauri
- Svelte or Poodle
- production service transport
- supervisor lifecycle
- product domain clients
- generated consumer payloads

## Steps

1. Split the high negotiation and contract files without changing public
   behavior.
2. Add bridge generation and zero-drift task coverage.
3. Generate exact TypeScript protocol types and constants.
4. Implement strict compatibility validators.
5. Implement negotiated session and authority projection.
6. Add injected typed domain codec/operation seams.
7. Implement direct execution and event delivery.
8. Implement deterministic JSON serialized loopback.
9. Reuse the checked snapshot lifetime for listener-first streams.
10. Run one cross-language semantic trace through both adapters.
11. Audit package exports, payload ownership, and optional dependencies.

## Acceptance Criteria

- generated TypeScript is deterministic and zero-diff checked
- unknown versions, states, retry classes, and authority shapes fail explicitly
- no handwritten duplicate bridge DTO is required
- direct and loopback traces have identical semantic outcomes
- serialization failures surface at the adapter boundary
- listener-before-snapshot cannot miss intervening current state
- late registration disposal leaks no listener
- query-only use imports no event adapter
- package root imports no Tauri, Svelte, Poodle, service, or consumer

## Evidence Required

- generated artifact and drift report
- compatibility rejection matrix
- cross-language golden trace
- direct-versus-loopback parity report
- listener lifetime and teardown fixtures
- dependency, export, and payload audit
- god-file scan with the Card 049 bridge findings cleared

## Stop Conditions

- TypeScript requires hand-maintained wire DTOs
- generic client must understand a product payload
- loopback behavior depends on network timing not represented by the contract
- package exports pull an optional host or UI runtime into the root

## Next Task

Card 052 is ready. Adapt the negotiated protocol to Tauri through narrow
registered-domain host assembly and the existing domain-free raw transport.

## Result

`longhorn-bindings` now emits the exact bridge v1 TypeScript protocol and a
Rust-owned golden fixture. The fixture includes negotiation, authority,
query/command, ordered stream, job, incompatibility, and semantic-trace
evidence. Generation is deterministic and zero-drift checked.

`@longhorn/bridge` now negotiates before exposing a session. Exact
compatibility validators reject unknown versions, states, retry classes,
authority shapes, outcomes, and correlation metadata. Domain codecs, routes,
operation names, and payload meaning remain injected domain authority.

Direct and deterministic JSON-loopback adapters share one host router and
produce identical semantic traces. Loopback reports encode and decode failure
at the adapter boundary. The optional stream export reuses the checked
listener-first snapshot lifetime, handles late registration disposal, and is
absent from query-only root imports.

The Card 049 negotiation and contract fixtures are split by protocol boundary.
No Card 049 or Card 051 source or fixture remains above the god-file high
threshold.

## Validation

- `effigy qa`
- `effigy check:bridge-bindings`
- `effigy test:bridge-core`
- `effigy test:bridge-ts`: 11 tests, 80 expectations
- `effigy check:bridge-package`
- `cargo clippy -p longhorn-bridge -p longhorn-bindings --all-targets -- -D warnings`
- 13 negotiation and authority contract tests
- direct/loopback parity, cross-language trace, rejection, listener lifetime,
  import-safety, package, dependency, export, and payload-authority audits

# 052 Tauri Bridge Host And Client Assembly

Status: complete
Owner: Tom
Roadmap: g01.009 batch 2
Governing refs: contracts 001, 007, 010, and 012; research memo 013
Depends on: Card 051
Auto-start next card: no
Completed: 2026-07-30

## Objective

Implement narrow Tauri bridge host and client assembly over registered domain
handlers, the generated bridge protocol, and the existing domain-free raw
transport.

## Scope

- `longhorn-tauri-bridge`
- bridge hello, authority snapshot, query, command, cancel, and resync handlers
- registered typed domain handler traits
- stable `longhorn_bridge_*` command and `longhorn://bridge/*` event names
- request/session/authority checks at dispatch
- optional event emitter and query-only assembly
- mock-runtime handler proof
- `@longhorn/bridge` composition over `@longhorn/tauri`
- minimal Tauri capability examples

## Public Behavior

One handler assembly is used by real and mock Tauri hosts. Registered domains
keep their typed payload codecs and authority. Tauri capability files admit
commands at the platform edge but never replace bridge or domain
authorization.

A query-only host registers no event channel. A subscription host emits only
checked current-session metadata and supports explicit resync.

## Out Of Scope

- product commands or payloads
- app-specific Tauri setup
- service/network transport
- Svelte, Poodle, or settings UI
- endpoint authentication
- consumer migration

## Steps

1. Add the narrow Tauri adapter crate.
2. Define injected domain handler and authority provider traits.
3. Assemble hello, authority, operation, cancellation, and resync handlers.
4. Validate session, capability, authority epoch, and request metadata.
5. Add optional checked event emission.
6. Compose the TypeScript bridge client over `TauriTransport`.
7. Exercise the same assembly in a Tauri mock runtime.
8. Add query-only and subscription-capable fixtures.
9. Publish minimal capability examples and audit command exposure.

## Acceptance Criteria

- real and mock hosts use one assembly function
- raw invoke/listen stays in `@longhorn/tauri`
- domain operation names and payloads stay in registered domain adapters
- invalid session, capability, authority, or request metadata reaches no handler
- query-only Bovine shape registers and imports no event support
- subscription fixture follows listener-before-snapshot ordering
- cancellation and terminal events remain request-correlated
- Tauri capability admission does not grant domain authority
- adapter crates import no consumer, Svelte, or Poodle package

## Evidence Required

- handler registration and rejection matrix
- mock-runtime direct/Tauri parity trace
- query-only dependency and command inventory
- subscription and resync trace
- capability and authority audit
- focused Rust, TypeScript, and Effigy checks

## Stop Conditions

- Tauri command registration must own product operation vocabulary
- mock and real hosts require different handler paths
- a query-only composition must include events
- Tauri capability policy must substitute for domain authorization

## Next Task

Card 053 is ready. Add reconnect, indeterminate-write, authority-epoch, and
injected supervision lifecycle without selecting a production transport.

## Result

`longhorn-tauri-bridge` now assembles registered typed domain query, command,
cancellation, and snapshot handlers behind stable generic Tauri commands.
Negotiation receipts must describe the registered Tauri-local host exactly.
Caller session, route/domain metadata, capability, read/write/execution
authority, and authority epoch are checked before dispatch.

One type-erased command service feeds direct and mock/real Tauri hosts. Event
publication is an injected edge. Query-only assembly has no sink and rejects
publication; subscription assembly publishes checked domain, progress, and
terminal envelopes.

`@longhorn/tauri` now separates its invoke-only root from optional `/events`.
`@longhorn/bridge/tauri` composes checked sessions and domain descriptors over
the invoke-only transport. `/tauri-events` adds listener-first snapshot
resync and checked request/job-correlated listeners.

The capability examples separate query-only admission from event-capable
admission. They grant Tauri reachability only; bridge authority remains
negotiated and checked in the host.

## Validation

- `effigy test:tauri-bridge`: 6 host, capability, and mock-runtime proofs
- `effigy test:bridge-ts`: 14 tests, 93 expectations
- `effigy test:tauri-ts`: 4 tests, 18 expectations
- `effigy check:bridge-ts`
- `effigy check:tauri-ts`
- `effigy check:bridge-package`
- `effigy check:tauri-package`
- `cargo clippy -p longhorn-tauri-bridge --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `git diff --check`
- `effigy scan god-files`: no Card 052 high finding

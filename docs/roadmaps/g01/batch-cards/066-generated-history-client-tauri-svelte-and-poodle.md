# 066 Generated History Client, Tauri, Svelte, And Poodle

Status: planned
Owner: Tom
Roadmap: g01.011 batch 2
Governing refs: contracts 007, 008, 010, 012, and 013; research memo 015
Depends on: Card 065
Auto-start next card: no

## Objective

Generate the metadata-only history protocol and add checked framework-neutral,
Tauri, Svelte, and public-Poodle composition without exposing product payloads
or creating a second history authority.

## Scope

- Rust-authoritative snapshots, entry pages, commands, receipts, and errors
- exact TypeScript compatibility checks
- framework-neutral direct and transport clients
- registered injected `longhorn-tauri-history` handler assembly
- listener-before-snapshot lifecycle where events are composed
- per-instance `/svelte` session state
- controlled `/poodle` linear history panel
- undo, redo, checkout, filtering, loading, error, and teardown behavior
- capability examples

## Public Behavior

Clients see history revision, mode, depths, current id, next labels,
authoritative entry pages, and exact navigation receipts. They never receive
the typed product payload.

The Tauri adapter invokes a registered consumer history authority. It checks
capability reachability but does not replace product authorization. Svelte
state is per instance. Poodle supplies public visual primitives.

## Out Of Scope

- product labels or icons
- generic product execution bus
- branch visualization
- persisted renderer state
- donor UI migration

## Steps

1. Generate the metadata protocol and golden fixture.
2. Add exact compatibility and framework-neutral clients.
3. Prove direct and serialized transport parity.
4. Add registered injected Tauri handler assembly and capabilities.
5. Add epoch/revision-safe snapshot and event lifecycle.
6. Add per-instance Svelte loading, navigation, pagination, and error state.
7. Compose a controlled Poodle linear panel from public primitives.
8. Prove teardown, stale response, gap, and authority replacement behavior.
9. Audit payload, capability, peer, subpath, and visual authority boundaries.

## Acceptance Criteria

- generated protocol contains no consumer payload
- direct and serialized clients produce equal semantics
- stale revision rejects and refreshes authoritative state
- listener-before-snapshot cannot miss a committed transition
- Tauri capability does not grant product authorization
- Svelte instances do not share state
- Poodle composition uses only public controlled APIs
- root import resolves no Tauri, Svelte, or Poodle dependency
- teardown disposes listeners and rejects late results

## Evidence Required

- Rust/TypeScript golden fixture
- direct/serialized conformance trace
- Tauri capability and authorization matrix
- Svelte lifecycle and SSR fixtures
- Poodle public-API proof
- payload and dependency audits
- focused Rust, TS, Svelte, package, and Effigy checks

## Stop Conditions

- product payload must cross into TypeScript
- Tauri handlers become product mutation authority
- renderer state becomes durable history
- a private or copied Poodle component is required

## Next Task

Card 067 is planned. Prove rich and minimal linear compositions from produced
artifacts, publish guidance, and pause before fork work.

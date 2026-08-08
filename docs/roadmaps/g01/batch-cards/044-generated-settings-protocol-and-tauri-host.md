# 044 Generated Settings Protocol And Tauri Host

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.008 batch 2
Governing refs: contracts 005, 010, and 012; research memo 012
Depends on: Card 043
Auto-start next card: no

## Objective

Generate checked TypeScript settings bindings, add a framework-neutral client,
and expose the pure authority through a narrow injected Tauri host.

## Scope

- settings slice in `longhorn-bindings`
- golden registry, snapshot, command, receipt, conflict, policy, recovery, and
  activation fixtures
- private `@inflatable-cookie/longhorn-settings` framework-neutral root
- registry projection, deterministic search, and structural deep-link helpers
- listener-first checked client over `@inflatable-cookie/longhorn-core`
- `longhorn-tauri-settings` handler assembly
- explicit command/event names and narrow capability examples
- direct and serialized conformance

## Public Behavior

Rust remains protocol authority. The TypeScript client validates compatibility,
registry generation, scope revision, authority tokens, bounded opaque values,
and all discriminants.

The Tauri crate adapts injected registries and authorities. It owns no
configuration schema, product page, or global singleton. Events are revision
hints; the client reloads authoritative state under the Card 036 lifetime.

## Out Of Scope

- Svelte or Poodle
- page rendering
- storage/backup/restore clients
- command/keymap or backend packages
- public registry publication

## Steps

1. Add deterministic Rust-to-TypeScript generation for settings protocol
   types.
2. Emit golden fixtures for every command, outcome, and failure discriminant.
3. Add compatibility guards and package-safe root exports.
4. Implement registry projection, search, and deep-link resolution.
5. Implement listener-first registry and scope connections.
6. Add injected Tauri command handlers and event publication.
7. Add minimal capability examples for read-only and mutable settings hosts.
8. Prove direct and serialized handler/client conformance.
9. Audit import safety, package contents, payload limits, and dependency graph.

## Acceptance Criteria

- generated TypeScript and golden fixtures are current
- unknown versions and variants fail safe
- root import touches no browser global
- search uses registered labels/keywords and deterministic ordering
- deep links resolve page id and stable anchor without DOM inspection
- listeners attach before initial registry and scope snapshots
- stale generation or scope revision cannot replace newer authority
- late unlisten runs exactly once
- Tauri host accepts injected authorities only
- capabilities expose only selected commands and event listen/unlisten
- package imports no layout, Surface, command, history, backend, Svelte, or
  Poodle dependency

## Evidence Required

- generated drift check
- cross-language fixture matrix
- direct/serialized conformance
- connection race and teardown fixtures
- capability and payload audit
- package dry run and dependency report
- TypeScript, Rust, and Effigy QA

## Stop Conditions

- generated values must interpret product schema
- Tauri handler must own application configuration
- events would carry mutable authority instead of hints
- search requires rendered page inspection
- package root gains an optional-system dependency

## Next Task

Card 045 is ready but not started. Add per-instance session state and the
public-Poodle shell without changing authority.

## Result

`longhorn-settings` now generates one checked TypeScript protocol and golden
fixture covering registry, authority, mutation, conflict, policy, recovery,
activation, durability, events, and future incompatibility.

The private framework-neutral `@inflatable-cookie/longhorn-settings` package adds strict
compatibility guards, deterministic registry projection, search and structural
deep links, checked commands, and listener-first registry and scope
connections. Stale authority cannot replace newer state, late unlisten runs
once, and the root imports no browser or optional-system global.

`longhorn-tauri-settings` adapts one injected consumer authority through four
commands and two non-durable hint events. Caller authorization and product
semantics remain consumer-owned. Capability examples expose only the selected
commands plus event listen/unlisten.

Evidence:
`../../../logs/2026-07/29-generated-settings-protocol-and-tauri-host.md`.

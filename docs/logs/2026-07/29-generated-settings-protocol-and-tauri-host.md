# Generated Settings Protocol And Tauri Host

Date: 2026-07-29  
Card: g01.008 / 044  
Status: complete

## Outcome

Added checked Rust-generated TypeScript settings bindings, framework-neutral
registry/client behavior, and a narrow injected Tauri host.

## Protocol

`longhorn-settings` now projects one sealed registry snapshot plus registry and
scope revision hints. Rust generation covers:

- registry declarations, limits, digest, generation, and admitted capabilities
- scope values, policy, recovery, activation, revision, and authority token
- load, apply, reset, receipt, conflict, rejection, and durability envelopes
- every enum, tagged result, and failure discriminant
- golden direct and future-incompatibility fixtures

`@inflatable-cookie/longhorn-settings` validates exact protocol versions, safe revisions,
bounded ids, registry limits, opaque codec envelopes, authority tokens, and
all discriminants.

## Client

The private framework-neutral package provides:

- deterministic navigation projection
- label/keyword/anchor search in registered order
- structural page and anchor deep links without DOM inspection
- checked registry, load, apply, and reset commands
- listener-before-snapshot registry and scope connections
- stale generation/revision rejection
- same-revision digest/token consistency checks
- bounded refresh coalescing and exactly-once late unlisten through
  `@inflatable-cookie/longhorn-core`

The root imports no browser global or optional Longhorn system.

## Tauri Host

`longhorn-tauri-settings` provides one mutex-protected assembly over an
injected `SettingsAuthority`. Caller authorization and product semantics stay
with that authority.

The adapter exposes four commands and two hint events. Changed mutation
receipts project scope hints. Unchanged, conflict, and rejected outcomes do
not. Event delivery remains non-durable and cannot erase the invoking
client's durable mutation receipt.

Read-only and mutable capability examples grant only selected commands plus
event listen/unlisten.

## Evidence

- 12 TypeScript protocol, registry, race, teardown, serialized-client, import,
  and package tests
- 22 pure Rust settings tests
- 7 injected-handler, serialized, authorization, event, capability, and
  dependency tests
- generated drift check
- TypeScript compile and package dry run
- Rust formatting and focused Clippy
- Cargo dependency report
- payload and forbidden-dependency audit
- full Effigy QA

All pass.

The god-file scan stays at the exact baseline: 59 findings, 58 warnings, and
one high finding in
`crates/longhorn-tauri-windowing/src/lifecycle/model.rs`. Card 044 adds no
finding.

## Limits

- registry recomposition replaces a generation; it is not live mutable state
- events are invalidation hints, not mutable authority or durable delivery
- opaque product values remain consumer codecs
- settings Tauri commands do not assemble application configuration
- Svelte session state and Poodle presentation remain Card 045

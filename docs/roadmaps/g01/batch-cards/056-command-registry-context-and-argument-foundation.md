# 056 Command Registry, Context, And Argument Foundation

Status: complete
Owner: Tom
Roadmap: g01.010 batch 1
Governing refs: contracts 001, 006, 010, and 012; research memo 014
Depends on: Card 055
Auto-start next card: no
Completed: 2026-07-30

## Objective

Implement the pure bounded command/context registry, closed v1 argument schema,
deterministic discovery projection, and search semantics without execution,
configuration, bridge, Tauri, renderer, or product dependencies.

## Scope

- `longhorn-command` crate
- bounded command, context, category, keyword, route, capability, field, and
  enum-value identities
- command and context declarations
- one `global` root and finite parent-chain validation
- closed no-arguments or bounded-object argument schema
- structural argument validation and normalized values
- sealed registry generation, deterministic digest, ordering, and projection
- palette/menu/shortcut/settings/help visibility metadata
- deterministic shared search and ranking
- Loophole-shaped and Jetstream-shaped registry fixtures

## Public Behavior

Registration is mutable only before sealing. Sealing rejects duplicate or
unknown ids, invalid context graphs, unbounded metadata, invalid argument
schemas/defaults, and unknown capability references.

The sealed registry is immutable for one generation. Runtime focus and
availability do not alter its digest. Search uses command id, label, category,
description, and keywords with stable tie-breaking.

Argument validation admits only declared fields and returns one normalized
bounded value. Product semantic validation remains outside this crate.

## Out Of Scope

- availability or execution
- keyboard triggers, presets, or overrides
- configuration or migration
- TypeScript generation
- Tauri, bridge, settings, Svelte, or Poodle
- product command catalogues and context facts

## Steps

1. Add the pure crate and bounded identity types.
2. Define command, context, visibility, capability, and discovery metadata.
3. Define the closed v1 argument schema and normalized value model.
4. Validate fields, defaults, bounds, enum values, and unknown input.
5. Validate one rooted finite context graph and declaration references.
6. Seal deterministic registry generation, digest, and ordering.
7. Project search records and implement canonical ranking.
8. Add rich Loophole and minimal Jetstream fixtures.
9. Audit dependencies, serialization, limits, and public API.

## Acceptance Criteria

- duplicate or malformed ids fail before sealing
- contexts form one bounded acyclic graph rooted at `global`
- unknown contexts and capabilities fail registration
- no-argument and bounded-object commands validate deterministically
- nested values, arrays, unknown fields, non-finite numbers, and invalid
  defaults fail
- registry generation and digest are stable across insertion order
- search ordering is stable and uses one command identity source
- hidden and surface-specific visibility is explicit
- Loophole and Jetstream fixtures share the crate without product types
- the crate depends only on core plus serialization/digest utilities

## Evidence Required

- registry success and rejection matrix
- argument schema/default/input validation matrix
- context graph validation fixtures
- digest and insertion-order invariance
- search ranking fixtures
- two donor-shaped catalogues
- dependency and public-API audit
- focused Rust and Effigy checks

## Stop Conditions

- arbitrary JSON is required for the shared argument contract
- current product state is needed to seal the registry
- a context graph requires Loophole selection or editor types
- search requires client-specific presentation state
- a host, bridge, or UI dependency enters the pure crate

## Next Task

Card 057 is ready. Add fresh availability and injected execution admission
without moving product authorization or transport authority into Longhorn.

## Result

`longhorn-command` now supplies bounded command, context, category, keyword,
route, capability, field, and enum-value types. Hosts register a single
finite context tree rooted at `global`, capabilities, and commands before
sealing one immutable generation and canonical digest.

V1 arguments admit only `null` or one declared bounded object. Boolean, finite
number, bounded integer, bounded string, and closed enum fields validate into
stable field-id order. Unknown fields, nested values, arrays, invalid
defaults, duplicate declarations, bad bounds, missing references, and
non-finite schema values fail closed.

Discovery projects the same semantic command ids for palette, menu, shortcut,
settings, and help surfaces. Hidden posture is explicit. Canonical search uses
label, keyword, category, command id, and description with stable score,
label, and id tie-breaking.

Loophole-shaped fixtures use the full global/project/surface/editor/region/
panel context tree. Jetstream-shaped fixtures use only global/editor. Neither
fixture introduces product types or changes the shared model.

## Validation

- `effigy test:command-core`
- `cargo clippy -p longhorn-command --all-targets -- -D warnings`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-command-core`
- `cargo doc -p longhorn-command --no-deps`
- 18 registry, argument, graph, digest, search, visibility, serialization, and
  donor contract tests
- dependency audit: `longhorn-core`, Serde, `serde_json`, and SHA-256 only; no
  config, bridge, Tauri, async runtime, renderer, Poodle, or donor dependency

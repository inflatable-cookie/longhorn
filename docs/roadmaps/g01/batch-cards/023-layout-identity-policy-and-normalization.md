# 023 Layout Identity, Policy, And Normalization

Status: complete
Owner: Tom
Roadmap: g01.005 batch 1
Governing refs: contracts 001, 002, 012, and 014; research memos 001 and 009
Auto-start next card: no

## Objective

Add the pure Rust layout identity, definition registry, bounded state model,
validation, normalization, and derived visibility foundation.

## Scope

- layout ids and monotonic revision primitives
- `longhorn-layout` workspace crate depending only on `longhorn-core`
- explicit document limits
- registered layout schemas, regions, families, sizing slots, and panel
  definitions
- explicit allowed placement, instance, move, and close policy
- ordered containers, regions, panel instances, active state, sizing, and
  collapse
- deterministic validation and normalization
- occupancy and transient-reveal visibility projection
- donor-shaped eight-region and five-region policy fixtures

## Public Behavior

Consumers register all policy. Missing policy never becomes permissive.
Durable state references registry ids and carries no product payload.

Current-schema invalid state fails typed. Normalization orders structural
records, preserves tab order, selects the first tab when valid input omits an
active selection, and canonicalizes supported collapse state. It does not
silently repair corrupt decoded documents. Mutation-specific active fallback
belongs to Card 024.

Sizing uses bounded integer millionths in named slots. The core does not
interpret a split tree or produce pixels.

## Out Of Scope

- mutation commands
- configuration persistence
- generated TypeScript
- window or Surface binding
- cross-window drag
- Svelte, Poodle, Tauri, or consumer writes

## Steps

1. Add bounded opaque layout ids and monotonic `LayoutRevision`.
2. Add the pure workspace crate and public module boundaries.
3. Define explicit document count limits.
4. Define layout-schema, region, family, and sizing-slot registrations.
5. Define panel definitions and explicit instance policy.
6. Reject duplicate, incomplete, unbounded, or contradictory registration.
7. Define durable document, container, region, sizing, and instance records.
8. Validate every reference, uniqueness rule, count, ratio, and active member.
9. Normalize structural order and active selection deterministically.
10. Project normal and transient-reveal region visibility without mutation.
11. Add Loophole eight-region and Nucleus five-region registry fixtures.

## Acceptance Criteria

- ids reject empty, oversized, uppercase, whitespace, and invalid characters
- revision increment is checked and cannot wrap
- zero or excessive document limits fail
- duplicate schema, region, slot, definition, or order fails
- missing instance or placement policy fails
- empty allowed placement rejects all regions
- ratio bounds and defaults are fixed-point and valid
- every panel instance exists exactly once
- active panel is absent or belongs to its region
- unsupported collapse state fails
- normalizing input permutations produces one snapshot
- transient reveal changes no durable byte or revision
- package graph contains only `longhorn-core`

## Evidence Required

- registry rejection matrix
- state invariant and permutation fixtures
- active-selection normalization fixtures
- fixed-point ratio boundary fixtures
- empty-region visibility and transient-reveal fixtures
- donor-shaped region definitions without donor types
- serde round trips
- Rust 1.85 and full Effigy QA

## Stop Conditions

- shared behavior requires a fixed Loophole or Nucleus region enum
- sizing requires a generic split-tree contract
- a Surface, window, product, Poodle, Svelte, Tauri, or config type enters the
  core
- invalid current-schema state must be silently repaired
- an unresolved policy choice changes durable document shape

## Outcome

`longhorn-core` now supplies seven bounded opaque layout ids and a checked
monotonic `LayoutRevision`. `longhorn-layout` adds:

- finite registry and document limits
- flat ordered schemas, semantic regions and families, and named sizing slots
- explicit panel placement, instance, move, and close policy
- Surface-independent containers, regions, panel instances, active state,
  collapse state, and integer-millionth sizing
- typed current-schema validation and deterministic normalization
- pure occupancy and transient-reveal visibility projection

Registration rejects duplicate ids and order, missing or contradictory
placement policy, invalid sizing bounds, and invalid bounded instance policy.
Document validation rejects missing or repeated structural state, unknown or
duplicate panel placement, invalid active selection, unsupported collapse,
out-of-range sizing, and exceeded instance policy.

Eighteen focused layout tests cover the registry matrix, state invariants,
normalization, serde, visibility, and Loophole eight-region and Nucleus
five-region shapes. Transient reveal preserves serialized state and revision.
The crate has no Surface, window, configuration, Tauri, TypeScript, Svelte,
Poodle, or donor dependency.

## Next Task

Cards 024-027 and `g01.005` are complete. Card 028 is ready under the compiled
`g01.006` runway.

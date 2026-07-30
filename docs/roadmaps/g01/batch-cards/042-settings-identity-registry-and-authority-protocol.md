# 042 Settings Identity, Registry, And Authority Protocol

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.008 batch 1
Governing refs: contracts 001, 005, 010, and 012; research memo 012
Depends on: Card 041
Auto-start next card: no

## Objective

Implement the pure Rust settings identity, sealed registry, projection, command,
and receipt foundation without configuration, Tauri, renderer, or product
dependencies.

## Scope

- `longhorn-settings` crate
- bounded module, section, page, renderer, anchor, scope, apply-unit,
  capability, and activation-target ids
- deterministic registration, admission, ordering, sealing, generation, and
  digest
- immediate/staged timing metadata
- configured/effective/policy/editability projection types
- authority tokens, scope revisions, load/apply/reset commands, outcomes, and
  receipts
- consumer-owned opaque value envelope with explicit limits and codec version
- Bovine, Soundcheck, and Loophole registry fixtures

## Public Behavior

Registration is mutable only before seal. Seal validates all references,
admits pages against composed capabilities, removes empty sections, orders by
explicit ordinal then stable id, and produces one immutable generation and
digest.

Authority types distinguish registry identity, scope revision, configured and
effective state, policy provenance, editability, recovery, mutation outcome,
durability evidence, and activation requirements. They do not interpret a
product value or mutate storage.

## Out Of Scope

- `longhorn-config` integration
- cross-domain transactions
- TypeScript generation
- Tauri commands or capabilities
- search UI, Svelte, or Poodle
- shared storage or recovery pages

## Steps

1. Add bounded settings ids and errors over `longhorn-core`.
2. Define module, section, page, anchor, scope, apply-unit, and capability
   declarations.
3. Validate ownership and cross-reference rules during registration.
4. Seal admitted pages into deterministic section and page order.
5. Derive registry generation and content digest from canonical declarations.
6. Define authority projection, opaque value, token, revision, and recovery
   types.
7. Define load, apply, reset, conflict, receipt, and activation protocol types.
8. Add minimal, product-rich, optional-module, duplicate, missing-reference,
   limit, and determinism fixtures.
9. Audit the crate graph and public API for product and host leakage.

## Acceptance Criteria

- every id is bounded and validated
- duplicate ids fail regardless of registration order
- missing owner, section, renderer, scope, or apply-unit references fail seal
- absent capabilities omit pages and empty sections deterministically
- equal order values use stable id as tie-break
- registry digest is stable across equivalent registration order
- sealed state cannot mutate
- immediate/staged timing does not encode restart behavior
- configured and effective values plus policy provenance remain distinct
- opaque values are bounded and versioned without becoming product schema
- commands carry registry generation, scope revision, and authority token
- pure crate imports no config, Tauri, Svelte, Poodle, or consumer package

## Evidence Required

- registry success and rejection matrix
- deterministic digest and ordering fixtures
- three donor-shaped declarations
- protocol round-trip and limit fixtures
- dependency and public-API audit
- focused Rust and Effigy QA

## Stop Conditions

- one registry shape cannot represent all three fixtures without product fields
- runtime capability admission requires Tauri security authority
- authority tokens cannot stay opaque to the pure protocol
- cross-domain atomicity must be claimed to complete the crate
- package boundaries require an unresolved product choice

## Next Task

Card 043 is ready but not started. Bind one apply unit to one registered
configuration domain without widening the atomicity claim.

## Result

`longhorn-settings` now supplies bounded settings identities, explicit limits,
mutable-before-seal registration, complete pre-admission validation,
capability-gated page admission, empty-section removal, canonical ordering,
host generations, and content-derived SHA-256 registry digests.

The pure authority protocol distinguishes configured, effective, default,
policy, editability, recovery, durability, and activation state. Load, apply,
and reset commands carry checked registry generation, scope revision, and
host-issued authority tokens. Consumer values remain versioned bounded JSON
envelopes without becoming shared schema.

Bovine, Soundcheck, and Loophole fixtures prove minimal, optional-recovery, and
consumer-owned specialist compositions. The crate graph contains only
`longhorn-core`, Serde/JSON, and SHA-256 support.

Evidence:
`../../../logs/2026-07/29-settings-identity-registry-and-authority-protocol.md`.
